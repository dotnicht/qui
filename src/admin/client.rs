use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::error::{AdminError, AdminResult};
use super::protocol::{Request, Response, parse_vm_list, parse_properties, parse_current_state};
use super::types::{QubeInfo, QubeProperties, QubeStats};

const SOCKET_PATH: &str = "/var/run/qubes/qubesd.sock";
const TIMEOUT: Duration = Duration::from_secs(10);

/// Access mode chosen at construction time.
#[derive(Debug, Clone)]
enum AccessMode {
    Socket(PathBuf),
    Cli,
}

#[derive(Debug, Clone)]
pub struct AdminClient {
    mode: AccessMode,
}

impl AdminClient {
    /// Auto-detect: try the qubesd socket; fall back to CLI tools.
    pub fn new() -> Self {
        let path = PathBuf::from(SOCKET_PATH);
        if path.exists() {
            Self { mode: AccessMode::Socket(path) }
        } else {
            Self { mode: AccessMode::Cli }
        }
    }

    /// Force a specific socket path (useful for testing / non-standard installs).
    pub fn with_socket(path: impl Into<PathBuf>) -> Self {
        Self { mode: AccessMode::Socket(path.into()) }
    }

    /// Force CLI fallback (when running without qubesd access).
    pub fn cli_only() -> Self {
        Self { mode: AccessMode::Cli }
    }

    // ── Public API ────────────────────────────────────────────────────────────

    pub fn list_qubes(&self) -> AdminResult<Vec<QubeInfo>> {
        match &self.mode {
            AccessMode::Socket(p) => {
                let req = Request { method: "admin.vm.List", destination: "dom0",
                                    arg: "", payload: b"" };
                let resp = self.socket_call(p, &req)?;
                parse_vm_list(&resp.data)
            }
            AccessMode::Cli => cli_list_qubes(),
        }
    }

    pub fn get_properties(&self, name: &str) -> AdminResult<QubeProperties> {
        match &self.mode {
            AccessMode::Socket(p) => {
                let req = Request { method: "admin.vm.property.GetAll", destination: "dom0",
                                    arg: name, payload: b"" };
                let resp = self.socket_call(p, &req)?;
                parse_properties(&resp.data)
            }
            AccessMode::Cli => cli_get_properties(name),
        }
    }

    pub fn get_stats(&self, name: &str) -> AdminResult<QubeStats> {
        match &self.mode {
            AccessMode::Socket(p) => {
                let req = Request { method: "admin.vm.CurrentState", destination: "dom0",
                                    arg: name, payload: b"" };
                let resp = self.socket_call(p, &req)?;
                parse_current_state(&resp.data)
            }
            AccessMode::Cli => Ok(QubeStats::default()), // not available via CLI
        }
    }

    pub fn start(&self, name: &str) -> AdminResult<()> {
        self.simple_op("admin.vm.Start", name, || cli_run(&["qvm-start", name]))
    }

    pub fn shutdown(&self, name: &str) -> AdminResult<()> {
        self.simple_op("admin.vm.Shutdown", name, || cli_run(&["qvm-shutdown", name]))
    }

    pub fn kill(&self, name: &str) -> AdminResult<()> {
        self.simple_op("admin.vm.Kill", name, || cli_run(&["qvm-kill", name]))
    }

    pub fn pause(&self, name: &str) -> AdminResult<()> {
        self.simple_op("admin.vm.Pause", name, || cli_run(&["qvm-pause", name]))
    }

    pub fn unpause(&self, name: &str) -> AdminResult<()> {
        self.simple_op("admin.vm.Unpause", name, || cli_run(&["qvm-unpause", name]))
    }

    pub fn remove(&self, name: &str) -> AdminResult<()> {
        self.simple_op("admin.vm.Remove", name, || cli_run(&["qvm-remove", "--force", name]))
    }

    pub fn create_appvm(&self, name: &str, template: &str, label: &str) -> AdminResult<()> {
        match &self.mode {
            AccessMode::Socket(p) => {
                let payload = format!("name={name} label={label}");
                let req = Request {
                    method:      "admin.vm.Create.AppVM",
                    destination: "dom0",
                    arg:         template,
                    payload:     payload.as_bytes(),
                };
                self.socket_call(p, &req)?;
                Ok(())
            }
            AccessMode::Cli => cli_run(&[
                "qvm-create", "--class", "AppVM",
                "--template", template,
                "--label", label,
                name,
            ]),
        }
    }

    /// Set a single VM property.
    /// `property` is the API name (e.g. `"memory"`, `"netvm"`, `"label"`).
    /// `value` is the string representation accepted by qubesd.
    pub fn set_property(&self, vm: &str, property: &str, value: &str) -> AdminResult<()> {
        match &self.mode {
            AccessMode::Socket(p) => {
                let payload = format!("{property}\0{value}");
                let req = Request {
                    method:      "admin.vm.property.Set",
                    destination: "dom0",
                    arg:         vm,
                    payload:     payload.as_bytes(),
                };
                self.socket_call(p, &req)?;
                Ok(())
            }
            AccessMode::Cli => cli_run(&["qvm-prefs", vm, property, value]),
        }
    }

    pub fn is_socket_mode(&self) -> bool {
        matches!(self.mode, AccessMode::Socket(_))
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn socket_call(&self, path: &Path, req: &Request<'_>) -> AdminResult<Response> {
        let mut stream = UnixStream::connect(path)?;
        stream.set_read_timeout(Some(TIMEOUT))?;
        stream.set_write_timeout(Some(TIMEOUT))?;

        let encoded = req.encode();
        stream.write_all(&encoded)?;
        // Signal end of payload to qubesd
        stream.shutdown(std::net::Shutdown::Write)?;

        let mut buf = Vec::new();
        stream.read_to_end(&mut buf)?;

        if buf.is_empty() {
            return Err(AdminError::ConnectionLost);
        }

        Response::decode(&buf)
    }

    fn simple_op(&self, method: &str, vm: &str, cli_fn: impl FnOnce() -> AdminResult<()>) -> AdminResult<()> {
        match &self.mode {
            AccessMode::Socket(p) => {
                let req = Request { method, destination: "dom0", arg: vm, payload: b"" };
                self.socket_call(p, &req)?;
                Ok(())
            }
            AccessMode::Cli => cli_fn(),
        }
    }
}

impl Default for AdminClient {
    fn default() -> Self {
        Self::new()
    }
}

// ── CLI fallback implementations ──────────────────────────────────────────────

fn cli_run(args: &[&str]) -> AdminResult<()> {
    let status = Command::new(args[0])
        .args(&args[1..])
        .status()
        .map_err(|e| AdminError::Io(e))?;
    if status.success() {
        Ok(())
    } else {
        Err(AdminError::Protocol(format!(
            "{} exited with status {}", args[0], status
        )))
    }
}

fn cli_list_qubes() -> AdminResult<Vec<QubeInfo>> {
    let out = Command::new("qvm-ls")
        .args(["--raw-data", "--fields", "name,class,state,label,template,netvm"])
        .output()
        .map_err(AdminError::Io)?;

    if !out.status.success() {
        return Err(AdminError::Protocol(
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ));
    }

    // qvm-ls --raw-data outputs pipe-separated values, one VM per line:
    //   name|class|state|label|template|netvm
    let text = std::str::from_utf8(&out.stdout)
        .map_err(|e| AdminError::Parse(e.to_string()))?;

    let mut qubes = Vec::new();
    for line in text.lines().skip(1) { // skip header
        let line = line.trim();
        if line.is_empty() { continue; }
        let cols: Vec<&str> = line.split('|').collect();
        if cols.len() < 6 { continue; }
        let name     = cols[0].to_string();
        let class    = super::types::QubeClass::from_str(cols[1]);
        let state    = super::types::QubeState::from_str(cols[2]);
        let label    = cols[3].to_string();
        let template = if cols[4].is_empty() || cols[4] == "-" { None } else { Some(cols[4].to_string()) };
        let netvm    = if cols[5].is_empty() || cols[5] == "-" || cols[5] == "None" { None } else { Some(cols[5].to_string()) };
        qubes.push(QubeInfo { name, class, state, label, template, netvm });
    }
    Ok(qubes)
}

fn cli_get_properties(name: &str) -> AdminResult<QubeProperties> {
    let out = Command::new("qvm-prefs")
        .arg(name)
        .output()
        .map_err(AdminError::Io)?;

    if !out.status.success() {
        return Err(AdminError::Protocol(
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ));
    }

    // qvm-prefs outputs: propname  value  (tab or space separated)
    let text = std::str::from_utf8(&out.stdout)
        .map_err(|e| AdminError::Parse(e.to_string()))?;

    let mut props = QubeProperties::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        // Split on first run of whitespace
        let mut parts = line.splitn(2, |c: char| c.is_whitespace());
        let key = parts.next().unwrap_or("").trim().to_string();
        let val = parts.next().unwrap_or("").trim().to_string();
        if key.is_empty() { continue; }
        props.raw.insert(key.clone(), val.clone());
        match key.as_str() {
            "memory"           => props.memory           = val.parse().ok(),
            "maxmem"           => props.maxmem           = val.parse().ok(),
            "vcpus"            => props.vcpus            = val.parse().ok(),
            "autostart"        => props.autostart        = parse_bool_cli(&val),
            "provides_network" => props.provides_network = parse_bool_cli(&val),
            "kernel"           => props.kernel           = Some(val),
            "default_dispvm"   => props.default_dispvm   = Some(val),
            _ => {}
        }
    }
    Ok(props)
}

fn parse_bool_cli(s: &str) -> Option<bool> {
    match s {
        "True" | "true" | "1" | "yes" => Some(true),
        "False" | "false" | "0" | "no" => Some(false),
        _ => None,
    }
}
