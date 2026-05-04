#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::admin::{AdminClient, QubeClass, QubeInfo, QubeProperties, QubeState};

use crate::action::{Action, OpKind, SideEffect};

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum ActiveView {
    QubeManager,
    ServiceManager,
    TemplateManager,
    WhonixManager,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Modal {
    None,
    Help,
    Detail,
    ConfirmDelete {
        vm_name: String,
    },
    ConfirmKill {
        vm_name: String,
    },
    ChangeNetvm {
        vm_name: String,
        candidates: Vec<String>, // "None" + names of network-providing VMs
        selected: usize,
    },
    EditProperty {
        vm_name: String,
        property: String,
        input: String,
    },
}

#[derive(Debug, Clone)]
pub struct PendingOp {
    pub op_id: u64,
    pub vm_name: String,
    pub kind: OpKind,
    pub started: Instant,
}

#[derive(Debug, Clone)]
pub enum MessageLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub level: MessageLevel,
}

pub struct App {
    pub client: Arc<AdminClient>,
    pub active_view: ActiveView,
    pub qubes: Vec<QubeInfo>,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
    pub modal: Modal,
    pub detail_scroll: u16,
    /// Which row in the detail popup is highlighted (for editing).
    pub detail_row: usize,
    /// Ordered list of (property_key, display_label) rows shown in the detail popup.
    /// Rebuilt by the UI layer and stored here so App::update() can look up by index.
    pub detail_rows: Vec<(String, String)>,
    pub properties_cache: HashMap<String, QubeProperties>,
    pub pending_ops: Vec<PendingOp>,
    pub next_op_id: u64,
    pub status: Option<StatusMessage>,
    pub should_quit: bool,
}

impl App {
    pub fn new(client: Arc<AdminClient>) -> Self {
        Self {
            client,
            active_view: ActiveView::QubeManager,
            qubes: Vec::new(),
            filtered_indices: Vec::new(),
            selected_index: 0,
            modal: Modal::None,
            detail_scroll: 0,
            detail_row: 0,
            detail_rows: Vec::new(),
            properties_cache: HashMap::new(),
            pending_ops: Vec::new(),
            next_op_id: 0,
            status: None,
            should_quit: false,
        }
    }

    /// Pure state mutation. Returns side effects that main.rs will execute.
    pub fn update(&mut self, action: Action) -> Vec<SideEffect> {
        // If a confirmation modal is open, only handle confirm/cancel/quit
        match &self.modal.clone() {
            Modal::ConfirmDelete { vm_name } => {
                let name = vm_name.clone();
                return match action {
                    Action::Confirm => {
                        self.modal = Modal::None;
                        self.push_op(name.clone(), OpKind::Delete);
                        vec![SideEffect::DeleteVm(name)]
                    }
                    Action::Cancel | Action::Quit => {
                        self.modal = Modal::None;
                        vec![]
                    }
                    _ => vec![],
                };
            }
            Modal::ConfirmKill { vm_name } => {
                let name = vm_name.clone();
                return match action {
                    Action::Confirm => {
                        self.modal = Modal::None;
                        self.push_op(name.clone(), OpKind::Kill);
                        vec![SideEffect::KillVm(name)]
                    }
                    Action::Cancel | Action::Quit => {
                        self.modal = Modal::None;
                        vec![]
                    }
                    _ => vec![],
                };
            }
            Modal::Help => {
                return match action {
                    Action::Cancel | Action::HideHelp | Action::ShowHelp | Action::Quit => {
                        if matches!(action, Action::Quit) {
                            self.should_quit = true;
                        }
                        self.modal = Modal::None;
                        vec![]
                    }
                    _ => vec![],
                };
            }
            Modal::Detail => {
                return match action {
                    Action::Cancel | Action::ToggleDetail | Action::Quit => {
                        if matches!(action, Action::Quit) {
                            self.should_quit = true;
                        }
                        self.modal = Modal::None;
                        vec![]
                    }
                    Action::MoveUp => {
                        if self.detail_row > 0 {
                            self.detail_row -= 1;
                        }
                        // Scroll up if needed
                        if (self.detail_row as u16) < self.detail_scroll {
                            self.detail_scroll = self.detail_row as u16;
                        }
                        vec![]
                    }
                    Action::MoveDown => {
                        if self.detail_row + 1 < self.detail_rows.len() {
                            self.detail_row += 1;
                        }
                        vec![]
                    }
                    Action::EditProperty => {
                        if let Some((key, _label)) = self.detail_rows.get(self.detail_row) {
                            let vm_name = self
                                .selected_qube()
                                .map(|q| q.name.clone())
                                .unwrap_or_default();
                            // Pre-fill current value from cache
                            let current = self
                                .properties_cache
                                .get(&vm_name)
                                .and_then(|p| p.raw.get(key))
                                .cloned()
                                .unwrap_or_default();
                            let property = key.clone();
                            self.modal = Modal::EditProperty {
                                vm_name,
                                property,
                                input: current,
                            };
                        }
                        vec![]
                    }
                    _ => vec![],
                };
            }
            Modal::ChangeNetvm {
                vm_name,
                candidates,
                selected,
            } => {
                let name = vm_name.clone();
                let candidates = candidates.clone();
                let mut selected = *selected;
                return match action {
                    Action::MoveUp => {
                        if selected > 0 {
                            selected -= 1;
                        }
                        self.modal = Modal::ChangeNetvm { vm_name: name, candidates, selected };
                        vec![]
                    }
                    Action::MoveDown => {
                        if selected + 1 < candidates.len() {
                            selected += 1;
                        }
                        self.modal = Modal::ChangeNetvm { vm_name: name, candidates, selected };
                        vec![]
                    }
                    Action::Confirm | Action::EditSubmit => {
                        let value = candidates[selected].clone();
                        self.modal = Modal::None;
                        self.properties_cache.remove(&name);
                        // Reflect immediately in the qube list
                        if let Some(q) = self.qubes.iter_mut().find(|q| q.name == name) {
                            q.netvm = if value == "None" { None } else { Some(value.clone()) };
                        }
                        vec![
                            SideEffect::SetProperty {
                                vm: name.clone(),
                                property: "netvm".into(),
                                value,
                            },
                            SideEffect::FetchProperties(name),
                        ]
                    }
                    Action::Cancel | Action::Quit => {
                        if matches!(action, Action::Quit) {
                            self.should_quit = true;
                        }
                        self.modal = Modal::None;
                        vec![]
                    }
                    _ => vec![],
                };
            }
            Modal::EditProperty {
                vm_name,
                property,
                input,
            } => {
                let vm_name = vm_name.clone();
                let property = property.clone();
                let mut input = input.clone();
                return match action {
                    Action::EditChar(c) => {
                        input.push(c);
                        self.modal = Modal::EditProperty {
                            vm_name,
                            property,
                            input,
                        };
                        vec![]
                    }
                    Action::EditBackspace => {
                        input.pop();
                        self.modal = Modal::EditProperty {
                            vm_name,
                            property,
                            input,
                        };
                        vec![]
                    }
                    Action::EditSubmit => {
                        self.modal = Modal::Detail;
                        // Invalidate cache so fresh props are fetched after the set
                        self.properties_cache.remove(&vm_name);
                        vec![
                            SideEffect::SetProperty {
                                vm: vm_name.clone(),
                                property,
                                value: input,
                            },
                            SideEffect::FetchProperties(vm_name),
                        ]
                    }
                    Action::Cancel | Action::Quit => {
                        self.modal = Modal::Detail;
                        vec![]
                    }
                    _ => vec![],
                };
            }
            Modal::None => {}
        }

        match action {
            Action::Quit => {
                self.should_quit = true;
                vec![]
            }
            Action::ShowHelp => {
                self.modal = Modal::Help;
                vec![]
            }
            Action::HideHelp => {
                self.modal = Modal::None;
                vec![]
            }
            Action::Cancel => {
                self.modal = Modal::None;
                vec![]
            }

            // Enter key — opens/closes the detail popup (EditSubmit is also Enter;
            // when no edit modal is active, treat it identically to ToggleDetail).
            Action::ToggleDetail | Action::EditSubmit => {
                if self.modal == Modal::Detail {
                    self.modal = Modal::None;
                    vec![]
                } else if self.selected_qube().is_some() {
                    self.modal = Modal::Detail;
                    self.detail_scroll = 0;
                    self.detail_row = 0;
                    // Trigger property fetch if not cached
                    if let Some(name) = self.selected_qube().map(|q| q.name.clone()) {
                        if !self.properties_cache.contains_key(&name) {
                            return vec![SideEffect::FetchProperties(name)];
                        }
                    }
                    vec![]
                } else {
                    vec![]
                }
            }

            Action::MoveUp => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.on_selection_changed()
                } else {
                    vec![]
                }
            }
            Action::MoveDown => {
                if !self.filtered_indices.is_empty()
                    && self.selected_index < self.filtered_indices.len() - 1
                {
                    self.selected_index += 1;
                    self.on_selection_changed()
                } else {
                    vec![]
                }
            }
            Action::MoveTop => {
                self.selected_index = 0;
                self.on_selection_changed()
            }
            Action::MoveBottom => {
                if !self.filtered_indices.is_empty() {
                    self.selected_index = self.filtered_indices.len() - 1;
                }
                self.on_selection_changed()
            }

            Action::SwitchToQubeManager => {
                self.active_view = ActiveView::QubeManager;
                self.selected_index = 0;
                self.rebuild_filtered();
                vec![]
            }
            Action::SwitchToServiceManager => {
                self.active_view = ActiveView::ServiceManager;
                self.selected_index = 0;
                self.rebuild_filtered();
                vec![]
            }
            Action::SwitchToTemplateManager => {
                self.active_view = ActiveView::TemplateManager;
                self.selected_index = 0;
                self.rebuild_filtered();
                vec![]
            }
            Action::SwitchToWhonixManager => {
                self.active_view = ActiveView::WhonixManager;
                self.selected_index = 0;
                self.rebuild_filtered();
                vec![]
            }

            Action::StartSelected => {
                if let Some(name) = self.selected_halted_name() {
                    self.push_op(name.clone(), OpKind::Start);
                    vec![SideEffect::StartVm(name)]
                } else {
                    vec![]
                }
            }
            Action::ShutdownSelected => {
                if let Some(name) = self.selected_running_name() {
                    self.push_op(name.clone(), OpKind::Shutdown);
                    vec![SideEffect::ShutdownVm(name)]
                } else {
                    vec![]
                }
            }
            Action::KillSelected => {
                if let Some(name) = self.selected_running_name() {
                    self.modal = Modal::ConfirmKill { vm_name: name };
                }
                vec![]
            }
            Action::PauseSelected => {
                if let Some(q) = self.selected_qube() {
                    let name = q.name.clone();
                    match &q.state {
                        QubeState::Running => {
                            self.push_op(name.clone(), OpKind::Start);
                            return vec![SideEffect::PauseVm(name)];
                        }
                        QubeState::Paused => {
                            self.push_op(name.clone(), OpKind::Start);
                            // unpause — treated as Start kind visually
                            return vec![SideEffect::StartVm(name)];
                        }
                        _ => {}
                    }
                }
                vec![]
            }
            Action::OpenTerminal => {
                if let Some(name) = self.selected_running_name() {
                    vec![SideEffect::OpenTerminal(name)]
                } else {
                    vec![]
                }
            }
            Action::DeleteSelected => {
                if let Some(q) = self.selected_qube() {
                    if q.state == QubeState::Halted {
                        let name = q.name.clone();
                        self.modal = Modal::ConfirmDelete { vm_name: name };
                    } else {
                        self.status = Some(StatusMessage {
                            text: "Shut down the qube before deleting".into(),
                            level: MessageLevel::Warning,
                        });
                    }
                }
                vec![]
            }
            Action::ChangeNetvm => {
                if let Some(q) = self.selected_qube() {
                    let vm_name = q.name.clone();
                    let current = q.netvm.clone();
                    let mut candidates: Vec<String> = self
                        .qubes
                        .iter()
                        .filter(|q| {
                            q.state == QubeState::Running
                                || self
                                    .properties_cache
                                    .get(&q.name)
                                    .and_then(|p| p.provides_network)
                                    .unwrap_or(false)
                        })
                        .filter(|q| q.name != vm_name)
                        .map(|q| q.name.clone())
                        .collect();
                    candidates.sort();
                    candidates.insert(0, "None".into());
                    let selected = current
                        .as_deref()
                        .and_then(|c| candidates.iter().position(|n| n == c))
                        .unwrap_or(0);
                    self.modal = Modal::ChangeNetvm { vm_name, candidates, selected };
                }
                vec![]
            }

            // ── Async results ─────────────────────────────────────────────────
            Action::QubeListLoaded(qubes) => {
                self.qubes = qubes;
                self.rebuild_filtered();
                // Clamp selection
                if !self.filtered_indices.is_empty() {
                    self.selected_index = self.selected_index.min(self.filtered_indices.len() - 1);
                }
                vec![]
            }
            Action::PropertiesLoaded { name, props } => {
                self.properties_cache.insert(name, props);
                vec![]
            }
            Action::OperationCompleted { op_id } => {
                if let Some(pos) = self.pending_ops.iter().position(|o| o.op_id == op_id) {
                    let op = self.pending_ops.remove(pos);
                    self.status = Some(StatusMessage {
                        text: format!("{} {:?} — done", op.vm_name, op.kind),
                        level: MessageLevel::Success,
                    });
                }
                // Refresh list after any op
                vec![SideEffect::FetchQubeList]
            }
            Action::OperationFailed { op_id, error } => {
                if let Some(pos) = self.pending_ops.iter().position(|o| o.op_id == op_id) {
                    let op = self.pending_ops.remove(pos);
                    self.status = Some(StatusMessage {
                        text: format!("{} {:?} failed: {}", op.vm_name, op.kind, error),
                        level: MessageLevel::Error,
                    });
                }
                vec![]
            }
            Action::EventReceived(evt) => {
                self.apply_event(evt);
                vec![]
            }
            // These are only meaningful inside modals handled above; ignore otherwise.
            Action::EditProperty | Action::EditChar(_) | Action::EditBackspace => vec![],

            Action::Tick | Action::Confirm => vec![],
        }
    }

    pub fn selected_qube(&self) -> Option<&QubeInfo> {
        self.filtered_indices
            .get(self.selected_index)
            .and_then(|&i| self.qubes.get(i))
    }

    pub fn rebuild_filtered(&mut self) {
        self.filtered_indices = self
            .qubes
            .iter()
            .enumerate()
            .filter_map(|(i, q)| {
                let name_lc = q.name.to_lowercase();
                let keep = match self.active_view {
                    ActiveView::QubeManager => {
                        !matches!(q.class, QubeClass::TemplateVM | QubeClass::AdminVM)
                            && !q.name.starts_with("sys-")
                            && !name_lc.contains("whonix")
                    }
                    ActiveView::ServiceManager => {
                        matches!(q.class, QubeClass::AdminVM) || q.name.starts_with("sys-")
                    }
                    ActiveView::TemplateManager => matches!(q.class, QubeClass::TemplateVM),
                    ActiveView::WhonixManager => name_lc.contains("whonix"),
                };
                if keep {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();
    }

    fn on_selection_changed(&mut self) -> Vec<SideEffect> {
        // Pre-fetch properties for newly selected qube if detail panel is open
        if self.modal == Modal::Detail {
            if let Some(name) = self.selected_qube().map(|q| q.name.clone()) {
                if !self.properties_cache.contains_key(&name) {
                    return vec![SideEffect::FetchProperties(name)];
                }
            }
        }
        vec![]
    }

    fn selected_running_name(&self) -> Option<String> {
        self.selected_qube()
            .filter(|q| q.state == QubeState::Running)
            .map(|q| q.name.clone())
    }

    fn selected_halted_name(&self) -> Option<String> {
        self.selected_qube()
            .filter(|q| q.state == QubeState::Halted)
            .map(|q| q.name.clone())
    }

    fn push_op(&mut self, vm_name: String, kind: OpKind) {
        let op_id = self.next_op_id;
        self.next_op_id += 1;
        self.pending_ops.push(PendingOp {
            op_id,
            vm_name,
            kind,
            started: Instant::now(),
        });
    }

    fn apply_event(&mut self, evt: crate::admin::AdminEvent) {
        match evt.event_type.as_str() {
            "domain-start" => {
                if let Some(q) = self.qubes.iter_mut().find(|q| q.name == evt.subject) {
                    q.state = QubeState::Running;
                }
            }
            "domain-shutdown" | "domain-stopped" => {
                if let Some(q) = self.qubes.iter_mut().find(|q| q.name == evt.subject) {
                    q.state = QubeState::Halted;
                }
            }
            "domain-paused" => {
                if let Some(q) = self.qubes.iter_mut().find(|q| q.name == evt.subject) {
                    q.state = QubeState::Paused;
                }
            }
            _ => {}
        }
    }
}
