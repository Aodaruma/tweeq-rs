use crate::ParamId;

/// Monotonic identity for one editing transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EditSessionId(u64);

impl EditSessionId {
    /// Creates a session ID from a context-local counter.
    #[must_use]
    pub const fn from_u64(value: u64) -> Self {
        Self(value)
    }

    /// Returns the numeric representation.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Compatibility class used to restrict simultaneous edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamKind {
    Number,
    Boolean,
    String,
    Vector,
    Color,
}

/// A value transformation interpreted against values captured at `Begin`.
#[derive(Debug, Clone, PartialEq)]
pub enum EditOperation {
    SetNumber(f64),
    AddNumber(f64),
    ScaleNumber(f64),
    SetBoolean(bool),
    SetString(String),
    AddVector { delta: [f64; 4], dimensions: u8 },
    SetColor([f32; 4]),
}

/// Typed parameter value captured before an edit mutates the model.
#[derive(Debug, Clone, PartialEq)]
pub enum ParamValue {
    Number(f64),
    Boolean(bool),
    String(String),
    Vector { value: [f64; 4], dimensions: u8 },
    Color([f32; 4]),
}

/// One target and its value at the start of a transaction.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamSnapshot {
    pub id: ParamId,
    pub value: ParamValue,
}

/// Explicit edit lifecycle consumed by host applications and undo systems.
#[derive(Debug, Clone, PartialEq)]
pub enum EditEvent {
    Begin {
        session: EditSessionId,
        source: ParamId,
        targets: Vec<ParamSnapshot>,
    },
    Update {
        session: EditSessionId,
        operation: EditOperation,
    },
    Commit {
        session: EditSessionId,
    },
    Cancel {
        session: EditSessionId,
    },
}

impl EditEvent {
    /// Returns the transaction associated with the event.
    #[must_use]
    pub const fn session(&self) -> EditSessionId {
        match *self {
            Self::Begin { session, .. }
            | Self::Update { session, .. }
            | Self::Commit { session }
            | Self::Cancel { session } => session,
        }
    }
}

/// Allocates session IDs and queues the edit lifecycle for a GUI adapter.
#[derive(Debug, Clone)]
pub struct EditSessions {
    next: u64,
    events: Vec<EditEvent>,
}

impl Default for EditSessions {
    fn default() -> Self {
        Self {
            next: 1,
            events: Vec::new(),
        }
    }
}

impl EditSessions {
    /// Records the values captured before editing and returns the session ID.
    pub fn begin(&mut self, source: ParamId, targets: Vec<ParamSnapshot>) -> EditSessionId {
        let session = EditSessionId::from_u64(self.next);
        self.next = self.next.wrapping_add(1).max(1);
        self.events.push(EditEvent::Begin {
            session,
            source,
            targets,
        });
        session
    }

    /// Queues a preview operation against the captured values.
    pub fn update(&mut self, session: EditSessionId, operation: EditOperation) {
        self.events.push(EditEvent::Update { session, operation });
    }

    /// Queues the terminal event selected by the adapter.
    pub fn finish(&mut self, session: EditSessionId, commit: bool) {
        self.events.push(if commit {
            EditEvent::Commit { session }
        } else {
            EditEvent::Cancel { session }
        });
    }

    /// Removes and returns pending events in emission order.
    pub fn drain(&mut self) -> impl Iterator<Item = EditEvent> + '_ {
        self.events.drain(..)
    }
}
