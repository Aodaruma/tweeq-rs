#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use std::ops::RangeInclusive;

use egui::{
    Color32, CornerRadius, FontId, LayerId, Order, Pos2, Rect, Response, Sense, Stroke, StrokeKind,
    Ui, Vec2,
};
use tweeq_core::{EditOperation, ParamId, ParamKind};

use crate::{TweeqContext, TweeqTheme};

const INVALID: Color32 = Color32::from_rgb(232, 78, 88);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TimeDisplay {
    #[default]
    Smpte,
    Frames,
}

#[derive(Clone)]
struct TimecodeState {
    editing: bool,
    buffer: String,
    captured: i64,
    drag_value: f64,
    scale: usize,
    display: TimeDisplay,
    request_focus: bool,
}

impl TimecodeState {
    fn new(frames: i64, frame_rate: u32, display: TimeDisplay) -> Self {
        Self {
            editing: false,
            buffer: format_time(frames, frame_rate, display),
            captured: frames,
            drag_value: frames as f64,
            scale: 0,
            display,
            request_focus: false,
        }
    }
}

/// Frame-backed Tweeq time input with SMPTE display and unit-aware scrubbing.
pub struct Timecode<'a> {
    id: ParamId,
    frames: &'a mut i64,
    frame_rate: u32,
    min: i64,
    max: i64,
    width: f32,
    enabled: bool,
    invalid: bool,
    display: Option<&'a mut TimeDisplay>,
}

impl<'a> Timecode<'a> {
    pub fn new(id: ParamId, frames: &'a mut i64) -> Self {
        Self {
            id,
            frames,
            frame_rate: 24,
            min: i64::MIN,
            max: i64::MAX,
            width: 240.0,
            enabled: true,
            invalid: false,
            display: None,
        }
    }

    #[must_use]
    pub fn frame_rate(mut self, frame_rate: u32) -> Self {
        self.frame_rate = frame_rate.max(1);
        self
    }

    #[must_use]
    pub fn range(mut self, range: RangeInclusive<i64>) -> Self {
        self.min = *range.start();
        self.max = *range.end();
        self
    }

    #[must_use]
    pub const fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub const fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    #[must_use]
    pub fn display(mut self, display: &'a mut TimeDisplay) -> Self {
        self.display = Some(display);
        self
    }

    #[allow(clippy::too_many_lines)]
    pub fn show(self, ui: &mut Ui, context: &mut TweeqContext) -> Response {
        context.register_number(self.id, *self.frames as f64);
        let theme = context.theme().clone();
        let state_id = ui.make_persistent_id(("tweeq-timecode", self.id.as_u64()));
        let requested_display = self
            .display
            .as_ref()
            .map_or(TimeDisplay::Smpte, |display| **display);
        let mut state = ui
            .data(|data| data.get_temp::<TimecodeState>(state_id))
            .unwrap_or_else(|| {
                TimecodeState::new(*self.frames, self.frame_rate, requested_display)
            });
        if self.display.is_some() {
            state.display = requested_display;
        }
        let before = *self.frames;

        let mut response = if state.editing && self.enabled {
            let mut response = ui
                .scope(|ui| {
                    ui.visuals_mut().override_text_color = self.invalid.then_some(INVALID);
                    ui.add_sized(
                        [self.width, theme.input_height],
                        egui::TextEdit::singleline(&mut state.buffer)
                            .id_source(state_id)
                            .font(egui::TextStyle::Monospace)
                            .horizontal_align(egui::Align::Center),
                    )
                })
                .inner;
            if state.request_focus {
                response.request_focus();
                state.request_focus = false;
            }

            if response.changed()
                && let Some(value) = parse_timecode(&state.buffer, self.frame_rate)
            {
                *self.frames = value.clamp(self.min, self.max);
            }
            let modifiers = ui.input(|input| input.modifiers);
            let up = ui.input(|input| input.key_pressed(egui::Key::ArrowUp));
            let down = ui.input(|input| input.key_pressed(egui::Key::ArrowDown));
            if up || down {
                let amount = if modifiers.alt {
                    1
                } else if modifiers.shift {
                    i64::from(self.frame_rate) * 60
                } else {
                    i64::from(self.frame_rate)
                };
                let direction = if up { 1 } else { -1 };
                *self.frames = self
                    .frames
                    .saturating_add(amount * direction)
                    .clamp(self.min, self.max);
                state.buffer = format_time(*self.frames, self.frame_rate, state.display);
                response.mark_changed();
            }
            if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                *self.frames = state.captured;
                state.editing = false;
                response.surrender_focus();
            } else if ui.input(|input| input.key_pressed(egui::Key::Enter)) || response.lost_focus()
            {
                state.editing = false;
                state.buffer = format_time(*self.frames, self.frame_rate, state.display);
                response.surrender_focus();
            }
            paint_clock_icon(ui, response.rect, theme.text_muted);
            response
        } else {
            let (rect, mut response) = ui.allocate_exact_size(
                Vec2::new(self.width, theme.input_height),
                if self.enabled {
                    Sense::click_and_drag()
                } else {
                    Sense::hover()
                },
            );
            response = response.on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
            let segments = time_segments(ui, rect, *self.frames, self.frame_rate, state.display);

            if self.enabled {
                if response.drag_started() {
                    response.request_focus();
                    state.captured = *self.frames;
                    state.drag_value = *self.frames as f64;
                    state.scale =
                        time_scale_at(&segments, ui.input(|input| input.pointer.interact_pos()))
                            .unwrap_or(0);
                }
                if response.dragged() {
                    state.scale = forced_time_scale(ui).unwrap_or_else(|| {
                        adjusted_time_scale(state.scale, ui.input(|input| input.modifiers))
                    });
                    let speed = tweak_speed(state.scale, self.frame_rate);
                    state.drag_value += f64::from(response.drag_motion().x) * speed;
                    let mut next = state.drag_value.round() as i64;
                    if ui.input(|input| input.key_down(egui::Key::Q)) {
                        next = snap_time(next, state.captured, state.scale, self.frame_rate);
                    }
                    *self.frames = next.clamp(self.min, self.max);
                    response.mark_changed();
                    ui.ctx().request_repaint();
                } else if response.clicked() {
                    state.editing = true;
                    state.request_focus = true;
                    state.captured = *self.frames;
                    state.buffer = format_time(*self.frames, self.frame_rate, state.display);
                    ui.ctx().request_repaint();
                }
            }

            let highlighted_scale = if response.dragged() {
                Some(state.scale)
            } else if response.hovered() {
                time_scale_at(&segments, ui.input(|input| input.pointer.hover_pos()))
            } else {
                None
            };

            paint_timecode(
                ui,
                rect,
                &response,
                &segments,
                highlighted_scale,
                self.enabled,
                self.invalid,
                &theme,
            );
            if response.dragged() {
                paint_time_overlay(
                    ui,
                    rect.center(),
                    *self.frames,
                    self.frame_rate,
                    state.scale,
                    &theme,
                );
            }
            response
        };

        if before != *self.frames {
            response.mark_changed();
            context.immediate_edit(
                self.id,
                ParamKind::Number,
                EditOperation::SetNumber(*self.frames as f64),
            );
        }
        if !state.editing {
            state.buffer = format_time(*self.frames, self.frame_rate, state.display);
        }
        let mut display_changed = false;
        response.context_menu(|ui| {
            ui.set_min_width(150.0);
            ui.weak("Display format");
            for (display, label) in [
                (TimeDisplay::Smpte, "SMPTE Timecode"),
                (TimeDisplay::Frames, "Frames"),
            ] {
                if ui
                    .selectable_label(state.display == display, label)
                    .clicked()
                {
                    state.display = display;
                    display_changed = true;
                    ui.close();
                }
            }
        });
        if display_changed {
            state.editing = false;
            state.buffer = format_time(*self.frames, self.frame_rate, state.display);
            response.surrender_focus();
            ui.ctx().request_repaint();
        }
        if let Some(display) = self.display {
            *display = state.display;
        }
        ui.data_mut(|data| data.insert_temp(state_id, state));
        response
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_timecode(
    ui: &Ui,
    rect: egui::Rect,
    response: &Response,
    segments: &[TimeSegment],
    highlighted_scale: Option<usize>,
    enabled: bool,
    invalid: bool,
    theme: &TweeqTheme,
) {
    let fill = if !enabled {
        Color32::TRANSPARENT
    } else if response.hovered() {
        theme.input_hover
    } else {
        theme.input
    };
    let stroke = if invalid {
        Stroke::new(1.0, INVALID)
    } else if !enabled {
        Stroke::new(1.0, theme.border)
    } else if response.has_focus() {
        Stroke::new(1.0, theme.accent)
    } else {
        Stroke::NONE
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(theme.input_radius),
        fill,
        stroke,
        StrokeKind::Inside,
    );
    paint_clock_icon(ui, rect, theme.text_muted);

    if let Some(scale) = highlighted_scale
        && let Some(segment) = segments.iter().find(|segment| segment.scale == scale)
    {
        ui.painter().rect_filled(
            segment.rect,
            CornerRadius::same(theme.input_radius),
            theme.text_muted.gamma_multiply(0.18),
        );
    }

    let color = if invalid {
        INVALID
    } else if enabled {
        theme.text
    } else {
        theme.text_muted
    };
    for (index, segment) in segments.iter().enumerate() {
        ui.painter().text(
            segment.rect.center(),
            egui::Align2::CENTER_CENTER,
            &segment.text,
            egui::TextStyle::Monospace.resolve(ui.style()),
            color,
        );
        if index + 1 < segments.len() {
            let next = &segments[index + 1];
            ui.painter().text(
                egui::pos2(
                    f32::midpoint(segment.rect.right(), next.rect.left()),
                    rect.center().y,
                ),
                egui::Align2::CENTER_CENTER,
                ":",
                egui::TextStyle::Monospace.resolve(ui.style()),
                theme.text_muted,
            );
        }
    }

    if (response.hovered() || response.dragged())
        && let Some(scale) = highlighted_scale
        && let Some(segment) = segments.iter().find(|segment| segment.scale == scale)
    {
        paint_time_unit_label(ui, response, segment, scale, theme);
    }
}

fn paint_time_unit_label(
    ui: &Ui,
    response: &Response,
    segment: &TimeSegment,
    scale: usize,
    theme: &TweeqTheme,
) {
    let painter = ui.ctx().layer_painter(LayerId::new(
        Order::Tooltip,
        response.id.with("time-unit-label"),
    ));
    let font = FontId::proportional(9.0);
    let galley = painter.layout_no_wrap(
        ["Frames", "Secs", "Mins", "Hrs"][scale].to_owned(),
        font,
        theme.text,
    );
    let size = galley.size() + Vec2::new(10.0, 6.0);
    let screen = ui.ctx().content_rect();
    let mut center = Pos2::new(
        segment.rect.center().x,
        segment.rect.top() - 5.0 - size.y * 0.5,
    );
    center.x = center.x.clamp(
        screen.left() + size.x * 0.5 + 3.0,
        screen.right() - size.x * 0.5 - 3.0,
    );
    if center.y - size.y * 0.5 < screen.top() + 3.0 {
        center.y = segment.rect.bottom() + 5.0 + size.y * 0.5;
    }
    let label_rect = Rect::from_center_size(center, size);
    painter.rect(
        label_rect,
        CornerRadius::same(theme.input_radius),
        theme.surface,
        Stroke::new(1.0, theme.border),
        StrokeKind::Inside,
    );
    painter.galley(
        label_rect.center() - galley.size() * 0.5,
        galley,
        theme.text,
    );
}

#[derive(Debug, Clone)]
struct TimeSegment {
    scale: usize,
    rect: egui::Rect,
    text: String,
}

fn time_segments(
    ui: &Ui,
    rect: egui::Rect,
    frames: i64,
    frame_rate: u32,
    display: TimeDisplay,
) -> Vec<TimeSegment> {
    let parts = time_parts(frames, frame_rate, display);
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let separator_width = ui
        .painter()
        .layout_no_wrap(":".to_owned(), font.clone(), Color32::WHITE)
        .size()
        .x;
    let widths: Vec<_> = parts
        .iter()
        .map(|(_, text)| {
            ui.painter()
                .layout_no_wrap(text.clone(), font.clone(), Color32::WHITE)
                .size()
                .x
                + 6.0
        })
        .collect();
    let total_width =
        widths.iter().sum::<f32>() + separator_width * parts.len().saturating_sub(1) as f32;
    let mut x = rect.center().x - total_width * 0.5;
    parts
        .into_iter()
        .zip(widths)
        .map(|((scale, text), width)| {
            let segment = TimeSegment {
                scale,
                rect: egui::Rect::from_min_size(
                    egui::pos2(x, rect.top() + 2.0),
                    egui::vec2(width, rect.height() - 4.0),
                ),
                text,
            };
            x += width + separator_width;
            segment
        })
        .collect()
}

fn time_scale_at(segments: &[TimeSegment], pointer: Option<egui::Pos2>) -> Option<usize> {
    let pointer = pointer?;
    segments
        .iter()
        .find(|segment| segment.rect.contains(pointer))
        .map(|segment| segment.scale)
}

fn paint_clock_icon(ui: &Ui, rect: egui::Rect, color: Color32) {
    let center = egui::pos2(rect.left() + 13.0, rect.center().y);
    let stroke = Stroke::new(1.25, color);
    ui.painter().circle_stroke(center, 5.2, stroke);
    ui.painter()
        .line_segment([center, center + Vec2::new(0.0, -3.1)], stroke);
    ui.painter()
        .line_segment([center, center + Vec2::new(2.7, 1.8)], stroke);
}

fn paint_time_overlay(
    ui: &Ui,
    center: egui::Pos2,
    frames: i64,
    frame_rate: u32,
    scale: usize,
    theme: &TweeqTheme,
) {
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("tweeq-time-overlay"),
    ));
    let radius = 82.0;
    for index in 0..12 {
        let angle = std::f32::consts::TAU * index as f32 / 12.0 - std::f32::consts::FRAC_PI_2;
        let direction = Vec2::angled(angle);
        painter.line_segment(
            [
                center + direction * (radius - 5.0),
                center + direction * radius,
            ],
            Stroke::new(1.0, theme.text_muted),
        );
    }
    let fps = i64::from(frame_rate.max(1));
    let values = [
        (frames.rem_euclid(fps) as f32 / fps as f32, 5.0),
        ((frames / fps).rem_euclid(60) as f32 / 60.0, 1.0),
        ((frames / (fps * 60)).rem_euclid(60) as f32 / 60.0, 3.0),
        ((frames / (fps * 3600)).rem_euclid(12) as f32 / 12.0, 5.0),
    ];
    for (index, (turn, width)) in values.into_iter().enumerate() {
        let direction = Vec2::angled(turn * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2);
        let length = [radius - 7.0, radius - 13.0, radius - 23.0, radius - 42.0][index];
        painter.line_segment(
            [
                center + direction * (radius * 0.5),
                center + direction * length,
            ],
            Stroke::new(
                width,
                if index == scale {
                    theme.accent
                } else {
                    theme.text_muted
                },
            ),
        );
    }
}

fn forced_time_scale(ui: &Ui) -> Option<usize> {
    if ui.input(|input| input.key_down(egui::Key::T) || input.key_down(egui::Key::F)) {
        Some(0)
    } else if ui.input(|input| input.key_down(egui::Key::S)) {
        Some(1)
    } else if ui.input(|input| input.key_down(egui::Key::M)) {
        Some(2)
    } else if ui.input(|input| input.key_down(egui::Key::H)) {
        Some(3)
    } else {
        None
    }
}

fn adjusted_time_scale(scale: usize, modifiers: egui::Modifiers) -> usize {
    if modifiers.shift {
        (scale + 1).min(3)
    } else if modifiers.alt {
        scale.saturating_sub(1)
    } else {
        scale
    }
}

fn tweak_speed(scale: usize, frame_rate: u32) -> f64 {
    let fps = f64::from(frame_rate.max(1));
    match scale {
        0 => 0.25,
        1 => fps / 10.0,
        2 => fps * 6.0,
        _ => fps * 36.0,
    }
}

fn snap_time(value: i64, captured: i64, scale: usize, frame_rate: u32) -> i64 {
    let fps = i64::from(frame_rate.max(1));
    let step = match scale {
        0 => 1,
        1 => fps,
        2 => fps * 60,
        _ => fps * 3600,
    };
    if step <= 1 {
        return value;
    }
    let offset = captured.rem_euclid(step);
    ((value - offset) as f64 / step as f64).round() as i64 * step + offset
}

fn format_timecode(frames: i64, frame_rate: u32) -> String {
    let fps = i64::from(frame_rate.max(1));
    let sign = if frames < 0 { "-" } else { "" };
    let absolute = frames.saturating_abs();
    let frame = absolute % fps;
    let seconds_total = absolute / fps;
    let second = seconds_total % 60;
    let minutes_total = seconds_total / 60;
    let minute = minutes_total % 60;
    let hour = minutes_total / 60;
    if hour == 0 {
        format!("{sign}{minute:02}:{second:02}:{frame:02}")
    } else {
        format!("{sign}{hour:02}:{minute:02}:{second:02}:{frame:02}")
    }
}

fn format_time(frames: i64, frame_rate: u32, display: TimeDisplay) -> String {
    match display {
        TimeDisplay::Smpte => format_timecode(frames, frame_rate),
        TimeDisplay::Frames => format!("{frames}F"),
    }
}

fn time_parts(frames: i64, frame_rate: u32, display: TimeDisplay) -> Vec<(usize, String)> {
    if display == TimeDisplay::Frames {
        return vec![(0, format!("{frames}F"))];
    }
    let text = format_timecode(frames, frame_rate);
    let chunks: Vec<_> = text.split(':').map(str::to_owned).collect();
    let length = chunks.len();
    chunks
        .into_iter()
        .enumerate()
        .map(|(index, text)| (length - index - 1, text))
        .collect()
}

fn parse_timecode(input: &str, frame_rate: u32) -> Option<i64> {
    let input = input.trim().to_lowercase();
    let fps = f64::from(frame_rate.max(1));
    if input.contains(':') {
        let negative = input.starts_with('-');
        let digits: Vec<f64> = input
            .trim_start_matches('-')
            .split(':')
            .rev()
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()?;
        let mut frames = 0.0;
        for (index, digit) in digits.into_iter().enumerate() {
            let multiplier = match index {
                0 => 1.0,
                1 => fps,
                _ => fps * 60_f64.powi(index as i32 - 1),
            };
            frames += digit * multiplier;
        }
        return Some((if negative { -frames } else { frames }).round() as i64);
    }
    let (number, multiplier) = if let Some(number) = input.strip_suffix("frames") {
        (number, 1.0)
    } else if let Some(number) = input.strip_suffix('f') {
        (number, 1.0)
    } else if let Some(number) = input.strip_suffix("seconds") {
        (number, fps)
    } else if let Some(number) = input.strip_suffix("secs") {
        (number, fps)
    } else if let Some(number) = input.strip_suffix('s') {
        (number, fps)
    } else if let Some(number) = input.strip_suffix("minutes") {
        (number, fps * 60.0)
    } else if let Some(number) = input.strip_suffix("mins") {
        (number, fps * 60.0)
    } else if let Some(number) = input.strip_suffix('m') {
        (number, fps * 60.0)
    } else if let Some(number) = input.strip_suffix("hours") {
        (number, fps * 3600.0)
    } else if let Some(number) = input.strip_suffix("hrs") {
        (number, fps * 3600.0)
    } else if let Some(number) = input.strip_suffix('h') {
        (number, fps * 3600.0)
    } else {
        (input.as_str(), 1.0)
    };
    Some((number.trim().parse::<f64>().ok()? * multiplier).round() as i64)
}

#[cfg(test)]
mod tests {
    use super::{TimeDisplay, format_time, format_timecode, parse_timecode, snap_time, time_parts};

    #[test]
    fn formats_frame_timecode_like_vue() {
        assert_eq!(format_timecode(12, 24), "00:00:12");
        assert_eq!(format_timecode(24 * 3661 + 12, 24), "01:01:01:12");
        assert_eq!(format_timecode(-24, 24), "-00:01:00");
    }

    #[test]
    fn parses_timecode_and_units() {
        assert_eq!(parse_timecode("01:02:03:12", 24), Some(89_364));
        assert_eq!(parse_timecode("1.5s", 24), Some(36));
        assert_eq!(parse_timecode("2m", 24), Some(2_880));
        assert_eq!(parse_timecode("12F", 24), Some(12));
    }

    #[test]
    fn snapping_preserves_captured_subunit_offset() {
        assert_eq!(snap_time(53, 5, 1, 24), 53);
        assert_eq!(snap_time(49, 5, 1, 24), 53);
        assert_eq!(snap_time(31, 5, 1, 24), 29);
    }

    #[test]
    fn display_modes_and_segment_scales_match_the_visible_text() {
        assert_eq!(format_time(48, 24, TimeDisplay::Frames), "48F");
        assert_eq!(
            time_parts(24 * 61 + 12, 24, TimeDisplay::Smpte),
            vec![
                (2, "01".to_owned()),
                (1, "01".to_owned()),
                (0, "12".to_owned())
            ]
        );
    }
}
