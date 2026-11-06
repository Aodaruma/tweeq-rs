use std::collections::BTreeMap;

use tweeq_core::{
    EditEvent, EditOperation, EditSessionId, EditSessions, GestureModifiers, NumberConstraints,
    ParamId, ParamSnapshot, ParamValue, Selection, TweakGesture, hsva_to_rgba, quantize,
};

use crate::color::{
    ColorAdjustment, ColorChannel, ColorMode, ColorParameter, ColorView, format_hex, parse_hex,
};
use crate::rotary::Rotary;

/// Numeric constraints and interaction options kept on the Rust side.
#[derive(Debug, Clone, Copy, Default)]
pub struct NumberOptions {
    pub constraints: NumberConstraints,
    pub snap: Option<f64>,
    pub default_value: Option<f64>,
}

impl NumberOptions {
    fn range(self) -> Option<(f64, f64)> {
        let min = self.constraints.min?;
        let max = self.constraints.max?;
        let span = max - min;
        (span.is_finite() && span > 0.0).then_some((min, span))
    }

    fn constrain(self, value: f64, step: bool) -> f64 {
        let stepped = self.constraints.validate(value, step).value;
        self.constraints.validate(stepped, false).value
    }
}

/// Presentation data for one numeric control.
#[derive(Debug, Clone, PartialEq)]
pub struct NumberView {
    pub display: String,
    pub edit_text: String,
    pub fraction: f32,
    pub bounded: bool,
    pub selected: bool,
}

/// Coordinates of a numeric drag in logical pixels.
#[derive(Debug, Clone, Copy)]
pub struct ScrubStart {
    pub x: f32,
    pub width: f32,
    pub modifiers: GestureModifiers,
}

#[derive(Debug, Clone, Copy)]
struct NumberParameter {
    value: f64,
    options: NumberOptions,
}

#[derive(Debug)]
enum Scrub {
    Numeric {
        gesture: TweakGesture,
        base: f64,
        speed: f64,
    },
    Rotary(Rotary),
}

#[derive(Debug)]
struct Active {
    source: ParamId,
    session: EditSessionId,
    targets: Vec<ParamSnapshot>,
    scrub: Option<Scrub>,
    last: Option<EditOperation>,
    color_hue: Option<f32>,
    color_drag: Option<ColorDrag>,
}

#[derive(Debug)]
struct ColorDrag {
    captured: Vec<(ParamId, ColorParameter)>,
    motion: [f32; 2],
}

/// An edit event from the Slint adapter.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SlintEditEvent {
    Core(EditEvent),
    RelativeColor {
        session: EditSessionId,
        adjustment: ColorAdjustment,
    },
}

/// Editing state shared by the controls in one Slint window.
#[derive(Debug, Default)]
pub struct Editor {
    numbers: BTreeMap<ParamId, NumberParameter>,
    colors: BTreeMap<ParamId, ColorParameter>,
    order: Vec<ParamId>,
    selection: Selection,
    sessions: EditSessions,
    slint_events: Vec<SlintEditEvent>,
    active: Option<Active>,
}

impl Editor {
    /// Registers a numeric parameter in visual selection order.
    pub fn insert_number(&mut self, id: ParamId, value: f64, options: NumberOptions) {
        self.cancel();
        if !self.contains(id) {
            self.order.push(id);
        }
        self.colors.remove(&id);
        self.numbers.insert(id, NumberParameter { value, options });
    }

    /// Removes a parameter and cancels any edit that captured it.
    pub fn remove_number(&mut self, id: ParamId) {
        if !self.numbers.contains_key(&id) {
            return;
        }
        self.cancel_if_target(id);
        self.numbers.remove(&id);
        self.order.retain(|candidate| *candidate != id);
        if self.selection.contains(id) {
            self.selection.toggle(id);
        }
    }

    /// Applies a value from the host, such as an undo or external update.
    pub fn set_number(&mut self, id: ParamId, value: f64) {
        self.cancel_if_target(id);
        if let Some(parameter) = self.numbers.get_mut(&id) {
            parameter.value = value;
        }
    }

    /// Returns the current numeric value, including any active preview.
    #[must_use]
    pub fn number(&self, id: ParamId) -> Option<f64> {
        Some(self.numbers.get(&id)?.value)
    }

    /// Registers a normalized RGBA color in visual selection order.
    pub fn insert_color(&mut self, id: ParamId, rgba: [f32; 4]) {
        self.cancel();
        if !self.contains(id) {
            self.order.push(id);
        }
        self.numbers.remove(&id);
        self.colors.insert(id, ColorParameter::new(rgba));
    }

    /// Reads the current typed value, including any preview.
    #[must_use]
    pub fn value(&self, id: ParamId) -> Option<ParamValue> {
        self.number(id).map(ParamValue::Number).or_else(|| {
            self.colors
                .get(&id)
                .map(|color| ParamValue::Color(color.rgba))
        })
    }

    /// Restores a previously captured value, such as an undo.
    pub fn set_snapshot(&mut self, snapshot: &ParamSnapshot) {
        match snapshot.value {
            ParamValue::Number(value) => self.set_number(snapshot.id, value),
            ParamValue::Color(value) => {
                self.cancel_if_target(snapshot.id);
                if let Some(color) = self.colors.get_mut(&snapshot.id) {
                    color.set(value);
                }
            }
            _ => {}
        }
    }

    /// Returns the display data for a registered color.
    #[must_use]
    pub fn color_view(&self, id: ParamId) -> Option<ColorView> {
        let color = *self.colors.get(&id)?;
        Some(ColorView {
            rgba: color.rgba,
            hsva: color.hsva(),
            hex: format_hex(color.rgba),
            selected: self.selection.contains(id),
        })
    }

    /// Opens a color transaction for a picker gesture.
    pub fn begin_color(&mut self, id: ParamId) -> bool {
        if !self.colors.contains_key(&id) {
            return false;
        }
        self.cancel();
        let color = self.colors[&id];
        self.select(id, false, false);
        self.capture(
            id,
            vec![ParamSnapshot {
                id,
                value: ParamValue::Color(color.rgba),
            }],
            None,
            Some(color.hue),
        );
        true
    }

    /// Captures the selected colors for a relative edit.
    pub fn begin_relative_color(&mut self, id: ParamId) -> bool {
        if !self.colors.contains_key(&id) {
            return false;
        }
        self.cancel();
        self.select(id, false, false);
        let captured: Vec<_> = self
            .selection
            .iter()
            .filter_map(|key| self.colors.get(&key).map(|color| (key, *color)))
            .collect();
        let targets = captured
            .iter()
            .map(|(id, color)| ParamSnapshot {
                id: *id,
                value: ParamValue::Color(color.rgba),
            })
            .collect();
        self.capture(id, targets, None, None).color_drag = Some(ColorDrag {
            captured,
            motion: [0.0, 0.0],
        });
        true
    }

    /// Applies one relative movement sample to every captured color.
    pub fn scrub_color(&mut self, id: ParamId, motion: [f32; 2], mode: ColorMode) -> bool {
        let Some(active) = self.active.as_mut().filter(|edit| edit.source == id) else {
            return false;
        };
        let Some(drag) = &mut active.color_drag else {
            return false;
        };
        let total = [drag.motion[0] + motion[0], drag.motion[1] + motion[1]];
        if !total.into_iter().all(f32::is_finite) {
            return false;
        }
        drag.motion = total;
        let adjustment = ColorAdjustment {
            mode,
            motion: total,
        };
        for (key, initial) in &drag.captured {
            self.colors.insert(*key, adjustment.transform(*initial));
        }
        let session = active.session;
        self.flush_sessions();
        self.slint_events.push(SlintEditEvent::RelativeColor {
            session,
            adjustment,
        });
        true
    }

    /// Updates one normalized channel during a color transaction.
    pub fn preview_channel(&mut self, id: ParamId, channel: ColorChannel, value: f32) -> bool {
        if !self.active_absolute_color(id) || !value.is_finite() {
            return false;
        }
        let Some(color) = self.colors.get_mut(&id) else {
            return false;
        };
        let rgba = color.change(channel, value);
        self.apply(EditOperation::SetColor(rgba), false);
        true
    }

    /// Updates both coordinates of a saturation/value pad.
    pub fn preview_sv(&mut self, id: ParamId, saturation: f32, value: f32) -> bool {
        if !self.active_absolute_color(id) || !saturation.is_finite() || !value.is_finite() {
            return false;
        }
        let [hue, _, _, alpha] = self.colors[&id].hsva();
        self.apply(
            EditOperation::SetColor(hsva_to_rgba([hue, saturation, value, alpha])),
            false,
        );
        true
    }

    /// Commits a channel typed as a normalized decimal.
    pub fn set_color_channel(&mut self, id: ParamId, channel: ColorChannel, text: &str) -> bool {
        let Ok(value) = text.trim().parse::<f32>() else {
            return false;
        };
        if !value.is_finite() || !self.begin_color(id) {
            return false;
        }
        self.preview_channel(id, channel, value);
        self.commit_color(id)
    }

    /// Commits RGB or RGBA hex input as one color edit.
    pub fn set_color_hex(&mut self, id: ParamId, text: &str) -> bool {
        let Some(rgba) = parse_hex(text) else {
            return false;
        };
        if !self.begin_color(id) {
            return false;
        }
        self.apply(EditOperation::SetColor(rgba), false);
        self.commit_color(id)
    }

    /// Closes a color gesture as a single undoable transaction.
    pub fn commit_color(&mut self, id: ParamId) -> bool {
        if !self.active.as_ref().is_some_and(|active| {
            active.source == id && (active.color_hue.is_some() || active.color_drag.is_some())
        }) {
            return false;
        }
        self.finish(true);
        true
    }

    /// Returns the data needed for the Slint view.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn number_view(&self, id: ParamId) -> Option<NumberView> {
        let NumberParameter { value, options } = *self.numbers.get(&id)?;
        let precision = usize::from(options.constraints.precision.min(15));
        let display = if value != 0.0 && !(1e-6..1e12).contains(&value.abs()) {
            format!("{value:.precision$e}")
        } else {
            let formatted = format!("{value:.precision$}");
            if formatted.contains('.') {
                formatted
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_owned()
            } else {
                formatted
            }
        };
        let range = options.range();
        let fraction = range
            .filter(|_| value.is_finite())
            .map_or(0.0, |(min, span)| ((value - min) / span).clamp(0.0, 1.0));
        Some(NumberView {
            display,
            edit_text: value.to_string(),
            fraction: fraction as f32,
            bounded: range.is_some(),
            selected: self.selection.contains(id),
        })
    }

    /// Updates the selection without starting an edit.
    pub fn select(&mut self, id: ParamId, range: bool, toggle: bool) {
        if !self.contains(id) {
            return;
        }
        if range {
            self.selection.select_range(&self.order, id);
        } else if toggle {
            self.selection.toggle(id);
        } else if !self.selection.contains(id) {
            self.selection.select_only(id);
        }
    }

    /// Opens a text edit, capturing the original values of all targets.
    pub fn begin_text(&mut self, id: ParamId) -> bool {
        self.begin(id, None)
    }

    /// Starts either a bounded bar drag or a relative scrub.
    pub fn begin_scrub(&mut self, id: ParamId, start: ScrubStart) -> bool {
        self.cancel();
        let Some(parameter) = self.numbers.get(&id) else {
            return false;
        };
        let width = f64::from(start.width).max(1.0);
        let (base, speed) = parameter.options.range().map_or_else(
            || (parameter.value, 1.0),
            |(min, span)| {
                let fraction = (f64::from(start.x) / width).clamp(0.0, 1.0);
                (span.mul_add(fraction, min), span / width)
            },
        );
        let base = if start.modifiers.fine {
            parameter.value
        } else {
            base
        };
        self.begin(
            id,
            Some(Scrub::Numeric {
                gesture: TweakGesture::default(),
                base,
                speed,
            }),
        )
    }

    /// Starts an angular scrub around the center of a rotary control.
    pub fn begin_rotary(&mut self, id: ParamId, position: [f32; 2], size: [f32; 2]) -> bool {
        self.cancel();
        let Some(degrees) = self.number(id) else {
            return false;
        };
        self.begin(
            id,
            Some(Scrub::Rotary(Rotary::new(position, size, degrees))),
        )
    }

    /// Applies a logical-pixel motion sample to the active scrub.
    pub fn scrub(&mut self, id: ParamId, motion: [f32; 2], modifiers: GestureModifiers) -> bool {
        if !motion.into_iter().all(f32::is_finite) {
            return false;
        }
        let Some(active) = self.active.as_mut().filter(|edit| edit.source == id) else {
            return false;
        };
        let Some(scrub) = &mut active.scrub else {
            return false;
        };
        let parameter = self.numbers[&id];
        let snap = parameter
            .options
            .snap
            .filter(|value| value.is_finite() && *value > 0.0);
        let mut candidate = match scrub {
            Scrub::Numeric {
                gesture,
                base,
                speed,
            } => {
                let update = gesture.update(
                    motion,
                    *speed,
                    modifiers,
                    snap.unwrap_or(10.0),
                    0.01..=100.0,
                );
                *base + update.accumulated_delta
            }
            Scrub::Rotary(rotary) => rotary.update(motion, modifiers.fine),
        };
        if modifiers.snap
            && let Some(step) = snap
        {
            candidate = quantize(
                candidate,
                step,
                parameter.options.constraints.min.unwrap_or(0.0),
            );
        }
        let initial = active.targets.iter().find_map(|target| match target.value {
            ParamValue::Number(value) if target.id == id => Some(value),
            _ => None,
        });
        let Some(initial) = initial else { return false };
        let delta = candidate - initial;
        if !delta.is_finite() {
            return false;
        }
        self.apply(EditOperation::AddNumber(delta), false);
        true
    }

    /// Parses and commits a text edit. Invalid text leaves the edit open.
    pub fn commit_text(&mut self, id: ParamId, text: &str) -> bool {
        let Some(active) = self.active.as_ref().filter(|edit| {
            edit.source == id
                && edit.scrub.is_none()
                && edit.color_hue.is_none()
                && edit.color_drag.is_none()
        }) else {
            return false;
        };
        let Ok(value) = text.trim().parse::<f64>() else {
            return false;
        };
        if !value.is_finite() {
            return false;
        }
        if !active.targets.iter().any(|target| {
            target.id == id && matches!(target.value, ParamValue::Number(initial) if text == initial.to_string())
        }) {
            self.apply(EditOperation::SetNumber(value), true);
        }
        self.finish(true);
        true
    }

    /// Applies the configured default as one edit.
    pub fn reset_number(&mut self, id: ParamId) -> bool {
        let Some(value) = self
            .numbers
            .get(&id)
            .and_then(|parameter| parameter.options.default_value)
        else {
            return false;
        };
        if !value.is_finite() || !self.begin_text(id) {
            return false;
        }
        self.apply(EditOperation::SetNumber(value), true);
        self.finish(true);
        true
    }

    /// Finishes the active drag from this source and applies each target's step.
    pub fn commit_scrub(&mut self, id: ParamId) -> bool {
        let Some(active) = self
            .active
            .as_ref()
            .filter(|edit| edit.source == id && edit.scrub.is_some())
        else {
            return false;
        };
        if let Some(operation) = active.last.clone() {
            self.apply(operation, true);
        }
        self.finish(true);
        true
    }

    /// Cancels the active edit and restores its starting values.
    pub fn cancel(&mut self) {
        self.finish(false);
    }

    /// Cancels an edit only if it was started by this control.
    pub fn cancel_from(&mut self, id: ParamId) {
        if self.active.as_ref().is_some_and(|edit| edit.source == id) {
            self.cancel();
        }
    }

    /// Removes and returns shared events. Use `drain_slint_events` to observe relative color updates as well.
    pub fn drain_events(&mut self) -> impl Iterator<Item = EditEvent> + '_ {
        self.drain_slint_events().filter_map(|event| match event {
            SlintEditEvent::Core(event) => Some(event),
            SlintEditEvent::RelativeColor { .. } => None,
        })
    }

    /// Removes and returns both shared and Slint-specific events in order.
    pub fn drain_slint_events(&mut self) -> impl Iterator<Item = SlintEditEvent> + '_ {
        self.flush_sessions();
        self.slint_events.drain(..)
    }

    fn flush_sessions(&mut self) {
        self.slint_events
            .extend(self.sessions.drain().map(SlintEditEvent::Core));
    }

    fn cancel_if_target(&mut self, id: ParamId) {
        if self
            .active
            .as_ref()
            .is_some_and(|edit| edit.targets.iter().any(|target| target.id == id))
        {
            self.cancel();
        }
    }

    fn begin(&mut self, source: ParamId, scrub: Option<Scrub>) -> bool {
        self.cancel();
        if !self.numbers.contains_key(&source) {
            return false;
        }
        self.select(source, false, false);
        let targets: Vec<_> = self
            .selection
            .iter()
            .filter_map(|id| {
                self.number(id).map(|value| ParamSnapshot {
                    id,
                    value: ParamValue::Number(value),
                })
            })
            .collect();
        self.capture(source, targets, scrub, None);
        true
    }

    fn contains(&self, id: ParamId) -> bool {
        self.numbers.contains_key(&id) || self.colors.contains_key(&id)
    }

    fn active_absolute_color(&self, id: ParamId) -> bool {
        self.active
            .as_ref()
            .is_some_and(|active| active.source == id && active.color_hue.is_some())
    }

    fn capture(
        &mut self,
        source: ParamId,
        targets: Vec<ParamSnapshot>,
        scrub: Option<Scrub>,
        color_hue: Option<f32>,
    ) -> &mut Active {
        let session = self.sessions.begin(source, targets.clone());
        self.active.insert(Active {
            source,
            session,
            targets,
            scrub,
            last: None,
            color_hue,
            color_drag: None,
        })
    }

    fn apply(&mut self, operation: EditOperation, step: bool) {
        let Some(active) = &mut self.active else {
            return;
        };
        for target in &active.targets {
            if let EditOperation::SetColor(rgba) = operation {
                if let Some(color) = self.colors.get_mut(&target.id) {
                    color.set(rgba);
                }
                continue;
            }
            let ParamValue::Number(initial) = target.value else {
                continue;
            };
            let candidate = match operation {
                EditOperation::SetNumber(value) => value,
                EditOperation::AddNumber(delta) => initial + delta,
                _ => continue,
            };
            if candidate.is_finite()
                && let Some(parameter) = self.numbers.get_mut(&target.id)
            {
                parameter.value = parameter.options.constrain(candidate, step);
            }
        }
        active.last = Some(operation.clone());
        self.sessions.update(active.session, operation);
    }

    fn finish(&mut self, commit: bool) {
        let Some(active) = self.active.take() else {
            return;
        };
        if !commit {
            if let Some(drag) = active.color_drag {
                self.colors.extend(drag.captured);
            } else {
                for target in active.targets {
                    self.set_snapshot(&target);
                }
                if let Some(hue) = active.color_hue
                    && let Some(color) = self.colors.get_mut(&active.source)
                {
                    color.hue = hue;
                }
            }
        }
        self.sessions.finish(active.session, commit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_preview_cancels_to_the_captured_color_and_hex_keeps_alpha() {
        let id = ParamId::from_u64(1);
        let mut editor = Editor::default();
        editor.insert_color(id, [0.1, 0.2, 0.3, 0.4]);
        assert!(editor.begin_color(id));
        assert!(editor.preview_channel(id, ColorChannel::Hue, 0.25));
        assert!(editor.preview_sv(id, 0.8, 0.7));
        assert_ne!(
            editor.value(id),
            Some(ParamValue::Color([0.1, 0.2, 0.3, 0.4]))
        );
        editor.cancel();
        assert_eq!(
            editor.value(id),
            Some(ParamValue::Color([0.1, 0.2, 0.3, 0.4]))
        );
        assert!(!editor.set_color_hex(id, "#%#!invalid"));
        assert!(editor.set_color_hex(id, "#33669980"));
        assert_eq!(editor.color_view(id).unwrap().hex, "#33669980");
    }

    #[test]
    fn relative_color_uses_each_snapshot_and_preserves_event_order() {
        let a = ParamId::from_u64(1);
        let b = ParamId::from_u64(2);
        let initial_a = [0.25, 0.5, 0.0, 0.5];
        let initial_b = [0.875, 0.0, 0.5, 1.0];
        let mut editor = Editor::default();
        editor.insert_color(a, initial_a);
        editor.insert_color(b, initial_b);
        editor.select(a, false, false);
        editor.select(b, false, true);
        assert!(editor.begin_relative_color(a));
        assert!(editor.scrub_color(a, [62.5, 0.0], ColorMode::Channel(ColorChannel::Red)));
        assert_eq!(
            editor.value(a),
            Some(ParamValue::Color([0.5, 0.5, 0.0, 0.5]))
        );
        assert_eq!(
            editor.value(b),
            Some(ParamValue::Color([1.0, 0.0, 0.5, 1.0]))
        );
        editor.cancel();
        assert_eq!(editor.value(a), Some(ParamValue::Color(initial_a)));
        assert_eq!(editor.value(b), Some(ParamValue::Color(initial_b)));
        let events: Vec<_> = editor.drain_slint_events().collect();
        assert!(matches!(events.as_slice(), [
            SlintEditEvent::Core(EditEvent::Begin { session: begin, targets, .. }),
            SlintEditEvent::RelativeColor { session: update, .. },
            SlintEditEvent::Core(EditEvent::Cancel { session: cancel }),
        ] if begin == update && update == cancel && targets.len() == 2));
        let hue = ColorAdjustment {
            mode: ColorMode::Channel(ColorChannel::Hue),
            motion: [600.0, 0.0],
        };
        assert_eq!(format_hex(hue.apply([1.0, 0.0, 0.0, 1.0])), "#00FFFF");
    }

    #[test]
    fn unchanged_text_keeps_double_precision_and_cancel_restores_a_selection() {
        let a = ParamId::from_u64(1);
        let b = ParamId::from_u64(2);
        let mut editor = Editor::default();
        editor.insert_number(a, 16_777_217.0, NumberOptions::default());
        editor.insert_number(b, 30.0, NumberOptions::default());
        editor.select(a, false, false);
        editor.select(b, false, true);
        assert!(editor.begin_text(a));
        assert!(editor.commit_text(a, "16777217"));
        assert_eq!(editor.number(a), Some(16_777_217.0));
        assert_eq!(editor.number(b), Some(30.0));
        assert!(editor.begin_scrub(
            a,
            ScrubStart {
                x: 0.0,
                width: 100.0,
                modifiers: GestureModifiers::default()
            }
        ));
        assert!(editor.scrub(a, [3.0, 0.0], GestureModifiers::default()));
        assert_eq!(editor.number(a), Some(16_777_220.0));
        assert_eq!(editor.number(b), Some(33.0));
        editor.cancel();
        assert_eq!(editor.number(a), Some(16_777_217.0));
        assert_eq!(editor.number(b), Some(30.0));
        assert!(matches!(
            editor.drain_events().last(),
            Some(EditEvent::Cancel { .. })
        ));
    }

    #[test]
    fn relative_commit_quantizes_each_target_against_its_own_constraints() {
        let a = ParamId::from_u64(1);
        let b = ParamId::from_u64(2);
        let mut editor = Editor::default();
        editor.insert_number(a, 0.0, NumberOptions::default());
        editor.insert_number(
            b,
            10.0,
            NumberOptions {
                constraints: NumberConstraints {
                    max: Some(12.5),
                    step: Some(2.0),
                    ..NumberConstraints::default()
                },
                ..NumberOptions::default()
            },
        );
        editor.select(a, false, false);
        editor.select(b, false, true);
        assert!(editor.begin_scrub(
            a,
            ScrubStart {
                x: 0.0,
                width: 100.0,
                modifiers: GestureModifiers::default()
            }
        ));
        assert!(editor.scrub(a, [3.0, 0.0], GestureModifiers::default()));
        assert!(editor.commit_scrub(a));
        assert_eq!(editor.number(a), Some(3.0));
        assert_eq!(editor.number(b), Some(12.0));
    }
}
