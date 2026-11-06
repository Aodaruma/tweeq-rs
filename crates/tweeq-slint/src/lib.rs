//! Editing bridge for the Tweeq Slint controls.
//!
//! Slint views carry display strings and pointer coordinates. The host owns an
//! [`Editor`] and observes its edit events to persist values or record history.

mod color;
mod editor;
mod rotary;

pub use color::{ColorChannel, ColorView};
pub use editor::{Editor, NumberOptions, NumberView, ScrubStart};
pub use tweeq_core::{
    EditEvent, EditOperation, EditSessionId, GestureModifiers, NumberConstraints, ParamId,
    ParamSnapshot, ParamValue, hsva_to_rgba,
};
