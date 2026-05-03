use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AdminEvent {
    pub subject:    String,
    pub event_type: String,
    pub properties: HashMap<String, String>,
}

// Placeholder — full implementation in Phase 3.
// The event stream connection reads frames in a loop from admin.Events.
// Each frame: 0x31 \0 subject \0 event_type \0 [key \0 value \0]* \0\0
