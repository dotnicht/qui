use crate::action::Action;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

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
        (Char('?'), _) => Some(Action::ShowHelp),
        (Esc, _) => Some(Action::Cancel),
        // Enter: EditSubmit when in edit modal; ToggleDetail otherwise.
        // App::update() disambiguates based on modal state.
        (Enter, _) => Some(Action::EditSubmit),

        // Navigation (vim + arrows)
        (Char('j'), _) | (Down, _) => Some(Action::MoveDown),
        (Char('k'), _) | (Up, _) => Some(Action::MoveUp),
        (Char('g'), _) | (Home, _) => Some(Action::MoveTop),
        (Char('G'), _) | (End, _) => Some(Action::MoveBottom),

        // Tab switching
        (Char('1'), _) => Some(Action::SwitchToQubeManager),
        (Char('2'), _) => Some(Action::SwitchToServiceManager),
        (Char('3'), _) => Some(Action::SwitchToTemplateManager),
        (Char('4'), _) => Some(Action::SwitchToWhonixManager),
        (Char('5'), _) => Some(Action::SwitchToDisposableManager),
        (Char('6'), _) => Some(Action::SwitchToAll),
        (Char('7'), _) => Some(Action::SwitchToStatsView),

        // VM operations (only meaningful when no modal is open)
        (Char('s'), Mods::NONE) => Some(Action::StartSelected),
        (Char('S'), _) => Some(Action::ShutdownSelected),
        (Char('K'), _) => Some(Action::KillSelected),
        (Char('p'), _) => Some(Action::PauseSelected),
        (Char('t'), _) => Some(Action::OpenTerminal),
        (Char('d'), _) => Some(Action::DeleteSelected),
        (Char('n'), _) => Some(Action::ChangeNetvm),
        (Char('c'), Mods::NONE) => Some(Action::ChangeLabel),
        (Char('T'), _) => Some(Action::ChangeTemplate),
        (Char('e'), _) => Some(Action::EditProperty),

        // Confirmation dialogs
        (Char('y'), _) => Some(Action::Confirm),

        // Edit input
        (Backspace, _) => Some(Action::EditBackspace),
        (Char(c), _) => Some(Action::EditChar(c)),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode, mods: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, mods))
    }

    fn press(code: KeyCode) -> Option<Action> {
        translate(key(code, KeyModifiers::NONE))
    }

    #[test]
    fn quit_and_ctrl_c() {
        assert!(matches!(press(KeyCode::Char('q')), Some(Action::Quit)));
        assert!(matches!(
            translate(key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(Action::Quit)
        ));
    }

    #[test]
    fn navigation_keys() {
        assert!(matches!(press(KeyCode::Char('j')), Some(Action::MoveDown)));
        assert!(matches!(press(KeyCode::Down), Some(Action::MoveDown)));
        assert!(matches!(press(KeyCode::Char('k')), Some(Action::MoveUp)));
        assert!(matches!(press(KeyCode::Up), Some(Action::MoveUp)));
        assert!(matches!(press(KeyCode::Char('g')), Some(Action::MoveTop)));
        assert!(matches!(press(KeyCode::Home), Some(Action::MoveTop)));
        assert!(matches!(press(KeyCode::Char('G')), Some(Action::MoveBottom)));
        assert!(matches!(press(KeyCode::End), Some(Action::MoveBottom)));
    }

    #[test]
    fn tab_switching_keys() {
        assert!(matches!(press(KeyCode::Char('1')), Some(Action::SwitchToQubeManager)));
        assert!(matches!(press(KeyCode::Char('2')), Some(Action::SwitchToServiceManager)));
        assert!(matches!(press(KeyCode::Char('3')), Some(Action::SwitchToTemplateManager)));
        assert!(matches!(press(KeyCode::Char('4')), Some(Action::SwitchToWhonixManager)));
        assert!(matches!(press(KeyCode::Char('5')), Some(Action::SwitchToDisposableManager)));
        assert!(matches!(press(KeyCode::Char('6')), Some(Action::SwitchToAll)));
    }

    #[test]
    fn vm_operation_keys() {
        assert!(matches!(press(KeyCode::Char('s')), Some(Action::StartSelected)));
        assert!(matches!(press(KeyCode::Char('S')), Some(Action::ShutdownSelected)));
        assert!(matches!(press(KeyCode::Char('K')), Some(Action::KillSelected)));
        assert!(matches!(press(KeyCode::Char('p')), Some(Action::PauseSelected)));
        assert!(matches!(press(KeyCode::Char('t')), Some(Action::OpenTerminal)));
        assert!(matches!(press(KeyCode::Char('d')), Some(Action::DeleteSelected)));
        assert!(matches!(press(KeyCode::Char('n')), Some(Action::ChangeNetvm)));
        assert!(matches!(press(KeyCode::Char('c')), Some(Action::ChangeLabel)));
        assert!(matches!(press(KeyCode::Char('e')), Some(Action::EditProperty)));
    }

    #[test]
    fn misc_keys() {
        assert!(matches!(press(KeyCode::Esc), Some(Action::Cancel)));
        assert!(matches!(press(KeyCode::Enter), Some(Action::EditSubmit)));
        assert!(matches!(press(KeyCode::Char('?')), Some(Action::ShowHelp)));
        assert!(matches!(press(KeyCode::Char('y')), Some(Action::Confirm)));
        assert!(matches!(press(KeyCode::Backspace), Some(Action::EditBackspace)));
    }

    #[test]
    fn printable_char_becomes_edit_char() {
        assert!(matches!(press(KeyCode::Char('x')), Some(Action::EditChar('x'))));
    }

    #[test]
    fn non_key_event_returns_none() {
        assert!(translate(Event::Resize(80, 24)).is_none());
    }
}
