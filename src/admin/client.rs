use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::error::{AdminError, AdminResult};
use super::protocol::{parse_properties, parse_vm_list, Request, Response};
use super::types::{QubeInfo, QubeProperties, VmStats};

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
            Self {
                mode: AccessMode::Socket(path),
            }
        } else {
            Self {
                mode: AccessMode::Cli,
            }
        }
    }

    // ── Public API ────────────────────────────────────────────────────────────

    pub fn list_qubes(&self) -> AdminResult<Vec<QubeInfo>> {
        match &self.mode {
            AccessMode::Socket(p) => {
                let req = Request {
                    method: "admin.vm.List",
                    destination: "dom0",
                    arg: "",
                    payload: b"",
                };
                let resp = self.socket_call(p, &req)?;
                parse_vm_list(&resp.data)
            }
            AccessMode::Cli => cli_list_qubes(),
        }
    }

    pub fn get_properties(&self, name: &str) -> AdminResult<QubeProperties> {
        match &self.mode {
            AccessMode::Socket(p) => {
                let req = Request {
                    method: "admin.vm.property.GetAll",
                    destination: "dom0",
                    arg: name,
                    payload: b"",
                };
                let resp = self.socket_call(p, &req)?;
                parse_properties(&resp.data)
            }
            AccessMode::Cli => cli_get_properties(name),
        }
    }

    pub fn start(&self, name: &str) -> AdminResult<()> {
        self.simple_op("admin.vm.Start", name, || cli_run(&["qvm-start", name]))
    }

    pub fn shutdown(&self, name: &str) -> AdminResult<()> {
        self.simple_op("admin.vm.Shutdown", name, || {
            cli_run(&["qvm-shutdown", name])
        })
    }

    pub fn kill(&self, name: &str) -> AdminResult<()> {
        self.simple_op("admin.vm.Kill", name, || cli_run(&["qvm-kill", name]))
    }

    pub fn pause(&self, name: &str) -> AdminResult<()> {
        self.simple_op("admin.vm.Pause", name, || cli_run(&["qvm-pause", name]))
    }

    pub fn remove(&self, name: &str) -> AdminResult<()> {
        self.simple_op("admin.vm.Remove", name, || {
            cli_run(&["qvm-remove", "--force", name])
        })
    }

    /// Set a single VM property.
    /// `property` is the API name (e.g. `"memory"`, `"netvm"`, `"label"`).
    /// `value` is the string representation accepted by qubesd.
    pub fn get_stats(&self) -> AdminResult<Vec<(String, VmStats)>> {
        match &self.mode {
            AccessMode::Socket(p) => {
                let req = Request {
                    method: "admin.vm.stats",
                    destination: "dom0",
                    arg: "",
                    payload: b"",
                };
                let resp = self.socket_call(p, &req)?;
                Ok(parse_stats_socket(&resp.data))
            }
            AccessMode::Cli => cli_get_stats(),
        }
    }

    pub fn set_property(&self, vm: &str, property: &str, value: &str) -> AdminResult<()> {
        match &self.mode {
            AccessMode::Socket(p) => {
                let payload = format!("{property}\0{value}");
                let req = Request {
                    method: "admin.vm.property.Set",
                    destination: "dom0",
                    arg: vm,
                    payload: payload.as_bytes(),
                };
                self.socket_call(p, &req)?;
                Ok(())
            }
            AccessMode::Cli => cli_run(&["qvm-prefs", vm, property, value]),
        }
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

    fn simple_op(
        &self,
        method: &str,
        vm: &str,
        cli_fn: impl FnOnce() -> AdminResult<()>,
    ) -> AdminResult<()> {
        match &self.mode {
            AccessMode::Socket(p) => {
                let req = Request {
                    method,
                    destination: "dom0",
                    arg: vm,
                    payload: b"",
                };
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
        .map_err(AdminError::Io)?;
    if status.success() {
        Ok(())
    } else {
        Err(AdminError::Protocol(format!(
            "{} exited with status {}",
            args[0], status
        )))
    }
}

fn cli_list_qubes() -> AdminResult<Vec<QubeInfo>> {
    let out = Command::new("qvm-ls")
        .args([
            "--raw-data",
            "--fields",
            "name,class,state,label,template,netvm",
        ])
        .output()
        .map_err(AdminError::Io)?;

    if !out.status.success() {
        return Err(AdminError::Protocol(
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ));
    }

    // qvm-ls --raw-data outputs pipe-separated values, one VM per line:
    //   name|class|state|label|template|netvm
    let text = std::str::from_utf8(&out.stdout).map_err(|e| AdminError::Parse(e.to_string()))?;

    let mut qubes = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('|').collect();
        if cols.is_empty() {
            continue;
        }
        let name = cols[0].to_string();
        let class = super::types::QubeClass::from_str(cols.get(1).copied().unwrap_or(""));
        let state = super::types::QubeState::from_str(cols.get(2).copied().unwrap_or(""));
        let label = cols.get(3).copied().unwrap_or("").to_string();
        let template = cols.get(4).filter(|&&v| !v.is_empty() && v != "-").map(|v| v.to_string());
        let netvm = cols.get(5).filter(|&&v| !v.is_empty() && v != "-" && v != "None").map(|v| v.to_string());
        qubes.push(QubeInfo {
            name,
            class,
            state,
            label,
            template,
            netvm,
        });
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
    let text = std::str::from_utf8(&out.stdout).map_err(|e| AdminError::Parse(e.to_string()))?;

    let mut props = QubeProperties::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Split on first run of whitespace
        let mut parts = line.splitn(2, |c: char| c.is_whitespace());
        let key = parts.next().unwrap_or("").trim().to_string();
        let val = parts.next().unwrap_or("").trim().to_string();
        if key.is_empty() {
            continue;
        }
        props.raw.insert(key.clone(), val.clone());
        match key.as_str() {
            "memory" => props.memory = val.parse().ok(),
            "maxmem" => props.maxmem = val.parse().ok(),
            "vcpus" => props.vcpus = val.parse().ok(),
            "autostart" => props.autostart = parse_bool_cli(&val),
            "provides_network" => props.provides_network = parse_bool_cli(&val),
            "kernel" => props.kernel = Some(val),
            "default_dispvm" => props.default_dispvm = Some(val),
            _ => {}
        }
    }
    Ok(props)
}

// ── stats parsers ─────────────────────────────────────────────────────────────

// Socket response format (admin.vm.stats):
//   vm_name\0cpu_time=N\0mem_used=N\0\0  (per VM, repeated)
fn parse_stats_socket(data: &[u8]) -> Vec<(String, VmStats)> {
    let mut out = Vec::new();
    for chunk in data.split(|&b| b == 0).collect::<Vec<_>>().chunks(2) {
        if chunk.len() < 2 { continue; }
        let name = String::from_utf8_lossy(chunk[0]).to_string();
        if name.is_empty() { continue; }
        let mut cpu_pct = 0.0f32;
        let mut mem_kb = 0u64;
        for kv in String::from_utf8_lossy(chunk[1]).split_whitespace() {
            if let Some(v) = kv.strip_prefix("cpu_usage=") {
                cpu_pct = v.parse::<f32>().unwrap_or(0.0) / 10.0;
            } else if let Some(v) = kv.strip_prefix("mem_used=") {
                mem_kb = v.parse::<u64>().unwrap_or(0) / 1024;
            }
        }
        out.push((name, VmStats { cpu_pct, mem_kb }));
    }
    out
}

// CLI fallback: parse `xl list` output
// Format (header + one row per domain):
//   Name  ID  Mem  VCPUs  State  Time(s)
fn cli_get_stats() -> AdminResult<Vec<(String, VmStats)>> {
    let out = Command::new("xl")
        .args(["list"])
        .output()
        .map_err(AdminError::Io)?;

    let text = std::str::from_utf8(&out.stdout).map_err(|e| AdminError::Parse(e.to_string()))?;
    let mut result = Vec::new();

    for line in text.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 6 { continue; }
        let name = cols[0].to_string();
        let mem_kb = cols[2].parse::<u64>().unwrap_or(0) * 1024;
        result.push((name, VmStats { cpu_pct: 0.0, mem_kb }));
    }
    Ok(result)
}

fn parse_bool_cli(s: &str) -> Option<bool> {
    match s {
        "True" | "true" | "1" | "yes" => Some(true),
        "False" | "false" | "0" | "no" => Some(false),
        _ => None,
    }
}
