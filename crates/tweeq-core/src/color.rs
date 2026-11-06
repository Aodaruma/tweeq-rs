/// Converts normalized RGBA to HSVA, with hue expressed in turns.
#[must_use]
pub fn rgba_to_hsva(rgba: [f32; 4]) -> [f32; 4] {
    let red = rgba[0].clamp(0.0, 1.0);
    let green = rgba[1].clamp(0.0, 1.0);
    let blue = rgba[2].clamp(0.0, 1.0);
    let maximum = red.max(green).max(blue);
    let minimum = red.min(green).min(blue);
    let delta = maximum - minimum;
    let hue = if delta <= f32::EPSILON {
        0.0
    } else if red >= green && red >= blue {
        ((green - blue) / delta).rem_euclid(6.0) / 6.0
    } else if green >= red && green >= blue {
        ((blue - red) / delta + 2.0) / 6.0
    } else {
        ((red - green) / delta + 4.0) / 6.0
    };
    let saturation = if maximum <= f32::EPSILON {
        0.0
    } else {
        delta / maximum
    };
    [hue, saturation, maximum, rgba[3].clamp(0.0, 1.0)]
}

/// Converts HSVA in turns and normalized channels to normalized RGBA.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn hsva_to_rgba(hsva: [f32; 4]) -> [f32; 4] {
    let hue = hsva[0].rem_euclid(1.0) * 6.0;
    let saturation = hsva[1].clamp(0.0, 1.0);
    let value = hsva[2].clamp(0.0, 1.0);
    let chroma = value * saturation;
    let x = chroma * (1.0 - (hue.rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = match hue as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let match_value = value - chroma;
    [
        red + match_value,
        green + match_value,
        blue + match_value,
        hsva[3].clamp(0.0, 1.0),
    ]
}
