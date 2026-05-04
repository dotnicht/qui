use crate::action::{Action, SideEffect};
use crate::admin::types::{QubeClass, QubeInfo, QubeState};
use crate::admin::{AdminClient, QubeProperties};
use crate::app::{ActiveView, App};
use std::sync::Arc;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_app() -> App {
    App::new(Arc::new(AdminClient::new()))
}

fn qube(name: &str, class: QubeClass, state: QubeState) -> QubeInfo {
    QubeInfo {
        name: name.into(),
        class,
        state,
        label: "red".into(),
        template: Some("fedora-41".into()),
        netvm: Some("sys-firewall".into()),
    }
}

fn appvm(name: &str) -> QubeInfo {
    qube(name, QubeClass::AppVM, QubeState::Halted)
}
fn sysvm(name: &str) -> QubeInfo {
    qube(name, QubeClass::AppVM, QubeState::Running)
}
fn templatevm(name: &str) -> QubeInfo {
    qube(name, QubeClass::TemplateVM, QubeState::Halted)
}
fn adminvm(name: &str) -> QubeInfo {
    qube(name, QubeClass::AdminVM, QubeState::Running)
}

fn load(app: &mut App, qubes: Vec<QubeInfo>) {
    app.update(Action::QubeListLoaded(qubes));
}

fn names(app: &App) -> Vec<String> {
    app.filtered_indices
        .iter()
        .map(|&i| app.qubes[i].name.clone())
        .collect()
}

// ── Tab filter: Qubes ─────────────────────────────────────────────────────────

#[test]
fn qubes_tab_shows_plain_appvm() {
    let mut app = make_app();
    load(&mut app, vec![appvm("personal")]);
    assert!(names(&app).contains(&"personal".to_string()));
}

#[test]
fn qubes_tab_excludes_sys_prefix() {
    let mut app = make_app();
    load(&mut app, vec![appvm("personal"), sysvm("sys-net")]);
    let n = names(&app);
    assert!(n.contains(&"personal".to_string()));
    assert!(!n.contains(&"sys-net".to_string()));
}

#[test]
fn qubes_tab_excludes_adminvm() {
    let mut app = make_app();
    load(&mut app, vec![appvm("personal"), adminvm("dom0")]);
    let n = names(&app);
    assert!(!n.contains(&"dom0".to_string()));
}

#[test]
fn qubes_tab_excludes_templatevm() {
    let mut app = make_app();
    load(&mut app, vec![appvm("personal"), templatevm("fedora-41")]);
    let n = names(&app);
    assert!(!n.contains(&"fedora-41".to_string()));
}

#[test]
fn qubes_tab_excludes_whonix_name() {
    let mut app = make_app();
    load(&mut app, vec![appvm("personal"), appvm("anon-whonix")]);
    let n = names(&app);
    assert!(!n.contains(&"anon-whonix".to_string()));
}

#[test]
fn qubes_tab_includes_standalone_and_dispvm() {
    let mut app = make_app();
    load(
        &mut app,
        vec![
            qube(
                "standalone-vault",
                QubeClass::StandaloneVM,
                QubeState::Halted,
            ),
            qube("disp42", QubeClass::DispVM, QubeState::Running),
        ],
    );
    let n = names(&app);
    assert!(n.contains(&"standalone-vault".to_string()));
    assert!(n.contains(&"disp42".to_string()));
}

// ── Tab filter: Services ──────────────────────────────────────────────────────

#[test]
fn services_tab_includes_sys_prefix() {
    let mut app = make_app();
    load(
        &mut app,
        vec![sysvm("sys-net"), sysvm("sys-firewall"), sysvm("sys-usb")],
    );
    app.update(Action::SwitchToServiceManager);
    let n = names(&app);
    assert!(n.contains(&"sys-net".to_string()));
    assert!(n.contains(&"sys-firewall".to_string()));
    assert!(n.contains(&"sys-usb".to_string()));
}

#[test]
fn services_tab_includes_adminvm() {
    let mut app = make_app();
    load(&mut app, vec![adminvm("dom0"), appvm("personal")]);
    app.update(Action::SwitchToServiceManager);
    let n = names(&app);
    assert!(n.contains(&"dom0".to_string()));
    assert!(!n.contains(&"personal".to_string()));
}

#[test]
fn services_tab_excludes_plain_appvm() {
    let mut app = make_app();
    load(&mut app, vec![appvm("personal"), sysvm("sys-net")]);
    app.update(Action::SwitchToServiceManager);
    assert!(!names(&app).contains(&"personal".to_string()));
}

// sys-whonix: matches both sys- prefix and whonix name — should appear in Services
#[test]
fn sys_whonix_appears_in_services() {
    let mut app = make_app();
    load(&mut app, vec![sysvm("sys-whonix")]);
    app.update(Action::SwitchToServiceManager);
    assert!(names(&app).contains(&"sys-whonix".to_string()));
}

// ── Tab filter: Templates ─────────────────────────────────────────────────────

#[test]
fn templates_tab_shows_only_templatevms() {
    let mut app = make_app();
    load(
        &mut app,
        vec![
            templatevm("fedora-41"),
            templatevm("debian-12"),
            appvm("personal"),
        ],
    );
    app.update(Action::SwitchToTemplateManager);
    let n = names(&app);
    assert!(n.contains(&"fedora-41".to_string()));
    assert!(n.contains(&"debian-12".to_string()));
    assert!(!n.contains(&"personal".to_string()));
}

// ── Tab filter: Whonix ────────────────────────────────────────────────────────

#[test]
fn whonix_tab_matches_name_containing_whonix() {
    let mut app = make_app();
    load(
        &mut app,
        vec![
            appvm("anon-whonix"),
            appvm("whonix-gw-17"),
            sysvm("sys-whonix"),
            appvm("personal"),
        ],
    );
    app.update(Action::SwitchToWhonixManager);
    let n = names(&app);
    assert!(n.contains(&"anon-whonix".to_string()));
    assert!(n.contains(&"whonix-gw-17".to_string()));
    assert!(n.contains(&"sys-whonix".to_string()));
    assert!(!n.contains(&"personal".to_string()));
}

#[test]
fn whonix_tab_match_is_case_insensitive() {
    let mut app = make_app();
    load(&mut app, vec![appvm("Whonix-WS")]);
    app.update(Action::SwitchToWhonixManager);
    assert!(names(&app).contains(&"Whonix-WS".to_string()));
}

// ── Tab switching — actions ───────────────────────────────────────────────────

#[test]
fn switch_to_service_manager_sets_active_view() {
    let mut app = make_app();
    app.update(Action::SwitchToServiceManager);
    assert_eq!(app.active_view, ActiveView::ServiceManager);
}

#[test]
fn switch_to_whonix_manager_sets_active_view() {
    let mut app = make_app();
    app.update(Action::SwitchToWhonixManager);
    assert_eq!(app.active_view, ActiveView::WhonixManager);
}

#[test]
fn switch_resets_selected_index_to_zero() {
    let mut app = make_app();
    load(
        &mut app,
        vec![sysvm("sys-net"), sysvm("sys-firewall"), sysvm("sys-usb")],
    );
    app.update(Action::SwitchToServiceManager);
    app.update(Action::MoveBottom); // index = 2
    app.update(Action::SwitchToQubeManager);
    assert_eq!(app.selected_index, 0);
}

#[test]
fn switch_to_empty_tab_does_not_panic() {
    let mut app = make_app();
    // Only plain appvms — Services tab will be empty
    load(&mut app, vec![appvm("personal"), appvm("work")]);
    app.update(Action::SwitchToServiceManager);
    assert!(app.filtered_indices.is_empty());
    // Navigation on empty list must not panic
    app.update(Action::MoveDown);
    app.update(Action::MoveUp);
    app.update(Action::MoveBottom);
}

#[test]
fn switch_clamps_selection_when_new_tab_is_smaller() {
    let mut app = make_app();
    load(
        &mut app,
        vec![
            appvm("a"),
            appvm("b"),
            appvm("c"),       // 3 in Qubes
            sysvm("sys-net"), // 1 in Services
        ],
    );
    app.update(Action::MoveBottom); // select index 2 in Qubes (3 items)
    app.update(Action::SwitchToServiceManager); // Services has 1 item
    assert_eq!(app.selected_index, 0);
}

// ── Mixed list: all four tabs show correct counts ─────────────────────────────

#[test]
fn all_four_tabs_partition_correctly() {
    let mut app = make_app();
    load(
        &mut app,
        vec![
            appvm("personal"),       // Qubes
            appvm("work"),           // Qubes
            sysvm("sys-net"),        // Services
            adminvm("dom0"),         // Services
            templatevm("fedora-41"), // Templates
            appvm("anon-whonix"),    // Whonix
            sysvm("sys-whonix"),     // Services + Whonix
        ],
    );

    // Qubes: personal, work (sys-net, dom0, fedora-41, anon-whonix, sys-whonix excluded)
    assert_eq!(names(&app), vec!["personal", "work"]);

    app.update(Action::SwitchToServiceManager);
    let svc = names(&app);
    assert!(svc.contains(&"sys-net".to_string()));
    assert!(svc.contains(&"dom0".to_string()));
    assert!(svc.contains(&"sys-whonix".to_string()));
    assert_eq!(svc.len(), 3);

    app.update(Action::SwitchToTemplateManager);
    assert_eq!(names(&app), vec!["fedora-41"]);

    app.update(Action::SwitchToWhonixManager);
    let whx = names(&app);
    assert!(whx.contains(&"anon-whonix".to_string()));
    assert!(whx.contains(&"sys-whonix".to_string()));
    assert_eq!(whx.len(), 2);
}

// ── Properties cache interaction ──────────────────────────────────────────────

#[test]
fn toggle_detail_fetches_properties_when_not_cached() {
    let mut app = make_app();
    load(&mut app, vec![appvm("personal")]);
    let effects = app.update(Action::ToggleDetail);
    assert!(effects
        .iter()
        .any(|e| matches!(e, SideEffect::FetchProperties(n) if n == "personal")));
}

#[test]
fn toggle_detail_no_fetch_when_cached() {
    let mut app = make_app();
    load(&mut app, vec![appvm("personal")]);
    app.properties_cache
        .insert("personal".into(), QubeProperties::default());
    let effects = app.update(Action::ToggleDetail);
    assert!(!effects
        .iter()
        .any(|e| matches!(e, SideEffect::FetchProperties(_))));
}
