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
            "Running"   => Self::Running,
            "Halted"    => Self::Halted,
            "Paused"    => Self::Paused,
            "Transient" => Self::Transient,
            other       => Self::Unknown(other.to_string()),
        }
    }

    pub fn short_label(&self) -> &str {
        match self {
            Self::Running    => "RUN",
            Self::Halted     => "OFF",
            Self::Paused     => "PAU",
            Self::Transient  => "...",
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
            "AppVM"        => Self::AppVM,
            "TemplateVM"   => Self::TemplateVM,
            "StandaloneVM" => Self::StandaloneVM,
            "DispVM"       => Self::DispVM,
            "AdminVM"      => Self::AdminVM,
            other          => Self::Unknown(other.to_string()),
        }
    }

    pub fn short_label(&self) -> &str {
        match self {
            Self::AppVM       => "AppVM",
            Self::TemplateVM  => "TmplVM",
            Self::StandaloneVM => "StndVM",
            Self::DispVM      => "DispVM",
            Self::AdminVM     => "Admin",
            Self::Unknown(s)  => s.as_str(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QubeInfo {
    pub name:     String,
    pub class:    QubeClass,
    pub state:    QubeState,
    pub label:    String,
    pub template: Option<String>,
    pub netvm:    Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct QubeProperties {
    pub memory:           Option<u64>,
    pub maxmem:           Option<u64>,
    pub vcpus:            Option<u32>,
    pub autostart:        Option<bool>,
    pub provides_network: Option<bool>,
    pub kernel:           Option<String>,
    pub default_dispvm:   Option<String>,
    pub raw:              HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct QubeStats {
    pub cpu_time:  u64,
    pub memory_kb: u64,
}
