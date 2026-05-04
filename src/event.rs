use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crate::action::Action;

pub fn translate(event: Event) -> Option<Action> {
    match event {
        Event::Key(key) => translate_key(key),
        _ => None,
    }
}

fn translate_key(key: KeyEvent) -> Option<Action> {
    use KeyCode::*;
    use KeyModifiers as Mods;

    match (key.code, key.modifiers) {
        (Char('q'), Mods::NONE) | (Char('c'), Mods::CONTROL) => Some(Action::Quit),
        (Char('?'), _)  => Some(Action::ShowHelp),
        (Esc, _)        => Some(Action::Cancel),
        // Enter: EditSubmit when in edit modal; ToggleDetail otherwise.
        // App::update() disambiguates based on modal state.
        (Enter, _)      => Some(Action::EditSubmit),

        // Navigation (vim + arrows)
        (Char('j'), _) | (Down, _)  => Some(Action::MoveDown),
        (Char('k'), _) | (Up, _)    => Some(Action::MoveUp),
        (Char('g'), _) | (Home, _)  => Some(Action::MoveTop),
        (Char('G'), _) | (End, _)   => Some(Action::MoveBottom),

        // Tab switching
        (Char('1'), _) => Some(Action::SwitchToQubeManager),
        (Char('2'), _) => Some(Action::SwitchToServiceManager),
        (Char('3'), _) => Some(Action::SwitchToTemplateManager),
        (Char('4'), _) => Some(Action::SwitchToWhonixManager),

        // VM operations (only meaningful when no modal is open)
        (Char('s'), Mods::NONE) => Some(Action::StartSelected),
        (Char('S'), _)          => Some(Action::ShutdownSelected),
        (Char('K'), _)          => Some(Action::KillSelected),
        (Char('p'), _)          => Some(Action::PauseSelected),
        (Char('t'), _)          => Some(Action::OpenTerminal),
        (Char('d'), _)          => Some(Action::DeleteSelected),
        (Char('e'), _)          => Some(Action::EditProperty),

        // Confirmation dialogs
        (Char('y'), _)    => Some(Action::Confirm),

        // Edit input
        (Backspace, _)    => Some(Action::EditBackspace),
        (Char(c), _)      => Some(Action::EditChar(c)),

        _ => None,
    }
}
