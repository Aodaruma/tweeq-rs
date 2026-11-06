//! Build-time distribution of the Tweeq Slint controls.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Writes the bundled controls and returns the `@tweeq` entry file.
///
/// Pass a directory inside Cargo's `OUT_DIR` from the consuming build script.
///
/// # Errors
///
/// Returns an I/O error if the directory or any bundled file cannot be written.
pub fn export_ui(destination: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(destination)?;
    for (name, source) in [
        ("tweeq.slint", include_str!("../ui/tweeq.slint")),
        ("bridge.slint", include_str!("../ui/bridge.slint")),
        ("number.slint", include_str!("../ui/number.slint")),
        ("scrub.slint", include_str!("../ui/scrub.slint")),
        ("angle.slint", include_str!("../ui/angle.slint")),
        ("color.slint", include_str!("../ui/color.slint")),
    ] {
        let file = destination.join(name);
        if fs::read_to_string(&file)
            .as_deref()
            .is_ok_and(|old| old == source)
        {
            continue;
        }
        fs::write(file, source)?;
    }
    Ok(destination.join("tweeq.slint"))
}
