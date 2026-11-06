#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

use egui::{
    Align2, Color32, CornerRadius, CursorGrab, FontId, Key, LayerId, Order, Pos2, Rect, Response,
    Sense, Stroke, StrokeKind, Ui, Vec2, ViewportCommand,
};
use tweeq_core::{EditOperation, ParamId, ParamKind};

use crate::{Number, TweeqContext};

const INVALID: Color32 = Color32::from_rgb(239, 92, 92);

/// Compact position input composed from a translation scrubber and X/Y fields.
pub struct Position<'a> {
    id: ParamId,
    value: &'a mut [f64; 2],
    scale: f64,
    minimum: Option<[f64; 2]>,
    maximum: Option<[f64; 2]>,
    step: f64,
    enabled: bool,
    invalid: bool,
}

impl<'a> Position<'a> {
    pub fn new(id: ParamId, value: &'a mut [f64; 2]) -> Self {
        Self {
            id,
            value,
            scale: 1.0,
            minimum: None,
            maximum: None,
            step: 1.0,
            enabled: true,
            invalid: false,
        }
    }

    #[must_use]
    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    #[must_use]
    pub fn min(mut self, minimum: [f64; 2]) -> Self {
        self.minimum = Some(minimum);
        self
    }

    #[must_use]
    pub fn max(mut self, maximum: [f64; 2]) -> Self {
        self.maximum = Some(maximum);
        self
    }

    #[must_use]
    pub fn step(mut self, step: f64) -> Self {
        if step.is_finite() && step > 0.0 {
            self.step = step;
        }
        self
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    #[allow(clippy::too_many_lines)]
    pub fn show(self, ui: &mut Ui, context: &mut TweeqContext) -> Response {
        let Self {
            id,
            value,
            scale,
            minimum,
            maximum,
            step,
            enabled,
            invalid,
        } = self;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            let mut translate = Translate::new(id, value)
                .scale(scale)
                .step(step)
                .enabled(enabled)
                .invalid(invalid);
            if let Some(minimum) = minimum {
                translate = translate.min(minimum);
            }
            if let Some(maximum) = maximum {
                translate = translate.max(maximum);
            }
            let drag = translate.show(ui, context);

            let mut vector = Vector::new(id.child(10), value)
                .step(step)
                .width(64.0)
                .enabled(enabled)
                .invalid(invalid);
            if let Some(minimum) = minimum {
                vector = vector.min(minimum);
            }
            if let Some(maximum) = maximum {
                vector = vector.max(maximum);
            }
            drag.union(vector.show(ui, context))
        })
        .inner
    }
}

/// Two-dimensional translation scrubber with a Tweeq-style drag overlay.
pub struct Translate<'a> {
    id: ParamId,
    value: &'a mut [f64; 2],
    scale: f64,
    minimum: Option<[f64; 2]>,
    maximum: Option<[f64; 2]>,
    step: f64,
    enabled: bool,
    invalid: bool,
    show_overlay_label: bool,
}

impl<'a> Translate<'a> {
    pub fn new(id: ParamId, value: &'a mut [f64; 2]) -> Self {
        Self {
            id,
            value,
            scale: 1.0,
            minimum: None,
            maximum: None,
            step: 1.0,
            enabled: true,
            invalid: false,
            show_overlay_label: true,
        }
    }

    #[must_use]
    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    #[must_use]
    pub fn min(mut self, minimum: [f64; 2]) -> Self {
        self.minimum = Some(minimum);
        self
    }

    #[must_use]
    pub fn max(mut self, maximum: [f64; 2]) -> Self {
        self.maximum = Some(maximum);
        self
    }

    #[must_use]
    pub fn step(mut self, step: f64) -> Self {
        if step.is_finite() && step > 0.0 {
            self.step = step;
        }
        self
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    #[must_use]
    pub fn show_overlay_label(mut self, show: bool) -> Self {
        self.show_overlay_label = show;
        self
    }

    #[allow(clippy::too_many_lines)]
    pub fn show(self, ui: &mut Ui, context: &mut TweeqContext) -> Response {
        context.register_vector(self.id, *self.value);
        let theme = context.theme().clone();
        let mut state = context.take_vector_drag_state(self.id);
        let sense = if self.enabled {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(theme.input_height), sense);
        let response = if self.enabled {
            response.on_hover_and_drag_cursor(egui::CursorIcon::Crosshair)
        } else {
            response
        };

        if self.enabled && response.drag_started() {
            response.request_focus();
            context.activate_selection(self.id, ParamKind::Vector, false, false);
            state.captured = *self.value;
            state.last_total = Vec2::ZERO;
            state.session = Some(context.start_edit(self.id, ParamKind::Vector));
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::CursorGrab(CursorGrab::Locked));
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::CursorVisible(false));
        }

        let modifiers = ui.input(|input| input.modifiers);
        let only_x = ui.input(|input| input.key_down(Key::X) || input.key_down(Key::Num0));
        let only_y = ui.input(|input| input.key_down(Key::Y) || input.key_down(Key::Num1));
        let speed = if modifiers.shift {
            5.0
        } else if modifiers.alt {
            0.1
        } else {
            1.0
        };

        if self.enabled
            && response.dragged()
            && let Some(session) = state.session
        {
            state.last_total += response.drag_motion();
            let mut delta = [
                f64::from(state.last_total.x) * self.scale * speed,
                f64::from(state.last_total.y) * self.scale * speed,
            ];
            if only_x {
                delta[1] = 0.0;
            }
            if only_y {
                delta[0] = 0.0;
            }
            let mut next = [state.captured[0] + delta[0], state.captured[1] + delta[1]];
            if ui.input(|input| input.key_down(Key::Q)) {
                next[0] = quantize(next[0], self.step);
                next[1] = quantize(next[1], self.step);
            }
            clamp_vector(&mut next, self.minimum, self.maximum);
            *self.value = next;
            context.update_edit(
                session,
                EditOperation::AddVector {
                    delta: [
                        next[0] - state.captured[0],
                        next[1] - state.captured[1],
                        0.0,
                        0.0,
                    ],
                    dimensions: 2,
                },
            );
        }

        if self.enabled && response.has_focus() && !response.dragged() {
            let direction = ui.input(|input| {
                let x = f64::from(input.key_pressed(Key::ArrowRight))
                    - f64::from(input.key_pressed(Key::ArrowLeft));
                let y = f64::from(input.key_pressed(Key::ArrowDown))
                    - f64::from(input.key_pressed(Key::ArrowUp));
                [x, y]
            });
            if direction != [0.0, 0.0] {
                let before = *self.value;
                self.value[0] += direction[0] * self.step * speed;
                self.value[1] += direction[1] * self.step * speed;
                clamp_vector(self.value, self.minimum, self.maximum);
                if self
                    .value
                    .iter()
                    .zip(before)
                    .any(|(current, previous)| (current - previous).abs() > f64::EPSILON)
                {
                    context.immediate_edit(
                        self.id,
                        ParamKind::Vector,
                        EditOperation::AddVector {
                            delta: [
                                self.value[0] - before[0],
                                self.value[1] - before[1],
                                0.0,
                                0.0,
                            ],
                            dimensions: 2,
                        },
                    );
                }
            }
        }

        if response.drag_stopped() {
            if let Some(session) = state.session.take() {
                context.finish_edit(session, true);
            }
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::CursorGrab(CursorGrab::None));
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::CursorVisible(true));
        }

        let fill = if !self.enabled {
            theme.input
        } else if response.hovered() {
            theme.accent_hover
        } else {
            theme.accent
        };
        ui.painter().rect(
            rect,
            CornerRadius::same(theme.input_radius),
            fill,
            Stroke::new(1.0, if self.invalid { INVALID } else { fill }),
            StrokeKind::Inside,
        );
        paint_grid_icon(
            ui,
            rect.center(),
            if self.enabled {
                Color32::WHITE
            } else {
                theme.text_muted
            },
        );

        if response.dragged() {
            paint_translate_overlay(
                ui,
                self.id,
                rect.center(),
                *self.value,
                self.minimum,
                self.maximum,
                speed,
                only_x,
                only_y,
                self.show_overlay_label,
                &theme,
            );
        }

        context.put_vector_drag_state(self.id, state);
        response
    }
}

/// Compact numeric vector editor.
pub struct Vector<'a, const N: usize> {
    id: ParamId,
    value: &'a mut [f64; N],
    minimum: Option<[f64; N]>,
    maximum: Option<[f64; N]>,
    step: f64,
    width: f32,
    enabled: bool,
    invalid: bool,
}

impl<'a, const N: usize> Vector<'a, N> {
    pub fn new(id: ParamId, value: &'a mut [f64; N]) -> Self {
        Self {
            id,
            value,
            minimum: None,
            maximum: None,
            step: 0.1,
            width: 72.0,
            enabled: true,
            invalid: false,
        }
    }

    #[must_use]
    pub fn min(mut self, minimum: [f64; N]) -> Self {
        self.minimum = Some(minimum);
        self
    }

    #[must_use]
    pub fn max(mut self, maximum: [f64; N]) -> Self {
        self.maximum = Some(maximum);
        self
    }

    #[must_use]
    pub fn step(mut self, step: f64) -> Self {
        if step.is_finite() && step > 0.0 {
            self.step = step;
        }
        self
    }

    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.max(48.0);
        self
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    pub fn show(self, ui: &mut Ui, context: &mut TweeqContext) -> Response {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            let mut combined = None;
            for (index, component) in self.value.iter_mut().enumerate() {
                let mut number = Number::new(self.id.child(index as u64 + 1), component)
                    .step(self.step)
                    .precision(2)
                    .prefix(axis_label(index))
                    .width(self.width)
                    .enabled(self.enabled)
                    .invalid(self.invalid);
                if let Some(minimum) = self.minimum {
                    number = number.min(minimum[index]);
                }
                if let Some(maximum) = self.maximum {
                    number = number.max(maximum[index]);
                }
                let response = number.show(ui, context).response;
                combined = Some(combined.map_or(response.clone(), |previous: Response| {
                    previous.union(response)
                }));
            }
            combined.unwrap_or_else(|| ui.allocate_response(Vec2::ZERO, Sense::hover()))
        })
        .inner
    }
}

/// Two-component size editor with an overlaid aspect-ratio link.
pub struct Size<'a> {
    id: ParamId,
    value: &'a mut [f64; 2],
    locked: &'a mut bool,
    enabled: bool,
    invalid: bool,
}

impl<'a> Size<'a> {
    pub fn new(id: ParamId, value: &'a mut [f64; 2], locked: &'a mut bool) -> Self {
        Self {
            id,
            value,
            locked,
            enabled: true,
            invalid: false,
        }
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    pub fn show(self, ui: &mut Ui, context: &mut TweeqContext) -> Response {
        let before = *self.value;
        let ratio = if before[1].abs() > f64::EPSILON {
            before[0] / before[1]
        } else {
            1.0
        };
        let theme = context.theme().clone();
        let inner = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            let width = Number::new(self.id.child(1), &mut self.value[0])
                .step(1.0)
                .precision(1)
                .prefix("W ")
                .bar(false)
                .width(78.0)
                .enabled(self.enabled)
                .invalid(self.invalid)
                .show(ui, context);
            let height = Number::new(self.id.child(2), &mut self.value[1])
                .step(1.0)
                .precision(1)
                .prefix("H ")
                .bar(false)
                .width(78.0)
                .enabled(self.enabled)
                .invalid(self.invalid)
                .show(ui, context);
            (width, height)
        });
        let (width, height) = inner.inner;

        let chain_rect = Rect::from_center_size(
            Pos2::new(
                f32::midpoint(width.response.rect.right(), height.response.rect.left()),
                inner.response.rect.center().y,
            ),
            Vec2::splat(theme.input_height),
        );
        let chain = ui.interact(
            chain_rect,
            ui.make_persistent_id(("tweeq-size-link", self.id.as_u64())),
            if self.enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        if chain.clicked() {
            *self.locked = !*self.locked;
        }

        if *self.locked && width.changed {
            self.value[1] = if ratio.abs() > f64::EPSILON {
                self.value[0] / ratio
            } else {
                self.value[1]
            };
        }
        if *self.locked && height.changed {
            self.value[0] = self.value[1] * ratio;
        }
        paint_link_icon(
            ui,
            chain_rect,
            if !self.enabled {
                theme.text_muted
            } else if *self.locked {
                theme.accent
            } else {
                theme.text_muted
            },
            *self.locked,
        );

        width.response.union(height.response).union(chain)
    }
}

fn axis_label(index: usize) -> &'static str {
    const LABELS: [&str; 4] = ["X ", "Y ", "Z ", "W "];
    LABELS.get(index).copied().unwrap_or("")
}

fn quantize(value: f64, step: f64) -> f64 {
    if step.is_finite() && step > 0.0 {
        (value / step).round() * step
    } else {
        value
    }
}

fn clamp_vector(value: &mut [f64; 2], minimum: Option<[f64; 2]>, maximum: Option<[f64; 2]>) {
    for index in 0..2 {
        if let Some(minimum) = minimum {
            value[index] = value[index].max(minimum[index]);
        }
        if let Some(maximum) = maximum {
            value[index] = value[index].min(maximum[index]);
        }
    }
}

fn paint_grid_icon(ui: &Ui, center: Pos2, color: Color32) {
    for y in -1..=1 {
        for x in -1..=1 {
            ui.painter().circle_filled(
                center + Vec2::new(x as f32 * 3.5, y as f32 * 3.5),
                1.0,
                color,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_translate_overlay(
    ui: &Ui,
    id: ParamId,
    center: Pos2,
    value: [f64; 2],
    minimum: Option<[f64; 2]>,
    maximum: Option<[f64; 2]>,
    speed: f64,
    only_x: bool,
    only_y: bool,
    show_label: bool,
    theme: &crate::TweeqTheme,
) {
    let painter = ui.ctx().layer_painter(LayerId::new(
        Order::Foreground,
        egui::Id::new(("tweeq-translate-overlay", id.as_u64())),
    ));
    let radius = 150.0;
    let visual_scale = if speed >= 5.0 {
        0.5
    } else if speed < 1.0 {
        4.0
    } else {
        2.0
    };
    let spacing = 10.0 * visual_scale;
    let offset_x = (-value[0] as f32 * visual_scale).rem_euclid(spacing);
    let offset_y = (-value[1] as f32 * visual_scale).rem_euclid(spacing);
    let bounds = Rect::from_center_size(center, Vec2::splat(radius * 2.0));

    let mut y = bounds.top() + offset_y;
    while y <= bounds.bottom() {
        let mut x = bounds.left() + offset_x;
        while x <= bounds.right() {
            let point = Pos2::new(x, y);
            let distance = point.distance(center);
            if distance <= radius {
                let alpha = ((1.0 - distance / radius) * 0.7).clamp(0.0, 0.7);
                painter.circle_filled(point, 1.0, theme.text_muted.gamma_multiply(alpha));
            }
            x += spacing;
        }
        y += spacing;
    }

    if only_x {
        painter.line_segment(
            [
                Pos2::new(bounds.left(), center.y),
                Pos2::new(bounds.right(), center.y),
            ],
            Stroke::new(2.0, theme.accent),
        );
    }
    if only_y {
        painter.line_segment(
            [
                Pos2::new(center.x, bounds.top()),
                Pos2::new(center.x, bounds.bottom()),
            ],
            Stroke::new(2.0, theme.accent),
        );
    }

    if let (Some(minimum), Some(maximum)) = (minimum, maximum) {
        let range_rect = Rect::from_min_max(
            Pos2::new(
                center.x + (minimum[0] - value[0]) as f32 * visual_scale,
                center.y + (minimum[1] - value[1]) as f32 * visual_scale,
            ),
            Pos2::new(
                center.x + (maximum[0] - value[0]) as f32 * visual_scale,
                center.y + (maximum[1] - value[1]) as f32 * visual_scale,
            ),
        );
        painter.rect_stroke(
            range_rect,
            0.0,
            Stroke::new(1.0, theme.accent),
            StrokeKind::Inside,
        );
    }

    if show_label {
        let precision = usize::from(speed < 1.0);
        let label = format!(
            "X  {:.*}    Y  {:.*}",
            precision, value[0], precision, value[1]
        );
        let label_center = Pos2::new(center.x, center.y - 28.0);
        let label_rect = Rect::from_center_size(
            label_center,
            Vec2::new((label.chars().count() as f32 * 7.0 + 16.0).max(92.0), 24.0),
        );
        painter.rect_filled(label_rect, 4.0, theme.surface);
        painter.rect_stroke(
            label_rect,
            4.0,
            Stroke::new(1.0, theme.border),
            StrokeKind::Inside,
        );
        painter.text(
            label_center,
            Align2::CENTER_CENTER,
            label,
            FontId::monospace(11.0),
            theme.text,
        );
    }
}

fn paint_link_icon(ui: &Ui, rect: Rect, color: Color32, linked: bool) {
    let painter = ui.painter();
    let center = rect.center();
    let loop_stroke = Stroke::new(1.25, color);
    let loop_size = Vec2::new(7.0, 5.5);
    if linked {
        painter.line_segment(
            [center + Vec2::new(-2.6, 0.0), center + Vec2::new(2.6, 0.0)],
            Stroke::new(1.0, color),
        );
    }
    for offset in [-6.0, 6.0] {
        painter.rect_stroke(
            Rect::from_center_size(center + Vec2::new(offset, 0.0), loop_size),
            CornerRadius::same(3),
            loop_stroke,
            StrokeKind::Middle,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{clamp_vector, quantize};

    #[test]
    fn translate_quantization_uses_requested_step() {
        assert!((quantize(1.24, 0.5) - 1.0).abs() < f64::EPSILON);
        assert!((quantize(1.26, 0.5) - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn translate_range_is_applied_per_axis() {
        let mut value = [-4.0, 12.0];
        clamp_vector(&mut value, Some([-2.0, -1.0]), Some([8.0, 9.0]));
        assert!((value[0] + 2.0).abs() < f64::EPSILON);
        assert!((value[1] - 9.0).abs() < f64::EPSILON);
    }
}
