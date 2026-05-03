use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct LayoutAreas {
    pub tab_bar:    Rect,
    pub main_table: Rect,
    pub key_hints:  Rect,
    pub status_bar: Rect,
}

pub fn compute(area: Rect) -> LayoutAreas {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // tab bar
            Constraint::Min(0),     // main content
            Constraint::Length(1),  // key hints
            Constraint::Length(1),  // status bar
        ])
        .split(area);

    LayoutAreas {
        tab_bar:    rows[0],
        main_table: rows[1],
        key_hints:  rows[2],
        status_bar: rows[3],
    }
}
