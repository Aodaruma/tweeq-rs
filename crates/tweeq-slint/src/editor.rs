use std::collections::BTreeMap;

use tweeq_core::{
    EditEvent, EditOperation, EditSessionId, EditSessions, GestureModifiers, NumberConstraints,
    ParamId, ParamSnapshot, ParamValue, Selection, TweakGesture, quantize,
};

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
struct Scrub {
    gesture: TweakGesture,
    base: f64,
    speed: f64,
}

#[derive(Debug)]
struct Active {
    source: ParamId,
    session: EditSessionId,
    targets: Vec<ParamSnapshot>,
    scrub: Option<Scrub>,
    last: Option<EditOperation>,
}

/// Numeric editing state shared by the controls in one Slint window.
#[derive(Debug, Default)]
pub struct Editor {
    numbers: BTreeMap<ParamId, NumberParameter>,
    order: Vec<ParamId>,
    selection: Selection,
    sessions: EditSessions,
    active: Option<Active>,
}

impl Editor {
    /// Registers a numeric parameter in visual selection order.
    pub fn insert_number(&mut self, id: ParamId, value: f64, options: NumberOptions) {
        self.cancel();
        if !self.numbers.contains_key(&id) {
            self.order.push(id);
        }
        self.numbers.insert(id, NumberParameter { value, options });
    }

    /// Removes a parameter and cancels any edit that captured it.
    pub fn remove_number(&mut self, id: ParamId) {
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
        if !self.numbers.contains_key(&id) {
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
            Some(Scrub {
                gesture: TweakGesture::default(),
                base,
                speed,
            }),
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
        let update = scrub.gesture.update(
            motion,
            scrub.speed,
            modifiers,
            snap.unwrap_or(10.0),
            0.01..=100.0,
        );
        let mut candidate = scrub.base + update.accumulated_delta;
        if update.snap
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
        let Some(active) = self
            .active
            .as_ref()
            .filter(|edit| edit.source == id && edit.scrub.is_none())
        else {
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

    /// Removes and returns pending events in order.
    pub fn drain_events(&mut self) -> impl Iterator<Item = EditEvent> + '_ {
        self.sessions.drain()
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
        let session = self.sessions.begin(source, targets.clone());
        self.active = Some(Active {
            source,
            session,
            targets,
            scrub,
            last: None,
        });
        true
    }

    fn apply(&mut self, operation: EditOperation, step: bool) {
        let Some(active) = &mut self.active else {
            return;
        };
        for target in &active.targets {
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
            for target in active.targets {
                if let ParamValue::Number(value) = target.value
                    && let Some(parameter) = self.numbers.get_mut(&target.id)
                {
                    parameter.value = value;
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
