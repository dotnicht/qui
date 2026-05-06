use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QubeState {
    Running,
    Halted,
    Paused,
    Transient,
    Unknown(String),
}

impl QubeState {
    pub fn from_str(s: &str) -> Self {
        match s {
            "Running" => Self::Running,
            "Halted" => Self::Halted,
            "Paused" => Self::Paused,
            "Transient" => Self::Transient,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn short_label(&self) -> &str {
        match self {
            Self::Running => "RUN",
            Self::Halted => "OFF",
            Self::Paused => "PAU",
            Self::Transient => "...",
            Self::Unknown(_) => "???",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QubeClass {
    AppVM,
    TemplateVM,
    StandaloneVM,
    DispVM,
    AdminVM,
    Unknown(String),
}

impl QubeClass {
    pub fn from_str(s: &str) -> Self {
        match s {
            "AppVM" => Self::AppVM,
            "TemplateVM" => Self::TemplateVM,
            "StandaloneVM" => Self::StandaloneVM,
            "DispVM" => Self::DispVM,
            "AdminVM" => Self::AdminVM,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn short_label(&self) -> &str {
        match self {
            Self::AppVM => "AppVM",
            Self::TemplateVM => "TmplVM",
            Self::StandaloneVM => "StndVM",
            Self::DispVM => "DispVM",
            Self::AdminVM => "Admin",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QubeInfo {
    pub name: String,
    pub class: QubeClass,
    pub state: QubeState,
    pub label: String,
    pub template: Option<String>,
    pub netvm: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct VmStats {
    pub cpu_pct: f32,
    pub mem_kb: u64,
}

#[derive(Debug, Clone, Default)]
pub struct QubeProperties {
    pub memory: Option<u64>,
    pub maxmem: Option<u64>,
    pub vcpus: Option<u32>,
    pub autostart: Option<bool>,
    pub provides_network: Option<bool>,
    pub kernel: Option<String>,
    pub default_dispvm: Option<String>,
    pub raw: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qubeclass_roundtrip() {
        let cases = [
            ("AppVM", QubeClass::AppVM, "AppVM"),
            ("TemplateVM", QubeClass::TemplateVM, "TmplVM"),
            ("StandaloneVM", QubeClass::StandaloneVM, "StndVM"),
            ("DispVM", QubeClass::DispVM, "DispVM"),
            ("AdminVM", QubeClass::AdminVM, "Admin"),
        ];
        for (input, expected, label) in cases {
            let c = QubeClass::from_str(input);
            assert_eq!(c, expected);
            assert_eq!(c.short_label(), label);
        }
    }

    #[test]
    fn qubeclass_unknown() {
        let c = QubeClass::from_str("FutureVM");
        assert_eq!(c.short_label(), "FutureVM");
    }

    #[test]
    fn qubestate_roundtrip() {
        let cases = [
            ("Running", QubeState::Running, "RUN"),
            ("Halted", QubeState::Halted, "OFF"),
            ("Paused", QubeState::Paused, "PAU"),
            ("Transient", QubeState::Transient, "..."),
        ];
        for (input, expected, label) in cases {
            let s = QubeState::from_str(input);
            assert_eq!(s, expected);
            assert_eq!(s.short_label(), label);
        }
    }

    #[test]
    fn qubestate_unknown() {
        let s = QubeState::from_str("Crashed");
        assert_eq!(s.short_label(), "???");
    }
}
