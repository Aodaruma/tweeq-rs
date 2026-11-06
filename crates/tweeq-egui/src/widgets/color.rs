#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use egui::{
    Color32, CornerRadius, FontId, Id, Key, LayerId, Mesh, Order, PopupCloseBehavior, Pos2, Rect,
    Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};
use tweeq_core::{EditOperation, EditSessionId, ParamId, ParamKind, hsva_to_rgba, rgba_to_hsva};

use crate::{TweeqContext, TweeqTheme};

const PICKER_WIDTH: f32 = 224.0;
const PAD_HEIGHT: f32 = 150.0;
const SLIDER_HEIGHT: f32 = 14.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum ColorSpace {
    #[default]
    Hsv,
    Rgb,
    Hex,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum ColorDragMode {
    #[default]
    SaturationValue,
    Hue,
    Saturation,
    Value,
    Alpha,
    Red,
    Green,
    Blue,
}

#[derive(Clone)]
struct ColorState {
    captured: [f32; 4],
    session: Option<EditSessionId>,
    drag_mode: ColorDragMode,
    drag_origin: Pos2,
    color_space: ColorSpace,
    hex_buffer: String,
}

impl ColorState {
    fn new(value: [f32; 4]) -> Self {
        Self {
            captured: value,
            session: None,
            drag_mode: ColorDragMode::SaturationValue,
            drag_origin: Pos2::ZERO,
            color_space: ColorSpace::Hsv,
            hex_buffer: rgba_to_opaque_hex(value),
        }
    }
}

/// Tweeq color parameter with relative channel scrubbing and a dedicated picker.
///
/// Drag the compact field to edit saturation/value. Hold Shift (or H), S, V,
/// Alt (or A), R, G, or B to edit one channel. Click to open the picker.
pub struct ColorInput<'a> {
    id: ParamId,
    value: &'a mut [f32; 4],
    width: f32,
    alpha: bool,
}

impl<'a> ColorInput<'a> {
    pub fn new(id: ParamId, value: &'a mut [f32; 4]) -> Self {
        Self {
            id,
            value,
            width: 240.0,
            alpha: true,
        }
    }

    /// Sets the width of the compact color field.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.max(72.0);
        self
    }

    /// Shows or hides alpha editing. Hidden alpha is forced to one.
    #[must_use]
    pub fn alpha(mut self, alpha: bool) -> Self {
        self.alpha = alpha;
        self
    }

    #[allow(clippy::too_many_lines)]
    pub fn show(self, ui: &mut Ui, context: &mut TweeqContext) -> Response {
        if !self.alpha {
            self.value[3] = 1.0;
        }
        context.register_color(self.id, *self.value);
        let theme = context.theme().clone();
        let state_id = ui.make_persistent_id(("tweeq-color", self.id.as_u64()));
        let mut state = ui
            .data(|data| data.get_temp::<ColorState>(state_id))
            .unwrap_or_else(|| ColorState::new(*self.value));
        let popup_id = ui.make_persistent_id(("tweeq-color-popup", self.id.as_u64()));
        let before_inline = *self.value;
        let inline = ui
            .scope(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.horizontal(|ui| {
                    let (pad_rect, mut pad_response) = ui.allocate_exact_size(
                        Vec2::splat(theme.input_height),
                        Sense::click_and_drag(),
                    );
                    pad_response = pad_response.on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                    paint_color_pad(ui, pad_rect, &pad_response, *self.value, &theme);

                    let alpha_width = if self.alpha { 50.0 } else { 0.0 };
                    let hex_width = (self.width - theme.input_height - alpha_width).max(48.0);
                    let hex_response = ui.add_sized(
                        [hex_width, theme.input_height],
                        egui::TextEdit::singleline(&mut state.hex_buffer)
                            .id_source(state_id.with("inline-hex"))
                            .font(egui::TextStyle::Monospace)
                            .margin(egui::Margin::symmetric(7, 2)),
                    );
                    if hex_response.changed()
                        && let Some(mut parsed) = parse_hex(&state.hex_buffer)
                    {
                        if state.hex_buffer.trim().trim_start_matches('#').len() == 6 {
                            parsed[3] = self.value[3];
                        }
                        *self.value = parsed;
                    }
                    if !hex_response.has_focus() && !pad_response.dragged() {
                        state.hex_buffer = rgba_to_opaque_hex(*self.value);
                    }

                    let alpha_response = if self.alpha {
                        let mut alpha_percent = self.value[3] * 100.0;
                        let response = ui.add_sized(
                            [alpha_width, theme.input_height],
                            egui::DragValue::new(&mut alpha_percent)
                                .range(0.0..=100.0)
                                .speed(1.0)
                                .suffix("%")
                                .max_decimals(0),
                        );
                        if response.changed() {
                            self.value[3] = (alpha_percent / 100.0).clamp(0.0, 1.0);
                        }
                        Some(response)
                    } else {
                        None
                    };
                    (pad_response, hex_response, alpha_response)
                })
                .inner
            })
            .inner;
        let (mut pad_response, hex_response, alpha_response) = inline;
        let inline_changed = colors_differ(before_inline, *self.value);

        if pad_response.drag_started() {
            egui::Popup::close_id(ui.ctx(), popup_id);
            let modifiers = ui.input(|input| input.modifiers);
            context.activate_selection(
                self.id,
                ParamKind::Color,
                modifiers.shift,
                modifiers.command,
            );
            state.captured = *self.value;
            state.drag_origin = pad_response
                .interact_pointer_pos()
                .unwrap_or(pad_response.rect.center());
            state.session = Some(context.start_edit(self.id, ParamKind::Color));
        }

        if pad_response.dragged() {
            state.drag_mode = color_drag_mode(ui, self.alpha);
            let total = pad_response.total_drag_delta().unwrap_or_default();
            *self.value = state.captured;
            apply_drag(self.value, state.drag_mode, total, self.alpha);
            state.hex_buffer = rgba_to_opaque_hex(*self.value);
            if let Some(session) = state.session {
                context.update_edit(session, EditOperation::SetColor(*self.value));
            }
            pad_response.mark_changed();
            paint_drag_overlay(
                ui,
                state_id,
                state.drag_origin,
                *self.value,
                state.drag_mode,
                &theme,
            );
            ui.ctx().request_repaint();
        }

        if pad_response.drag_stopped()
            && let Some(session) = state.session.take()
        {
            context.finish_edit(session, true);
        }

        if inline_changed {
            context.immediate_edit(
                self.id,
                ParamKind::Color,
                EditOperation::SetColor(*self.value),
            );
        }

        let popup_before = *self.value;
        let popup = egui::Popup::from_toggle_button_response(&pad_response)
            .id(popup_id)
            .gap(4.0)
            .width(PICKER_WIDTH + 20.0)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_min_width(PICKER_WIDTH);
                show_picker(ui, self.value, &mut state, self.alpha, &theme)
            });
        if popup.is_some() && colors_differ(*self.value, popup_before) {
            pad_response.mark_changed();
            state.hex_buffer = rgba_to_opaque_hex(*self.value);
            context.immediate_edit(
                self.id,
                ParamKind::Color,
                EditOperation::SetColor(*self.value),
            );
        }

        let mut response = pad_response.union(hex_response);
        if let Some(alpha_response) = alpha_response {
            response = response.union(alpha_response);
        }
        if inline_changed || colors_differ(popup_before, *self.value) {
            response.mark_changed();
        }
        ui.data_mut(|data| data.insert_temp(state_id, state));
        response
    }
}

fn paint_color_pad(ui: &Ui, rect: Rect, response: &Response, value: [f32; 4], theme: &TweeqTheme) {
    paint_checkerboard(ui, rect.shrink(1.0), 5.0);
    ui.painter().rect_filled(
        rect.shrink(1.0),
        CornerRadius::same(theme.input_radius.saturating_sub(1)),
        color32(value),
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(theme.input_radius),
        Stroke::new(
            1.0,
            if response.hovered() || response.has_focus() || response.dragged() {
                theme.accent
            } else {
                theme.border
            },
        ),
        StrokeKind::Inside,
    );
}

fn show_picker(
    ui: &mut Ui,
    value: &mut [f32; 4],
    state: &mut ColorState,
    alpha: bool,
    theme: &TweeqTheme,
) -> bool {
    let before = *value;
    let mut hsva = rgba_to_hsva(*value);
    show_sv_pad(ui, &mut hsva, theme);
    ui.add_space(4.0);
    show_hue_slider(ui, &mut hsva, theme);
    if alpha {
        ui.add_space(3.0);
        show_alpha_slider(ui, &mut hsva, theme);
    } else {
        hsva[3] = 1.0;
    }
    *value = hsva_to_rgba(hsva);

    ui.add_space(5.0);
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.color_space, ColorSpace::Hsv, "HSV");
        ui.selectable_value(&mut state.color_space, ColorSpace::Rgb, "RGB");
        ui.selectable_value(&mut state.color_space, ColorSpace::Hex, "HEX");
    });

    match state.color_space {
        ColorSpace::Hsv => show_hsv_channels(ui, value, alpha),
        ColorSpace::Rgb => show_rgb_channels(ui, value, alpha),
        ColorSpace::Hex => show_hex_channel(ui, value, state),
    }

    ui.add_space(4.0);
    ui.weak("Swatches");
    let swatches = [
        [0.95, 0.22, 0.25, 1.0],
        [1.0, 0.52, 0.12, 1.0],
        [1.0, 0.82, 0.12, 1.0],
        [0.24, 0.78, 0.38, 1.0],
        [0.12, 0.78, 0.82, 1.0],
        [0.18, 0.42, 1.0, 1.0],
        [0.58, 0.32, 0.92, 1.0],
        [1.0, 1.0, 1.0, 1.0],
        [0.48, 0.48, 0.5, 1.0],
        [0.05, 0.05, 0.06, 1.0],
    ];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        for swatch in swatches {
            let (rect, response) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
            ui.painter().rect(
                rect,
                CornerRadius::same(3),
                color32(swatch),
                Stroke::new(1.0, theme.border),
                StrokeKind::Inside,
            );
            if response.clicked() {
                let retained_alpha = if alpha { value[3] } else { 1.0 };
                *value = [swatch[0], swatch[1], swatch[2], retained_alpha];
            }
        }
    });
    colors_differ(before, *value)
}

fn show_sv_pad(ui: &mut Ui, hsva: &mut [f32; 4], theme: &TweeqTheme) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(PICKER_WIDTH, PAD_HEIGHT), Sense::click_and_drag());
    if let Some(pointer) = response.interact_pointer_pos() {
        hsva[1] = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        hsva[2] = (1.0 - (pointer.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
    }
    paint_sv_mesh(ui, rect, hsva[0]);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(theme.input_radius),
        Stroke::new(1.0, theme.border),
        StrokeKind::Inside,
    );
    let marker = Pos2::new(
        egui::lerp(rect.left()..=rect.right(), hsva[1]),
        egui::lerp(rect.bottom()..=rect.top(), hsva[2]),
    );
    let picked = color32(hsva_to_rgba(*hsva));
    ui.painter()
        .circle_stroke(marker, 5.5, Stroke::new(2.0, contrast_color(picked)));
    response
}

fn show_hue_slider(ui: &mut Ui, hsva: &mut [f32; 4], theme: &TweeqTheme) -> Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(PICKER_WIDTH, SLIDER_HEIGHT),
        Sense::click_and_drag(),
    );
    if let Some(pointer) = response.interact_pointer_pos() {
        hsva[0] = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
    }
    paint_horizontal_gradient(ui, rect, |t| color32(hsva_to_rgba([t, 1.0, 1.0, 1.0])));
    paint_slider_marker(ui, rect, hsva[0], theme);
    response
}

fn show_alpha_slider(ui: &mut Ui, hsva: &mut [f32; 4], theme: &TweeqTheme) -> Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(PICKER_WIDTH, SLIDER_HEIGHT),
        Sense::click_and_drag(),
    );
    if let Some(pointer) = response.interact_pointer_pos() {
        hsva[3] = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
    }
    paint_checkerboard(ui, rect, 4.0);
    paint_horizontal_gradient(ui, rect, |t| {
        color32(hsva_to_rgba([hsva[0], hsva[1], hsva[2], t]))
    });
    paint_slider_marker(ui, rect, hsva[3], theme);
    response
}

fn paint_slider_marker(ui: &Ui, rect: Rect, value: f32, theme: &TweeqTheme) {
    let x = egui::lerp(rect.left()..=rect.right(), value);
    ui.painter().line_segment(
        [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
        Stroke::new(2.0, theme.text),
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(2),
        Stroke::new(1.0, theme.border),
        StrokeKind::Inside,
    );
}

fn show_hsv_channels(ui: &mut Ui, value: &mut [f32; 4], alpha: bool) {
    let mut hsva = rgba_to_hsva(*value);
    ui.horizontal(|ui| {
        channel(ui, "H", &mut hsva[0], 360.0);
        channel(ui, "S", &mut hsva[1], 100.0);
        channel(ui, "V", &mut hsva[2], 100.0);
        if alpha {
            channel(ui, "A", &mut hsva[3], 100.0);
        }
    });
    *value = hsva_to_rgba(hsva);
}

fn show_rgb_channels(ui: &mut Ui, value: &mut [f32; 4], alpha: bool) {
    ui.horizontal(|ui| {
        channel(ui, "R", &mut value[0], 255.0);
        channel(ui, "G", &mut value[1], 255.0);
        channel(ui, "B", &mut value[2], 255.0);
        if alpha {
            channel(ui, "A", &mut value[3], 100.0);
        }
    });
    for channel in value.iter_mut() {
        *channel = channel.clamp(0.0, 1.0);
    }
}

fn channel(ui: &mut Ui, label: &str, value: &mut f32, scale: f32) {
    ui.vertical(|ui| {
        ui.weak(label);
        let mut displayed = *value * scale;
        if ui
            .add_sized(
                [45.0, 20.0],
                egui::DragValue::new(&mut displayed)
                    .speed(1.0)
                    .range(0.0..=scale),
            )
            .changed()
        {
            *value = (displayed / scale).clamp(0.0, 1.0);
        }
    });
}

fn show_hex_channel(ui: &mut Ui, value: &mut [f32; 4], state: &mut ColorState) {
    let response = ui.add_sized(
        [PICKER_WIDTH, 22.0],
        egui::TextEdit::singleline(&mut state.hex_buffer)
            .font(egui::TextStyle::Monospace)
            .hint_text("#RRGGBBAA"),
    );
    if response.changed()
        && let Some(parsed) = parse_hex(&state.hex_buffer)
    {
        *value = parsed;
    }
    if !response.has_focus() {
        state.hex_buffer = rgba_to_hex(*value);
    }
}

fn color_drag_mode(ui: &Ui, alpha: bool) -> ColorDragMode {
    ui.input(|input| {
        if input.modifiers.alt || input.key_down(Key::A) {
            if alpha {
                ColorDragMode::Alpha
            } else {
                ColorDragMode::SaturationValue
            }
        } else if input.modifiers.shift || input.key_down(Key::H) || input.key_down(Key::F) {
            ColorDragMode::Hue
        } else if input.key_down(Key::S) {
            ColorDragMode::Saturation
        } else if input.key_down(Key::V) {
            ColorDragMode::Value
        } else if input.key_down(Key::R) {
            ColorDragMode::Red
        } else if input.key_down(Key::G) {
            ColorDragMode::Green
        } else if input.key_down(Key::B) {
            ColorDragMode::Blue
        } else {
            ColorDragMode::SaturationValue
        }
    })
}

fn apply_drag(value: &mut [f32; 4], mode: ColorDragMode, motion: Vec2, alpha: bool) {
    let mut hsva = rgba_to_hsva(*value);
    let fine = 0.004;
    match mode {
        ColorDragMode::SaturationValue => {
            hsva[1] = (hsva[1] + motion.x * fine).clamp(0.0, 1.0);
            hsva[2] = (hsva[2] - motion.y * fine).clamp(0.0, 1.0);
            *value = hsva_to_rgba(hsva);
        }
        ColorDragMode::Hue => {
            hsva[0] = (hsva[0] + motion.x * 0.0025).rem_euclid(1.0);
            *value = hsva_to_rgba(hsva);
        }
        ColorDragMode::Saturation => {
            hsva[1] = (hsva[1] + motion.x * fine).clamp(0.0, 1.0);
            *value = hsva_to_rgba(hsva);
        }
        ColorDragMode::Value => {
            hsva[2] = (hsva[2] + motion.x * fine).clamp(0.0, 1.0);
            *value = hsva_to_rgba(hsva);
        }
        ColorDragMode::Alpha => value[3] = (value[3] + motion.x * fine).clamp(0.0, 1.0),
        ColorDragMode::Red => value[0] = (value[0] + motion.x * fine).clamp(0.0, 1.0),
        ColorDragMode::Green => value[1] = (value[1] + motion.x * fine).clamp(0.0, 1.0),
        ColorDragMode::Blue => value[2] = (value[2] + motion.x * fine).clamp(0.0, 1.0),
    }
    if !alpha {
        value[3] = 1.0;
    }
}

fn paint_drag_overlay(
    ui: &Ui,
    id: Id,
    origin: Pos2,
    value: [f32; 4],
    mode: ColorDragMode,
    theme: &TweeqTheme,
) {
    let painter = ui
        .ctx()
        .layer_painter(LayerId::new(Order::Foreground, id.with("drag-overlay")));
    let hsva = rgba_to_hsva(value);
    let tweak_width = PICKER_WIDTH;

    if matches!(
        mode,
        ColorDragMode::SaturationValue
            | ColorDragMode::Hue
            | ColorDragMode::Saturation
            | ColorDragMode::Value
    ) {
        let pad = Rect::from_min_size(
            Pos2::new(
                origin.x - hsva[1] * tweak_width,
                origin.y - (1.0 - hsva[2]) * tweak_width,
            ),
            Vec2::splat(tweak_width),
        );
        let pad_opacity = if mode == ColorDragMode::SaturationValue {
            1.0
        } else {
            0.1
        };
        paint_sv_mesh_with_opacity(&painter, pad, hsva[0], pad_opacity);
        painter.rect_stroke(
            pad,
            CornerRadius::same(theme.input_radius),
            Stroke::new(1.0, theme.border.gamma_multiply(pad_opacity)),
            StrokeKind::Inside,
        );
        paint_hue_wheel(
            &painter,
            origin,
            tweak_width * 0.5 - 4.0,
            hsva[0],
            if mode == ColorDragMode::Hue { 1.0 } else { 0.1 },
        );
    }

    if matches!(
        mode,
        ColorDragMode::Saturation
            | ColorDragMode::Value
            | ColorDragMode::Alpha
            | ColorDragMode::Red
            | ColorDragMode::Green
            | ColorDragMode::Blue
    ) {
        paint_drag_slider(&painter, origin, tweak_width, value, hsva, mode, theme);
    }

    paint_overlay_label(
        &painter,
        ui.ctx().content_rect(),
        origin,
        value,
        hsva,
        mode,
        theme,
    );

    let preview_radius = theme.input_height * 0.9;
    painter.circle_filled(origin, preview_radius, theme.input);
    painter.circle_filled(
        origin,
        preview_radius,
        color32(if mode == ColorDragMode::Alpha {
            value
        } else {
            [value[0], value[1], value[2], 1.0]
        }),
    );
    painter.circle_stroke(origin, preview_radius, Stroke::new(1.0, theme.border));
}

fn paint_hue_wheel(painter: &egui::Painter, center: Pos2, radius: f32, hue: f32, opacity: f32) {
    let segments = 72_u32;
    for index in 0..segments {
        let t0 = index as f32 / segments as f32;
        let t1 = (index + 1) as f32 / segments as f32;
        let angle0 = (t0 - hue) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let angle1 = (t1 - hue) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let start = center + Vec2::angled(angle0) * radius;
        let end = center + Vec2::angled(angle1) * radius;
        painter.line_segment(
            [start, end],
            Stroke::new(
                4.0,
                color32(hsva_to_rgba([t0, 1.0, 1.0, 1.0])).gamma_multiply(opacity),
            ),
        );
    }
    for index in 0..6 {
        let angle = index as f32 / 6.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        painter.circle_filled(
            center + Vec2::angled(angle) * (radius - 1.0),
            1.8,
            Color32::BLACK.gamma_multiply(opacity),
        );
    }
}

fn paint_drag_slider(
    painter: &egui::Painter,
    origin: Pos2,
    width: f32,
    value: [f32; 4],
    hsva: [f32; 4],
    mode: ColorDragMode,
    theme: &TweeqTheme,
) {
    let channel = match mode {
        ColorDragMode::Saturation => hsva[1],
        ColorDragMode::Value => hsva[2],
        ColorDragMode::Alpha => value[3],
        ColorDragMode::Red => value[0],
        ColorDragMode::Green => value[1],
        ColorDragMode::Blue => value[2],
        _ => return,
    };
    let vertical = mode == ColorDragMode::Value;
    let rect = if vertical {
        Rect::from_min_size(
            Pos2::new(
                origin.x - SLIDER_HEIGHT * 0.5,
                origin.y - (1.0 - channel) * width,
            ),
            Vec2::new(SLIDER_HEIGHT, width),
        )
    } else {
        Rect::from_min_size(
            Pos2::new(origin.x - channel * width, origin.y - SLIDER_HEIGHT * 0.5),
            Vec2::new(width, SLIDER_HEIGHT),
        )
    };

    if mode == ColorDragMode::Alpha {
        paint_checkerboard_with(painter, rect, 4.0);
    }
    if vertical {
        paint_vertical_gradient_with(painter, rect, |t| {
            color32(hsva_to_rgba([hsva[0], hsva[1], 1.0 - t, hsva[3]]))
        });
    } else {
        paint_horizontal_gradient_with(painter, rect, |t| match mode {
            ColorDragMode::Saturation => color32(hsva_to_rgba([hsva[0], t, hsva[2], hsva[3]])),
            ColorDragMode::Alpha => color32([value[0], value[1], value[2], t]),
            ColorDragMode::Red => color32([t, value[1], value[2], value[3]]),
            ColorDragMode::Green => color32([value[0], t, value[2], value[3]]),
            ColorDragMode::Blue => color32([value[0], value[1], t, value[3]]),
            _ => Color32::TRANSPARENT,
        });
    }
    painter.rect_stroke(
        rect,
        CornerRadius::same(theme.input_radius),
        Stroke::new(1.0, theme.border),
        StrokeKind::Inside,
    );
}

fn paint_overlay_label(
    painter: &egui::Painter,
    screen: Rect,
    origin: Pos2,
    value: [f32; 4],
    hsva: [f32; 4],
    mode: ColorDragMode,
    theme: &TweeqTheme,
) {
    let text = match mode {
        ColorDragMode::SaturationValue => {
            format!(
                "Sat  {:>5.1}%    Val  {:>5.1}%",
                hsva[1] * 100.0,
                hsva[2] * 100.0
            )
        }
        ColorDragMode::Hue => format!("Hue  {:>5.1}°", hsva[0] * 360.0),
        ColorDragMode::Saturation => format!("Sat  {:>5.1}%", hsva[1] * 100.0),
        ColorDragMode::Value => format!("Val  {:>5.1}%", hsva[2] * 100.0),
        ColorDragMode::Alpha => format!("α  {:>5.1}%", value[3] * 100.0),
        ColorDragMode::Red => format!("R  {:>3.0}", value[0] * 255.0),
        ColorDragMode::Green => format!("G  {:>3.0}", value[1] * 255.0),
        ColorDragMode::Blue => format!("B  {:>3.0}", value[2] * 255.0),
    };
    let font = FontId::monospace(10.0);
    let galley = painter.layout_no_wrap(text, font, theme.text);
    let size = galley.size() + Vec2::new(12.0, 8.0);
    let mut center = Pos2::new(origin.x, origin.y - theme.input_height * 1.7 - size.y * 0.5);
    center.x = center.x.clamp(
        screen.left() + size.x * 0.5 + 4.0,
        screen.right() - size.x * 0.5 - 4.0,
    );
    center.y = center.y.clamp(
        screen.top() + size.y * 0.5 + 4.0,
        screen.bottom() - size.y * 0.5 - 4.0,
    );
    let rect = Rect::from_center_size(center, size);
    painter.rect(
        rect,
        CornerRadius::same(theme.input_radius),
        theme.surface,
        Stroke::new(1.0, theme.border),
        StrokeKind::Inside,
    );
    painter.galley(rect.center() - galley.size() * 0.5, galley, theme.text);
}

fn paint_sv_mesh(ui: &Ui, rect: Rect, hue: f32) {
    paint_sv_mesh_with(ui.painter(), rect, hue);
}

fn paint_sv_mesh_with(painter: &egui::Painter, rect: Rect, hue: f32) {
    paint_sv_mesh_with_opacity(painter, rect, hue, 1.0);
}

fn paint_sv_mesh_with_opacity(painter: &egui::Painter, rect: Rect, hue: f32, opacity: f32) {
    let subdivisions = 16_u32;
    let mut mesh = Mesh::default();
    for y in 0..=subdivisions {
        for x in 0..=subdivisions {
            let saturation = x as f32 / subdivisions as f32;
            let value = 1.0 - y as f32 / subdivisions as f32;
            let position = Pos2::new(
                egui::lerp(rect.left()..=rect.right(), saturation),
                egui::lerp(rect.top()..=rect.bottom(), y as f32 / subdivisions as f32),
            );
            mesh.colored_vertex(
                position,
                color32(hsva_to_rgba([hue, saturation, value, 1.0])).gamma_multiply(opacity),
            );
            if x < subdivisions && y < subdivisions {
                let row = subdivisions + 1;
                let top_left = y * row + x;
                mesh.add_triangle(top_left, top_left + 1, top_left + row);
                mesh.add_triangle(top_left + 1, top_left + row, top_left + row + 1);
            }
        }
    }
    painter.add(Shape::mesh(mesh));
}

fn paint_vertical_gradient_with(
    painter: &egui::Painter,
    rect: Rect,
    color_at: impl Fn(f32) -> Color32,
) {
    let subdivisions = 24_u32;
    let mut mesh = Mesh::default();
    for index in 0..=subdivisions {
        let t = index as f32 / subdivisions as f32;
        let y = egui::lerp(rect.top()..=rect.bottom(), t);
        mesh.colored_vertex(Pos2::new(rect.left(), y), color_at(t));
        mesh.colored_vertex(Pos2::new(rect.right(), y), color_at(t));
        if index < subdivisions {
            let base = index * 2;
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base + 1, base + 2, base + 3);
        }
    }
    painter.add(Shape::mesh(mesh));
}

fn paint_horizontal_gradient(ui: &Ui, rect: Rect, color_at: impl Fn(f32) -> Color32) {
    paint_horizontal_gradient_with(ui.painter(), rect, color_at);
}

fn paint_horizontal_gradient_with(
    painter: &egui::Painter,
    rect: Rect,
    color_at: impl Fn(f32) -> Color32,
) {
    let subdivisions = 24_u32;
    let mut mesh = Mesh::default();
    for index in 0..=subdivisions {
        let t = index as f32 / subdivisions as f32;
        let x = egui::lerp(rect.left()..=rect.right(), t);
        mesh.colored_vertex(Pos2::new(x, rect.top()), color_at(t));
        mesh.colored_vertex(Pos2::new(x, rect.bottom()), color_at(t));
        if index < subdivisions {
            let base = index * 2;
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base + 1, base + 2, base + 3);
        }
    }
    painter.add(Shape::mesh(mesh));
}

fn paint_checkerboard(ui: &Ui, rect: Rect, size: f32) {
    paint_checkerboard_with(ui.painter(), rect, size);
}

fn paint_checkerboard_with(painter: &egui::Painter, rect: Rect, size: f32) {
    let rows = (rect.height() / size).ceil() as i32;
    let columns = (rect.width() / size).ceil() as i32;
    for row in 0..rows {
        for column in 0..columns {
            let min = Pos2::new(
                rect.left() + column as f32 * size,
                rect.top() + row as f32 * size,
            );
            let cell = Rect::from_min_size(min, Vec2::splat(size)).intersect(rect);
            let color = if (row + column) % 2 == 0 {
                Color32::from_gray(70)
            } else {
                Color32::from_gray(125)
            };
            painter.rect_filled(cell, 0, color);
        }
    }
}

fn colors_differ(left: [f32; 4], right: [f32; 4]) -> bool {
    left.into_iter()
        .zip(right)
        .any(|(left, right)| left.to_bits() != right.to_bits())
}

fn color32(value: [f32; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (value[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (value[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (value[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        (value[3].clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

fn contrast_color(color: Color32) -> Color32 {
    let luminance = 0.2126 * f32::from(color.r())
        + 0.7152 * f32::from(color.g())
        + 0.0722 * f32::from(color.b());
    if luminance < 140.0 {
        Color32::WHITE
    } else {
        Color32::BLACK
    }
}

fn rgba_to_hex(value: [f32; 4]) -> String {
    let color = color32(value);
    if color.a() == 255 {
        format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())
    } else {
        format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            color.r(),
            color.g(),
            color.b(),
            color.a()
        )
    }
}

fn rgba_to_opaque_hex(value: [f32; 4]) -> String {
    let color = color32(value);
    format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())
}

fn parse_hex(text: &str) -> Option<[f32; 4]> {
    let text = text.trim().trim_start_matches('#');
    if text.len() != 6 && text.len() != 8 {
        return None;
    }
    let red = u8::from_str_radix(&text[0..2], 16).ok()?;
    let green = u8::from_str_radix(&text[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&text[4..6], 16).ok()?;
    let alpha = if text.len() == 8 {
        u8::from_str_radix(&text[6..8], 16).ok()?
    } else {
        255
    };
    Some([
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        f32::from(alpha) / 255.0,
    ])
}

#[cfg(test)]
mod tests {
    use super::{hsva_to_rgba, parse_hex, rgba_to_hex, rgba_to_hsva};

    #[test]
    fn hsv_round_trip_preserves_rgba() {
        let source = [0.16, 0.42, 1.0, 0.75];
        let round_trip = hsva_to_rgba(rgba_to_hsva(source));
        for (actual, expected) in round_trip.into_iter().zip(source) {
            assert!((actual - expected).abs() < 0.000_1);
        }
    }

    #[test]
    fn hex_supports_rgb_and_rgba() {
        assert_eq!(rgba_to_hex([1.0, 0.0, 0.5, 1.0]), "#FF0080");
        let parsed = parse_hex("#33669980").expect("valid RGBA hex");
        assert!((parsed[3] - 128.0 / 255.0).abs() < f32::EPSILON);
        assert!(parse_hex("#xyz").is_none());
    }
}
