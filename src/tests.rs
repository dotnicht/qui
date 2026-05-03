use std::sync::Arc;
use crate::admin::{AdminClient, QubeInfo, QubeProperties};
use crate::admin::types::{QubeClass, QubeState};

use crate::action::{Action, SideEffect};
use crate::app::{ActiveView, App, MessageLevel, Modal};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_app() -> App {
    // Point at a nonexistent socket so the client falls back to CLI.
    // update() is pure — it never touches the socket; only spawned threads do.
    App::new(Arc::new(AdminClient::with_socket("/nonexistent/qubesd.sock")))
}

fn appvm(name: &str, state: QubeState) -> QubeInfo {
    QubeInfo {
        name:     name.into(),
        class:    QubeClass::AppVM,
        state,
        label:    "red".into(),
        template: Some("fedora-41".into()),
        netvm:    Some("sys-firewall".into()),
    }
}

fn load_qubes(app: &mut App, qubes: Vec<QubeInfo>) {
    app.update(Action::QubeListLoaded(qubes));
}

// ── Navigation ────────────────────────────────────────────────────────────────

#[test]
fn move_down_increments_index() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("a", QubeState::Halted), appvm("b", QubeState::Halted)]);
    assert_eq!(app.selected_index, 0);
    app.update(Action::MoveDown);
    assert_eq!(app.selected_index, 1);
}

#[test]
fn move_down_does_not_exceed_list() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("a", QubeState::Halted)]);
    app.update(Action::MoveDown);
    assert_eq!(app.selected_index, 0);
}

#[test]
fn move_up_decrements_index() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("a", QubeState::Halted), appvm("b", QubeState::Halted)]);
    app.update(Action::MoveDown);
    app.update(Action::MoveUp);
    assert_eq!(app.selected_index, 0);
}

#[test]
fn move_up_does_not_go_below_zero() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("a", QubeState::Halted)]);
    app.update(Action::MoveUp);
    assert_eq!(app.selected_index, 0);
}

#[test]
fn move_top_jumps_to_zero() {
    let mut app = make_app();
    load_qubes(&mut app, vec![
        appvm("a", QubeState::Halted),
        appvm("b", QubeState::Halted),
        appvm("c", QubeState::Halted),
    ]);
    app.update(Action::MoveBottom);
    assert_eq!(app.selected_index, 2);
    app.update(Action::MoveTop);
    assert_eq!(app.selected_index, 0);
}

#[test]
fn move_bottom_jumps_to_last() {
    let mut app = make_app();
    load_qubes(&mut app, vec![
        appvm("a", QubeState::Halted),
        appvm("b", QubeState::Halted),
        appvm("c", QubeState::Halted),
    ]);
    app.update(Action::MoveBottom);
    assert_eq!(app.selected_index, 2);
}

// ── Selection clamping on list reload ────────────────────────────────────────

#[test]
fn selection_clamped_when_list_shrinks() {
    let mut app = make_app();
    load_qubes(&mut app, vec![
        appvm("a", QubeState::Halted),
        appvm("b", QubeState::Halted),
        appvm("c", QubeState::Halted),
    ]);
    app.update(Action::MoveBottom); // index = 2
    // Reload with only 1 VM
    load_qubes(&mut app, vec![appvm("a", QubeState::Halted)]);
    assert_eq!(app.selected_index, 0);
}

// ── VM operations — side effects ─────────────────────────────────────────────

#[test]
fn start_halted_vm_returns_side_effect() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("personal", QubeState::Halted)]);
    let effects = app.update(Action::StartSelected);
    assert!(effects.iter().any(|e| matches!(e, SideEffect::StartVm(n) if n == "personal")));
}

#[test]
fn start_running_vm_returns_no_effect() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("personal", QubeState::Running)]);
    let effects = app.update(Action::StartSelected);
    assert!(effects.is_empty());
}

#[test]
fn shutdown_running_vm_returns_side_effect() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("personal", QubeState::Running)]);
    let effects = app.update(Action::ShutdownSelected);
    assert!(effects.iter().any(|e| matches!(e, SideEffect::ShutdownVm(n) if n == "personal")));
}

#[test]
fn shutdown_halted_vm_returns_no_effect() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("personal", QubeState::Halted)]);
    let effects = app.update(Action::ShutdownSelected);
    assert!(effects.is_empty());
}

#[test]
fn delete_halted_vm_opens_confirm_modal() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("vault", QubeState::Halted)]);
    app.update(Action::DeleteSelected);
    assert!(matches!(app.modal, Modal::ConfirmDelete { ref vm_name } if vm_name == "vault"));
}

#[test]
fn delete_running_vm_shows_warning_not_modal() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("vault", QubeState::Running)]);
    app.update(Action::DeleteSelected);
    assert!(matches!(app.modal, Modal::None));
    assert!(app.status.as_ref().map(|s| matches!(s.level, MessageLevel::Warning)).unwrap_or(false));
}

#[test]
fn kill_running_vm_opens_confirm_modal() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("work", QubeState::Running)]);
    app.update(Action::KillSelected);
    assert!(matches!(app.modal, Modal::ConfirmKill { ref vm_name } if vm_name == "work"));
}

// ── Confirm / Cancel modals ───────────────────────────────────────────────────

#[test]
fn confirm_delete_dispatches_delete_side_effect() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("vault", QubeState::Halted)]);
    app.update(Action::DeleteSelected); // opens modal
    let effects = app.update(Action::Confirm);
    assert!(effects.iter().any(|e| matches!(e, SideEffect::DeleteVm(n) if n == "vault")));
    assert!(matches!(app.modal, Modal::None));
}

#[test]
fn cancel_delete_closes_modal_no_effect() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("vault", QubeState::Halted)]);
    app.update(Action::DeleteSelected);
    let effects = app.update(Action::Cancel);
    assert!(effects.is_empty());
    assert!(matches!(app.modal, Modal::None));
}

#[test]
fn confirm_kill_dispatches_kill_side_effect() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("work", QubeState::Running)]);
    app.update(Action::KillSelected);
    let effects = app.update(Action::Confirm);
    assert!(effects.iter().any(|e| matches!(e, SideEffect::KillVm(n) if n == "work")));
    assert!(matches!(app.modal, Modal::None));
}

// ── Detail popup ─────────────────────────────────────────────────────────────

#[test]
fn toggle_detail_opens_and_closes() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("personal", QubeState::Running)]);
    app.update(Action::ToggleDetail);
    assert!(matches!(app.modal, Modal::Detail));
    app.update(Action::ToggleDetail);
    assert!(matches!(app.modal, Modal::None));
}

#[test]
fn toggle_detail_on_empty_list_does_nothing() {
    let mut app = make_app();
    app.update(Action::ToggleDetail);
    assert!(matches!(app.modal, Modal::None));
}

#[test]
fn toggle_detail_returns_fetch_properties_when_not_cached() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("personal", QubeState::Running)]);
    let effects = app.update(Action::ToggleDetail);
    assert!(effects.iter().any(|e| matches!(e, SideEffect::FetchProperties(n) if n == "personal")));
}

#[test]
fn toggle_detail_no_fetch_when_already_cached() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("personal", QubeState::Running)]);
    app.properties_cache.insert("personal".into(), QubeProperties::default());
    let effects = app.update(Action::ToggleDetail);
    assert!(!effects.iter().any(|e| matches!(e, SideEffect::FetchProperties(_))));
}

#[test]
fn esc_closes_detail_modal() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("personal", QubeState::Running)]);
    app.update(Action::ToggleDetail);
    app.update(Action::Cancel);
    assert!(matches!(app.modal, Modal::None));
}

// ── Help modal ────────────────────────────────────────────────────────────────

#[test]
fn help_opens_and_closes_with_cancel() {
    let mut app = make_app();
    app.update(Action::ShowHelp);
    assert!(matches!(app.modal, Modal::Help));
    app.update(Action::Cancel);
    assert!(matches!(app.modal, Modal::None));
}

// ── Edit property flow ────────────────────────────────────────────────────────

fn open_edit(app: &mut App, prop: &str) {
    app.modal = Modal::EditProperty {
        vm_name:  "personal".into(),
        property: prop.into(),
        input:    "old".into(),
    };
}

#[test]
fn edit_char_appends_to_input() {
    let mut app = make_app();
    open_edit(&mut app, "label");
    app.update(Action::EditChar('r'));
    app.update(Action::EditChar('e'));
    app.update(Action::EditChar('d'));
    assert!(matches!(&app.modal,
        Modal::EditProperty { input, .. } if input == "oldred"
    ));
}

#[test]
fn edit_backspace_removes_last_char() {
    let mut app = make_app();
    open_edit(&mut app, "label");
    app.update(Action::EditChar('x'));
    app.update(Action::EditBackspace);
    assert!(matches!(&app.modal,
        Modal::EditProperty { input, .. } if input == "old"
    ));
}

#[test]
fn edit_backspace_on_empty_does_not_panic() {
    let mut app = make_app();
    app.modal = Modal::EditProperty {
        vm_name:  "personal".into(),
        property: "label".into(),
        input:    "".into(),
    };
    app.update(Action::EditBackspace); // should not panic
    assert!(matches!(&app.modal, Modal::EditProperty { input, .. } if input.is_empty()));
}

#[test]
fn edit_submit_returns_set_property_and_fetch_effects() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("personal", QubeState::Running)]);
    app.modal = Modal::EditProperty {
        vm_name:  "personal".into(),
        property: "label".into(),
        input:    "green".into(),
    };
    let effects = app.update(Action::EditSubmit);
    assert!(effects.iter().any(|e| matches!(e,
        SideEffect::SetProperty { vm, property, value }
        if vm == "personal" && property == "label" && value == "green"
    )));
    assert!(effects.iter().any(|e| matches!(e, SideEffect::FetchProperties(n) if n == "personal")));
}

#[test]
fn edit_submit_invalidates_properties_cache() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("personal", QubeState::Running)]);
    app.properties_cache.insert("personal".into(), QubeProperties::default());
    app.modal = Modal::EditProperty {
        vm_name:  "personal".into(),
        property: "label".into(),
        input:    "blue".into(),
    };
    app.update(Action::EditSubmit);
    assert!(!app.properties_cache.contains_key("personal"));
}

#[test]
fn edit_submit_returns_to_detail_modal() {
    let mut app = make_app();
    load_qubes(&mut app, vec![appvm("personal", QubeState::Running)]);
    open_edit(&mut app, "label");
    app.update(Action::EditSubmit);
    assert!(matches!(app.modal, Modal::Detail));
}

#[test]
fn edit_cancel_returns_to_detail_without_side_effects() {
    let mut app = make_app();
    open_edit(&mut app, "label");
    let effects = app.update(Action::Cancel);
    assert!(effects.is_empty());
    assert!(matches!(app.modal, Modal::Detail));
}

// ── Tab switching ─────────────────────────────────────────────────────────────

#[test]
fn switch_to_template_manager_filters_appvms() {
    let mut app = make_app();
    load_qubes(&mut app, vec![
        appvm("personal", QubeState::Halted),
        QubeInfo {
            name:     "fedora-41".into(),
            class:    QubeClass::TemplateVM,
            state:    QubeState::Halted,
            label:    "black".into(),
            template: None,
            netvm:    None,
        },
    ]);
    app.update(Action::SwitchToTemplateManager);
    assert_eq!(app.active_view, ActiveView::TemplateManager);
    assert_eq!(app.filtered_indices.len(), 1);
    let idx = app.filtered_indices[0];
    assert_eq!(app.qubes[idx].name, "fedora-41");
}

#[test]
fn switch_back_to_qube_manager_shows_all() {
    let mut app = make_app();
    load_qubes(&mut app, vec![
        appvm("personal", QubeState::Halted),
        QubeInfo {
            name:     "fedora-41".into(),
            class:    QubeClass::TemplateVM,
            state:    QubeState::Halted,
            label:    "black".into(),
            template: None,
            netvm:    None,
        },
    ]);
    app.update(Action::SwitchToTemplateManager);
    app.update(Action::SwitchToQubeManager);
    assert_eq!(app.active_view, ActiveView::QubeManager);
    assert_eq!(app.filtered_indices.len(), 2);
}

// ── Quit ─────────────────────────────────────────────────────────────────────

#[test]
fn quit_sets_should_quit() {
    let mut app = make_app();
    app.update(Action::Quit);
    assert!(app.should_quit);
}

#[test]
fn quit_inside_help_modal_still_quits() {
    let mut app = make_app();
    app.update(Action::ShowHelp);
    app.update(Action::Quit);
    assert!(app.should_quit);
}

// ── Operation result handling ─────────────────────────────────────────────────

#[test]
fn operation_completed_triggers_list_refresh() {
    let mut app = make_app();
    // Inject a fake pending op so the completed message can be matched
    use crate::action::OpKind;
    use crate::app::PendingOp;
    app.pending_ops.push(PendingOp {
        op_id:   42,
        vm_name: "personal".into(),
        kind:    OpKind::Start,
        started: std::time::Instant::now(),
    });
    let effects = app.update(Action::OperationCompleted { op_id: 42 });
    assert!(effects.iter().any(|e| matches!(e, SideEffect::FetchQubeList)));
    assert!(app.pending_ops.is_empty());
    assert!(matches!(app.status.as_ref().map(|s| &s.level), Some(MessageLevel::Success)));
}

#[test]
fn operation_failed_sets_error_status() {
    let mut app = make_app();
    use crate::action::OpKind;
    use crate::app::PendingOp;
    app.pending_ops.push(PendingOp {
        op_id:   7,
        vm_name: "work".into(),
        kind:    OpKind::Shutdown,
        started: std::time::Instant::now(),
    });
    app.update(Action::OperationFailed { op_id: 7, error: "timed out".into() });
    assert!(matches!(app.status.as_ref().map(|s| &s.level), Some(MessageLevel::Error)));
}
