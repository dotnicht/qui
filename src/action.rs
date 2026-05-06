#![allow(dead_code)]

use crate::admin::{AdminEvent, QubeInfo, QubeProperties};

#[derive(Debug, Clone)]
pub enum OpKind {
    Start,
    Shutdown,
    Kill,
    Pause,
    Create,
    Delete,
}

/// Side effects returned from App::update() that require spawning threads.
#[derive(Debug)]
pub enum SideEffect {
    FetchQubeList,
    FetchProperties(String),
    StartVm(String),
    ShutdownVm(String),
    KillVm(String),
    PauseVm(String),
    DeleteVm(String),
    OpenTerminal(String),
    SetProperty {
        vm: String,
        property: String,
        value: String,
    },
}

/// All actions that can be dispatched — from keyboard input or background threads.
#[derive(Debug, Clone)]
pub enum Action {
    // Navigation
    MoveUp,
    MoveDown,
    MoveTop,
    MoveBottom,

    // View switching
    SwitchToQubeManager,
    SwitchToServiceManager,
    SwitchToTemplateManager,
    SwitchToWhonixManager,
    SwitchToDisposableManager,
    SwitchToAll,
    ShowHelp,
    HideHelp,
    ToggleDetail,
    Quit,
    Confirm,
    Cancel,

    // VM operations
    StartSelected,
    ShutdownSelected,
    KillSelected,
    PauseSelected,
    OpenTerminal,
    DeleteSelected,
    ChangeNetvm,    // open netvm picker for selected VM
    ChangeLabel,    // open label picker for selected VM
    EditProperty,   // open edit modal for the highlighted property row
    EditChar(char), // printable character typed into the edit input
    EditBackspace,  // delete last char in edit input
    EditSubmit,     // confirm the edit (Enter)

    // Async results from background threads
    QubeListLoaded(Vec<QubeInfo>),
    PropertiesLoaded { name: String, props: QubeProperties },
    OperationCompleted { op_id: u64 },
    OperationFailed { op_id: u64, error: String },
    EventReceived(AdminEvent),
    Tick,
}
