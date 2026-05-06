use super::error::{AdminError, AdminResult};
use super::types::{QubeClass, QubeInfo, QubeProperties, QubeState};

// ── Request ──────────────────────────────────────────────────────────────────

pub struct Request<'a> {
    pub method: &'a str,
    pub destination: &'a str,
    pub arg: &'a str,
    pub payload: &'a [u8],
}

impl<'a> Request<'a> {
    /// Encode as: `method\0destination\0arg\0payload`
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(
            self.method.len()
                + 1
                + self.destination.len()
                + 1
                + self.arg.len()
                + 1
                + self.payload.len(),
        );
        buf.extend_from_slice(self.method.as_bytes());
        buf.push(0);
        buf.extend_from_slice(self.destination.as_bytes());
        buf.push(0);
        buf.extend_from_slice(self.arg.as_bytes());
        buf.push(0);
        buf.extend_from_slice(self.payload);
        buf
    }
}

// ── Response ─────────────────────────────────────────────────────────────────

pub const RESP_OK: u8 = 0x30;
pub const RESP_EVENT: u8 = 0x31;
pub const RESP_EXCEPTION: u8 = 0x32;

#[derive(Debug)]
pub struct Response {
    pub data: Vec<u8>,
}

impl Response {
    /// Parse raw bytes. Format: `type_byte \0 data...`
    pub fn decode(raw: &[u8]) -> AdminResult<Self> {
        if raw.len() < 2 {
            return Err(AdminError::Protocol(format!(
                "response too short ({} bytes)",
                raw.len()
            )));
        }
        let type_byte = raw[0];
        // raw[1] should be 0x00 separator
        let data = raw[2..].to_vec();

        match type_byte {
            RESP_OK | RESP_EVENT => Ok(Response { data }),
            RESP_EXCEPTION => Err(parse_exception(&data)),
            b => Err(AdminError::Protocol(format!(
                "unknown response type byte 0x{:02x}",
                b
            ))),
        }
    }
}

fn parse_exception(data: &[u8]) -> AdminError {
    // Format: exc_type \0 [traceback \0] message
    let parts: Vec<&[u8]> = data.splitn(3, |&b| b == 0).collect();
    let exc_type = String::from_utf8_lossy(parts.first().copied().unwrap_or(b"")).into_owned();
    let message = String::from_utf8_lossy(parts.last().copied().unwrap_or(b"")).into_owned();
    AdminError::QubesDException { exc_type, message }
}

// ── admin.vm.List parser ──────────────────────────────────────────────────────
//
// Response body format (one VM per line):
//   name class=AppVM state=Running label=red template=fedora-41 netvm=sys-firewall\n
//   dom0 class=AdminVM state=Running label=black\n

pub fn parse_vm_list(data: &[u8]) -> AdminResult<Vec<QubeInfo>> {
    let text = std::str::from_utf8(data).map_err(|e| AdminError::Parse(e.to_string()))?;
    let mut qubes = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        qubes.push(parse_vm_list_line(line)?);
    }
    Ok(qubes)
}

fn parse_vm_list_line(line: &str) -> AdminResult<QubeInfo> {
    let mut parts = line.splitn(2, ' ');
    let name = parts
        .next()
        .ok_or_else(|| AdminError::Parse(format!("empty vm list line: {line:?}")))?
        .to_string();

    let mut class = QubeClass::Unknown(String::new());
    let mut state = QubeState::Unknown(String::new());
    let mut label = String::new();
    let mut template = None;
    let mut netvm = None;

    if let Some(rest) = parts.next() {
        for kv in rest.split_whitespace() {
            if let Some((k, v)) = kv.split_once('=') {
                match k {
                    "class" => class = QubeClass::from_str(v),
                    "state" => state = QubeState::from_str(v),
                    "label" => label = v.to_string(),
                    "template" => template = Some(v.to_string()),
                    "netvm" => {
                        netvm = if v == "None" {
                            None
                        } else {
                            Some(v.to_string())
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(QubeInfo {
        name,
        class,
        state,
        label,
        template,
        netvm,
    })
}

// ── admin.vm.property.GetAll parser ──────────────────────────────────────────
//
// Format (one property per line):
//   propname default=yes type=bool value=False\n

pub fn parse_properties(data: &[u8]) -> AdminResult<QubeProperties> {
    let text = std::str::from_utf8(data).map_err(|e| AdminError::Parse(e.to_string()))?;

    let mut props = QubeProperties::default();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let mut iter = line.splitn(2, ' ');
        let name = iter.next().unwrap_or("").to_string();
        let rest = iter.next().unwrap_or("");

        let mut value: Option<&str> = None;
        for kv in rest.split_whitespace() {
            if let Some(v) = kv.strip_prefix("value=") {
                value = Some(v);
            }
        }
        let Some(v) = value else { continue };

        props.raw.insert(name.clone(), v.to_string());

        match name.as_str() {
            "memory" => props.memory = v.parse().ok(),
            "maxmem" => props.maxmem = v.parse().ok(),
            "vcpus" => props.vcpus = v.parse().ok(),
            "autostart" => props.autostart = parse_bool(v),
            "provides_network" => props.provides_network = parse_bool(v),
            "kernel" => props.kernel = Some(v.to_string()),
            "default_dispvm" => props.default_dispvm = Some(v.to_string()),
            _ => {}
        }
    }

    Ok(props)
}

fn parse_bool(s: &str) -> Option<bool> {
    match s {
        "True" | "true" | "1" | "yes" => Some(true),
        "False" | "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Request::encode ───────────────────────────────────────────────────────

    #[test]
    fn encode_joins_fields_with_nul() {
        let r = Request { method: "admin.vm.List", destination: "dom0", arg: "", payload: b"" };
        let enc = r.encode();
        assert_eq!(enc, b"admin.vm.List\x00dom0\x00\x00");
    }

    #[test]
    fn encode_includes_payload() {
        let r = Request { method: "m", destination: "d", arg: "a", payload: b"data" };
        let enc = r.encode();
        assert!(enc.ends_with(b"data"));
    }

    // ── Response::decode ──────────────────────────────────────────────────────

    #[test]
    fn decode_ok_response() {
        let raw = b"\x30\x00hello";
        let resp = Response::decode(raw).unwrap();
        assert_eq!(resp.data, b"hello");
    }

    #[test]
    fn decode_event_response() {
        let raw = b"\x31\x00payload";
        let resp = Response::decode(raw).unwrap();
        assert_eq!(resp.data, b"payload");
    }

    #[test]
    fn decode_exception_returns_err() {
        let raw = b"\x32\x00QubesException\x00traceback\x00something went wrong";
        let err = Response::decode(raw).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("QubesException"));
        assert!(msg.contains("something went wrong"));
    }

    #[test]
    fn decode_too_short_returns_err() {
        let err = Response::decode(b"\x30").unwrap_err();
        assert!(err.to_string().contains("too short"));
    }

    #[test]
    fn decode_unknown_type_byte_returns_err() {
        let raw = b"\xff\x00data";
        let err = Response::decode(raw).unwrap_err();
        assert!(err.to_string().contains("unknown response type"));
    }

    // ── parse_vm_list ─────────────────────────────────────────────────────────

    #[test]
    fn parse_typical_vm_list() {
        let data = b"personal class=AppVM state=Running label=red template=fedora-41 netvm=sys-firewall\n\
                     dom0 class=AdminVM state=Running label=black\n\
                     fedora-41 class=TemplateVM state=Halted label=black\n";
        let qubes = parse_vm_list(data).unwrap();
        assert_eq!(qubes.len(), 3);

        let personal = &qubes[0];
        assert_eq!(personal.name, "personal");
        assert_eq!(personal.class, QubeClass::AppVM);
        assert_eq!(personal.state, QubeState::Running);
        assert_eq!(personal.label, "red");
        assert_eq!(personal.template.as_deref(), Some("fedora-41"));
        assert_eq!(personal.netvm.as_deref(), Some("sys-firewall"));

        let dom0 = &qubes[1];
        assert_eq!(dom0.class, QubeClass::AdminVM);
        assert!(dom0.netvm.is_none());
    }

    #[test]
    fn parse_vm_list_netvm_none_becomes_option_none() {
        let data = b"vault class=AppVM state=Halted label=black netvm=None\n";
        let qubes = parse_vm_list(data).unwrap();
        assert!(qubes[0].netvm.is_none());
    }

    #[test]
    fn parse_vm_list_skips_blank_lines() {
        let data = b"\npersonal class=AppVM state=Halted label=red\n\n";
        let qubes = parse_vm_list(data).unwrap();
        assert_eq!(qubes.len(), 1);
    }

    #[test]
    fn parse_vm_list_invalid_utf8_returns_err() {
        let data = b"\xff\xfe";
        assert!(parse_vm_list(data).is_err());
    }

    // ── parse_properties ─────────────────────────────────────────────────────

    #[test]
    fn parse_properties_numeric_and_bool() {
        let data = b"memory default=no type=int value=4000\n\
                     maxmem default=no type=int value=8000\n\
                     vcpus default=no type=int value=2\n\
                     autostart default=no type=bool value=False\n\
                     provides_network default=no type=bool value=True\n\
                     kernel default=no type=str value=5.15.0\n\
                     default_dispvm default=no type=str value=fedora-dvm\n";
        let props = parse_properties(data).unwrap();
        assert_eq!(props.memory, Some(4000));
        assert_eq!(props.maxmem, Some(8000));
        assert_eq!(props.vcpus, Some(2));
        assert_eq!(props.autostart, Some(false));
        assert_eq!(props.provides_network, Some(true));
        assert_eq!(props.kernel.as_deref(), Some("5.15.0"));
        assert_eq!(props.default_dispvm.as_deref(), Some("fedora-dvm"));
    }

    #[test]
    fn parse_bool_all_truthy_variants() {
        for v in ["True", "true", "1", "yes"] {
            let data = format!("autostart default=no type=bool value={v}\n");
            let props = parse_properties(data.as_bytes()).unwrap();
            assert_eq!(props.autostart, Some(true), "expected true for value={v}");
        }
    }

    #[test]
    fn parse_bool_all_falsy_variants() {
        for v in ["False", "false", "0", "no"] {
            let data = format!("autostart default=no type=bool value={v}\n");
            let props = parse_properties(data.as_bytes()).unwrap();
            assert_eq!(props.autostart, Some(false), "expected false for value={v}");
        }
    }

    #[test]
    fn parse_properties_skips_lines_without_value() {
        let data = b"memory default=yes type=int\n";
        let props = parse_properties(data).unwrap();
        assert!(props.memory.is_none());
        assert!(props.raw.is_empty());
    }

    #[test]
    fn parse_properties_raw_map_populated() {
        let data = b"memory default=no type=int value=2048\n";
        let props = parse_properties(data).unwrap();
        assert_eq!(props.raw.get("memory").map(|s| s.as_str()), Some("2048"));
    }
}
