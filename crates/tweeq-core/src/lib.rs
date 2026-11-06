//! Renderer-independent interaction semantics for Tweeq.
//!
//! Values remain owned by the host application. Tweeq emits explicit edit
//! sessions so applications can implement preview, multi-edit, and undo without
//! storing mutable references across UI frames.
//!
//! ```
//! use tweeq_core::{NumberConstraints, ParamId};
//!
//! let opacity = ParamId::from_static("opacity");
//! let constraints = NumberConstraints {
//!     min: Some(0.0),
//!     max: Some(1.0),
//!     step: Some(0.01),
//!     precision: 3,
//!     ..NumberConstraints::default()
//! };
//! assert_eq!(opacity, ParamId::from_static("opacity"));
//! assert_eq!(constraints.validate(1.2, true).value, 1.0);
//! ```

#![forbid(unsafe_code)]

mod edit;
mod gesture;
mod id;
mod selection;
mod validation;
mod workspace;

pub use edit::{
    EditEvent, EditOperation, EditSessionId, EditSessions, ParamKind, ParamSnapshot, ParamValue,
};
pub use gesture::{GestureModifiers, GestureUpdate, TweakGesture};
pub use id::ParamId;
pub use selection::Selection;
pub use validation::{NumberConstraints, NumberValidation, quantize};
pub use workspace::{ViewTransform, nice_tick_step, seeded_unit};

/// The workspace package version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_workspace_version() {
        assert!(!super::VERSION.is_empty());
    }
}
