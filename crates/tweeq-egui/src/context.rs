use std::collections::HashMap;

use tweeq_core::{
    EditEvent, EditOperation, EditSessionId, EditSessions, ParamId, ParamKind, ParamSnapshot,
    ParamValue, Selection, TweakGesture,
};

use crate::TweeqTheme;

#[derive(Debug, Clone)]
pub(crate) struct NumberState {
    pub editing: bool,
    pub invalid: bool,
    pub buffer: String,
    pub captured: f64,
    pub drag_base: f64,
    pub gesture: TweakGesture,
    pub last_drag_total: egui::Vec2,
    pub session: Option<EditSessionId>,
    pub virtual_position: Option<egui::Pos2>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ScalarDragState {
    pub captured: f64,
    pub session: Option<EditSessionId>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum RotaryMode {
    Absolute,
    #[default]
    Relative,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RotaryState {
    pub captured: f64,
    pub local: f64,
    pub origin: egui::Pos2,
    pub previous: egui::Pos2,
    pub current: egui::Pos2,
    pub pointer_mode: RotaryMode,
    pub session: Option<EditSessionId>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct VectorDragState {
    pub captured: [f64; 2],
    pub last_total: egui::Vec2,
    pub session: Option<EditSessionId>,
}

impl NumberState {
    pub fn new(value: f64, buffer: String) -> Self {
        Self {
            editing: false,
            invalid: false,
            buffer,
            captured: value,
            drag_base: value,
            gesture: TweakGesture::default(),
            last_drag_total: egui::Vec2::ZERO,
            session: None,
            virtual_position: None,
        }
    }
}

/// App-owned Tweeq state shared by widgets.
#[derive(Debug, Clone)]
pub struct TweeqContext {
    theme: TweeqTheme,
    selection: Selection,
    known_kinds: HashMap<ParamId, ParamKind>,
    known_values: HashMap<ParamId, ParamValue>,
    current_order: Vec<ParamId>,
    previous_order: Vec<ParamId>,
    number_states: HashMap<ParamId, NumberState>,
    scalar_drag_states: HashMap<ParamId, ScalarDragState>,
    rotary_states: HashMap<ParamId, RotaryState>,
    vector_drag_states: HashMap<ParamId, VectorDragState>,
    sessions: EditSessions,
}

impl TweeqContext {
    /// Creates a context using the given theme.
    #[must_use]
    pub fn new(theme: TweeqTheme) -> Self {
        Self {
            theme,
            selection: Selection::default(),
            known_kinds: HashMap::new(),
            known_values: HashMap::new(),
            current_order: Vec::new(),
            previous_order: Vec::new(),
            number_states: HashMap::new(),
            scalar_drag_states: HashMap::new(),
            rotary_states: HashMap::new(),
            vector_drag_states: HashMap::new(),
            sessions: EditSessions::default(),
        }
    }

    /// Returns the active theme.
    #[must_use]
    pub const fn theme(&self) -> &TweeqTheme {
        &self.theme
    }

    /// Replaces the active theme.
    pub fn set_theme(&mut self, theme: TweeqTheme) {
        self.theme = theme;
    }

    /// Starts registration for a new egui frame.
    pub fn begin_frame(&mut self) {
        self.previous_order = std::mem::take(&mut self.current_order);
    }

    /// Drains edit events emitted since the previous call.
    pub fn drain_events(&mut self) -> impl Iterator<Item = EditEvent> + '_ {
        self.sessions.drain()
    }

    /// Returns whether a parameter is part of the current selection.
    #[must_use]
    pub fn is_selected(&self, id: ParamId) -> bool {
        self.selection.contains(id)
    }

    pub(crate) fn register(&mut self, id: ParamId, kind: ParamKind) {
        self.known_kinds.insert(id, kind);
        if !self.current_order.contains(&id) {
            self.current_order.push(id);
        }
    }

    pub(crate) fn register_number(&mut self, id: ParamId, value: f64) {
        self.register(id, ParamKind::Number);
        self.known_values.insert(id, ParamValue::Number(value));
    }

    pub(crate) fn register_boolean(&mut self, id: ParamId, value: bool) {
        self.register(id, ParamKind::Boolean);
        self.known_values.insert(id, ParamValue::Boolean(value));
    }

    pub(crate) fn register_string(&mut self, id: ParamId, value: &str) {
        self.register(id, ParamKind::String);
        self.known_values
            .insert(id, ParamValue::String(value.to_owned()));
    }

    pub(crate) fn register_vector(&mut self, id: ParamId, value: [f64; 2]) {
        self.register(id, ParamKind::Vector);
        self.known_values.insert(
            id,
            ParamValue::Vector {
                value: [value[0], value[1], 0.0, 0.0],
                dimensions: 2,
            },
        );
    }

    pub(crate) fn register_color(&mut self, id: ParamId, value: [f32; 4]) {
        self.register(id, ParamKind::Color);
        self.known_values.insert(id, ParamValue::Color(value));
    }

    pub(crate) fn activate_selection(
        &mut self,
        id: ParamId,
        kind: ParamKind,
        shift: bool,
        command: bool,
    ) {
        self.register(id, kind);
        if shift {
            let order = if self.previous_order.contains(&id) {
                &self.previous_order
            } else {
                &self.current_order
            };
            self.selection.select_range(order, id);
        } else if command {
            self.selection.toggle(id);
        } else if !self.selection.contains(id) {
            self.selection.select_only(id);
        }
    }

    pub(crate) fn start_edit(&mut self, source: ParamId, kind: ParamKind) -> EditSessionId {
        let mut target_ids: Vec<_> = self
            .selection
            .iter()
            .filter(|id| self.known_kinds.get(id) == Some(&kind))
            .collect();
        if !target_ids.contains(&source) {
            target_ids.push(source);
        }
        let targets = target_ids
            .into_iter()
            .filter_map(|id| {
                self.known_values
                    .get(&id)
                    .cloned()
                    .map(|value| ParamSnapshot { id, value })
            })
            .collect();
        self.sessions.begin(source, targets)
    }

    pub(crate) fn start_exclusive_edit(
        &mut self,
        source: ParamId,
        kind: ParamKind,
    ) -> EditSessionId {
        self.register(source, kind);
        self.selection.select_only(source);
        self.start_edit(source, kind)
    }

    pub(crate) fn update_edit(&mut self, session: EditSessionId, operation: EditOperation) {
        self.sessions.update(session, operation);
    }

    pub(crate) fn finish_edit(&mut self, session: EditSessionId, commit: bool) {
        self.sessions.finish(session, commit);
    }

    pub(crate) fn immediate_edit(
        &mut self,
        source: ParamId,
        kind: ParamKind,
        operation: EditOperation,
    ) {
        self.activate_selection(source, kind, false, false);
        let session = self.start_edit(source, kind);
        self.update_edit(session, operation);
        self.finish_edit(session, true);
    }

    pub(crate) fn take_number_state(
        &mut self,
        id: ParamId,
        value: f64,
        buffer: String,
    ) -> NumberState {
        self.number_states
            .remove(&id)
            .unwrap_or_else(|| NumberState::new(value, buffer))
    }

    pub(crate) fn put_number_state(&mut self, id: ParamId, state: NumberState) {
        self.number_states.insert(id, state);
    }

    pub(crate) fn take_scalar_drag_state(&mut self, id: ParamId) -> ScalarDragState {
        self.scalar_drag_states.remove(&id).unwrap_or_default()
    }

    pub(crate) fn put_scalar_drag_state(&mut self, id: ParamId, state: ScalarDragState) {
        self.scalar_drag_states.insert(id, state);
    }

    pub(crate) fn take_rotary_state(&mut self, id: ParamId) -> RotaryState {
        self.rotary_states.remove(&id).unwrap_or_default()
    }

    pub(crate) fn put_rotary_state(&mut self, id: ParamId, state: RotaryState) {
        self.rotary_states.insert(id, state);
    }

    pub(crate) fn take_vector_drag_state(&mut self, id: ParamId) -> VectorDragState {
        self.vector_drag_states.remove(&id).unwrap_or_default()
    }

    pub(crate) fn put_vector_drag_state(&mut self, id: ParamId, state: VectorDragState) {
        self.vector_drag_states.insert(id, state);
    }
}

impl Default for TweeqContext {
    fn default() -> Self {
        Self::new(TweeqTheme::dark())
    }
}

#[cfg(test)]
mod tests {
    use super::TweeqContext;
    use tweeq_core::{EditEvent, ParamId, ParamKind};

    #[test]
    fn exclusive_edit_drops_an_unrelated_numeric_selection() {
        let selected = ParamId::from_static("test.selected-number");
        let timeline = ParamId::from_static("test.timeline");
        let mut context = TweeqContext::default();
        context.register_number(selected, 12.0);
        context.register_number(timeline, 24.0);
        context.activate_selection(selected, ParamKind::Number, false, false);

        context.start_exclusive_edit(timeline, ParamKind::Number);

        let event = context.drain_events().next().expect("begin event");
        let EditEvent::Begin {
            source, targets, ..
        } = event
        else {
            panic!("expected begin event");
        };
        assert_eq!(source, timeline);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].id, timeline);
    }
}
