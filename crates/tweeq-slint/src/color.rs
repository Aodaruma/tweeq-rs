use tweeq_core::{hsva_to_rgba, rgba_to_hsva};

/// Channel edited by the Slint color controls.
#[derive(Debug, Clone, Copy)]
pub enum ColorChannel {
    Red,
    Green,
    Blue,
    Alpha,
    Hue,
    Saturation,
    Value,
}

/// The color and channel values shown by the Slint controls.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorView {
    pub rgba: [f32; 4],
    pub hsva: [f32; 4],
    pub hex: String,
    pub selected: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ColorParameter {
    pub(crate) rgba: [f32; 4],
    pub(crate) hue: f32,
}

impl ColorParameter {
    pub(crate) fn new(rgba: [f32; 4]) -> Self {
        let mut parameter = Self {
            rgba: [0.0, 0.0, 0.0, 1.0],
            hue: 0.0,
        };
        parameter.set(rgba);
        parameter
    }

    pub(crate) fn set(&mut self, rgba: [f32; 4]) {
        if !rgba.into_iter().all(f32::is_finite) {
            return;
        }
        self.rgba = rgba.map(|value| value.clamp(0.0, 1.0));
        let [hue, saturation, _, _] = rgba_to_hsva(self.rgba);
        if saturation > f32::EPSILON {
            self.hue = hue;
        }
    }

    pub(crate) fn hsva(self) -> [f32; 4] {
        let mut hsva = rgba_to_hsva(self.rgba);
        hsva[0] = self.hue;
        hsva
    }

    pub(crate) fn change(&mut self, channel: ColorChannel, value: f32) -> [f32; 4] {
        let value = value.clamp(0.0, 1.0);
        let mut rgba = self.rgba;
        let mut hsva = self.hsva();
        match channel {
            ColorChannel::Red => rgba[0] = value,
            ColorChannel::Green => rgba[1] = value,
            ColorChannel::Blue => rgba[2] = value,
            ColorChannel::Alpha => rgba[3] = value,
            ColorChannel::Hue => {
                self.hue = value.rem_euclid(1.0);
                hsva[0] = self.hue;
                return hsva_to_rgba(hsva);
            }
            ColorChannel::Saturation => {
                hsva[1] = value;
                return hsva_to_rgba(hsva);
            }
            ColorChannel::Value => {
                hsva[2] = value;
                return hsva_to_rgba(hsva);
            }
        }
        rgba
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn format_hex(rgba: [f32; 4]) -> String {
    let [red, green, blue, alpha] = rgba.map(|value| (value.clamp(0.0, 1.0) * 255.0).round() as u8);
    if alpha == 255 {
        format!("#{red:02X}{green:02X}{blue:02X}")
    } else {
        format!("#{red:02X}{green:02X}{blue:02X}{alpha:02X}")
    }
}

pub(crate) fn parse_hex(text: &str) -> Option<[f32; 4]> {
    let text = text.trim();
    let text = text.strip_prefix('#').unwrap_or(text);
    if !matches!(text.len(), 6 | 8) || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut rgba = [1.0; 4];
    for (index, channel) in rgba.iter_mut().enumerate().take(text.len() / 2) {
        *channel = f32::from(u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).ok()?) / 255.0;
    }
    Some(rgba)
}
