use super::protocol::*;
use super::types::*;

// ── Request encoding ──────────────────────────────────────────────────────────

#[test]
fn request_encode_simple() {
    let req = Request { method: "admin.vm.List", destination: "dom0", arg: "", payload: b"" };
    let enc = req.encode();
    assert_eq!(enc, b"admin.vm.List\0dom0\0\0");
}

#[test]
fn request_encode_with_arg() {
    let req = Request {
        method:      "admin.vm.Start",
        destination: "dom0",
        arg:         "personal",
        payload:     b"",
    };
    let enc = req.encode();
    assert_eq!(enc, b"admin.vm.Start\0dom0\0personal\0");
}

#[test]
fn request_encode_with_payload() {
    let req = Request {
        method:      "admin.vm.property.Set",
        destination: "dom0",
        arg:         "personal",
        payload:     b"label\0red",
    };
    let enc = req.encode();
    assert_eq!(enc, b"admin.vm.property.Set\0dom0\0personal\0label\0red");
}

// ── Response decoding ─────────────────────────────────────────────────────────

#[test]
fn response_decode_ok_empty() {
    let raw = b"\x30\x00";
    let resp = Response::decode(raw).unwrap();
    assert!(matches!(resp.rtype, ResponseType::Ok));
    assert!(resp.data.is_empty());
}

#[test]
fn response_decode_ok_with_data() {
    let raw = b"\x30\x00hello";
    let resp = Response::decode(raw).unwrap();
    assert!(matches!(resp.rtype, ResponseType::Ok));
    assert_eq!(resp.data, b"hello");
}

#[test]
fn response_decode_event() {
    let raw = b"\x31\x00somedata";
    let resp = Response::decode(raw).unwrap();
    assert!(matches!(resp.rtype, ResponseType::Event));
    assert_eq!(resp.data, b"somedata");
}

#[test]
fn response_decode_exception_becomes_error() {
    // Exception: type_byte=0x32, then exc_type\0traceback\0message
    let raw = b"\x32\x00QubesVMNotFoundError\x00\x00No such domain: personal";
    let err = Response::decode(raw).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("QubesVMNotFoundError"), "got: {msg}");
    assert!(msg.contains("No such domain"), "got: {msg}");
}

#[test]
fn response_decode_too_short() {
    assert!(Response::decode(b"").is_err());
    assert!(Response::decode(b"\x30").is_err());
}

#[test]
fn response_decode_unknown_type_byte() {
    let raw = b"\xff\x00data";
    assert!(Response::decode(raw).is_err());
}

// ── parse_vm_list ─────────────────────────────────────────────────────────────

#[test]
fn vm_list_basic() {
    let data = b"dom0 class=AdminVM state=Running label=black\n\
                 personal class=AppVM state=Running label=yellow template=fedora-41 netvm=sys-firewall\n";
    let qubes = parse_vm_list(data).unwrap();
    assert_eq!(qubes.len(), 2);

    let dom0 = &qubes[0];
    assert_eq!(dom0.name, "dom0");
    assert!(matches!(dom0.class, QubeClass::AdminVM));
    assert!(matches!(dom0.state, QubeState::Running));
    assert_eq!(dom0.label, "black");
    assert!(dom0.template.is_none());
    assert!(dom0.netvm.is_none());

    let personal = &qubes[1];
    assert_eq!(personal.name, "personal");
    assert!(matches!(personal.class, QubeClass::AppVM));
    assert!(matches!(personal.state, QubeState::Running));
    assert_eq!(personal.label, "yellow");
    assert_eq!(personal.template.as_deref(), Some("fedora-41"));
    assert_eq!(personal.netvm.as_deref(), Some("sys-firewall"));
}

#[test]
fn vm_list_halted_template() {
    let data = b"fedora-41 class=TemplateVM state=Halted label=black\n";
    let qubes = parse_vm_list(data).unwrap();
    assert_eq!(qubes.len(), 1);
    assert!(matches!(qubes[0].class, QubeClass::TemplateVM));
    assert!(matches!(qubes[0].state, QubeState::Halted));
}

#[test]
fn vm_list_netvm_none() {
    let data = b"vault class=AppVM state=Halted label=black template=fedora-41 netvm=None\n";
    let qubes = parse_vm_list(data).unwrap();
    assert!(qubes[0].netvm.is_none(), "netvm=None should parse as None");
}

#[test]
fn vm_list_unknown_class_and_state() {
    let data = b"somevm class=FutureClass state=FutureState label=red\n";
    let qubes = parse_vm_list(data).unwrap();
    assert!(matches!(qubes[0].class, QubeClass::Unknown(_)));
    assert!(matches!(qubes[0].state, QubeState::Unknown(_)));
}

#[test]
fn vm_list_empty_input() {
    let qubes = parse_vm_list(b"").unwrap();
    assert!(qubes.is_empty());
}

#[test]
fn vm_list_blank_lines_ignored() {
    let data = b"\n\ndom0 class=AdminVM state=Running label=black\n\n";
    let qubes = parse_vm_list(data).unwrap();
    assert_eq!(qubes.len(), 1);
}

#[test]
fn vm_list_dispvm() {
    let data = b"disp42 class=DispVM state=Running label=red template=fedora-41-dvm netvm=sys-firewall\n";
    let qubes = parse_vm_list(data).unwrap();
    assert!(matches!(qubes[0].class, QubeClass::DispVM));
}

// ── parse_properties ─────────────────────────────────────────────────────────

#[test]
fn properties_basic() {
    let data = b"memory default=no type=int value=400\n\
                 maxmem default=no type=int value=4000\n\
                 vcpus default=no type=int value=2\n\
                 autostart default=yes type=bool value=False\n\
                 provides_network default=yes type=bool value=False\n\
                 kernel default=yes type=str value=6.1.80-1\n";
    let props = parse_properties(data).unwrap();
    assert_eq!(props.memory,   Some(400));
    assert_eq!(props.maxmem,   Some(4000));
    assert_eq!(props.vcpus,    Some(2));
    assert_eq!(props.autostart, Some(false));
    assert_eq!(props.provides_network, Some(false));
    assert_eq!(props.kernel.as_deref(), Some("6.1.80-1"));
}

#[test]
fn properties_autostart_true() {
    let data = b"autostart default=no type=bool value=True\n";
    let props = parse_properties(data).unwrap();
    assert_eq!(props.autostart, Some(true));
}

#[test]
fn properties_raw_map_populated() {
    let data = b"memory default=no type=int value=512\n\
                 someprop default=no type=str value=hello\n";
    let props = parse_properties(data).unwrap();
    assert_eq!(props.raw.get("memory").map(|s| s.as_str()), Some("512"));
    assert_eq!(props.raw.get("someprop").map(|s| s.as_str()), Some("hello"));
}

#[test]
fn properties_line_without_value_skipped() {
    // Line has no value= token — should be silently skipped
    let data = b"memory default=no type=int\n\
                 vcpus default=no type=int value=4\n";
    let props = parse_properties(data).unwrap();
    assert!(props.memory.is_none());
    assert_eq!(props.vcpus, Some(4));
}

#[test]
fn properties_empty_input() {
    let props = parse_properties(b"").unwrap();
    assert!(props.memory.is_none());
    assert!(props.raw.is_empty());
}

#[test]
fn properties_default_dispvm() {
    let data = b"default_dispvm default=yes type=vm value=fedora-41-dvm\n";
    let props = parse_properties(data).unwrap();
    assert_eq!(props.default_dispvm.as_deref(), Some("fedora-41-dvm"));
}

// ── parse_current_state ───────────────────────────────────────────────────────

#[test]
fn current_state_basic() {
    let data = b"state=Running mem=400 cpu_time=123456789";
    let stats = parse_current_state(data).unwrap();
    assert_eq!(stats.memory_kb, 400);
    assert_eq!(stats.cpu_time,  123456789);
}

#[test]
fn current_state_extra_fields_ignored() {
    let data = b"state=Running mem=800 cpu_time=42 xid=7 some_future_field=foo";
    let stats = parse_current_state(data).unwrap();
    assert_eq!(stats.memory_kb, 800);
    assert_eq!(stats.cpu_time,  42);
}

#[test]
fn current_state_empty_returns_defaults() {
    let stats = parse_current_state(b"").unwrap();
    assert_eq!(stats.memory_kb, 0);
    assert_eq!(stats.cpu_time,  0);
}

#[test]
fn current_state_missing_fields_are_zero() {
    let data = b"state=Running";
    let stats = parse_current_state(data).unwrap();
    assert_eq!(stats.memory_kb, 0);
    assert_eq!(stats.cpu_time,  0);
}

// ── QubeState helpers ─────────────────────────────────────────────────────────

#[test]
fn qube_state_short_labels() {
    assert_eq!(QubeState::Running.short_label(),    "RUN");
    assert_eq!(QubeState::Halted.short_label(),     "OFF");
    assert_eq!(QubeState::Paused.short_label(),     "PAU");
    assert_eq!(QubeState::Transient.short_label(),  "...");
    assert_eq!(QubeState::Unknown("X".into()).short_label(), "???");
}

#[test]
fn qube_state_roundtrip() {
    for s in &["Running", "Halted", "Paused", "Transient"] {
        let state = QubeState::from_str(s);
        assert!(!matches!(state, QubeState::Unknown(_)), "failed for {s}");
    }
    assert!(matches!(QubeState::from_str("Bogus"), QubeState::Unknown(_)));
}

#[test]
fn qube_class_roundtrip() {
    for c in &["AppVM", "TemplateVM", "StandaloneVM", "DispVM", "AdminVM"] {
        let class = QubeClass::from_str(c);
        assert!(!matches!(class, QubeClass::Unknown(_)), "failed for {c}");
    }
    assert!(matches!(QubeClass::from_str("Bogus"), QubeClass::Unknown(_)));
}
