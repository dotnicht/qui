use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};
use ratatui::Frame;

use crate::admin::QubeState;
use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App, state: &mut TableState) {
    let header = Row::new([
        Cell::from("NAME").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("CLASS").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("STATE").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("LABEL").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("TEMPLATE").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("NETVM").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .height(1);

    let rows: Vec<Row> = app
        .filtered_indices
        .iter()
        .map(|&i| {
            let q = &app.qubes[i];

            let state_style = match &q.state {
                QubeState::Running => Style::default().fg(Color::Green),
                QubeState::Halted => Style::default().fg(Color::DarkGray),
                QubeState::Paused => Style::default().fg(Color::Yellow),
                QubeState::Transient => Style::default().fg(Color::Cyan),
                QubeState::Unknown(_) => Style::default(),
            };

            let label_color = label_to_color(&q.label);

            // Show spinner if there's a pending op on this VM
            let name_str = if app.pending_ops.iter().any(|op| op.vm_name == q.name) {
                format!("{} {}", spinner_char(), q.name)
            } else {
                q.name.clone()
            };

            Row::new([
                Cell::from(name_str),
                Cell::from(q.class.short_label()),
                Cell::from(Span::styled(q.state.short_label(), state_style)),
                Cell::from(Span::styled(
                    q.label.clone(),
                    Style::default().fg(label_color),
                )),
                Cell::from(q.template.clone().unwrap_or_else(|| "-".into())),
                Cell::from(q.netvm.clone().unwrap_or_else(|| "-".into())),
            ])
        })
        .collect();

    let title = match app.active_view {
        crate::app::ActiveView::QubeManager => " Qubes ",
        crate::app::ActiveView::ServiceManager => " Services ",
        crate::app::ActiveView::TemplateManager => " Templates ",
        crate::app::ActiveView::WhonixManager => " Whonix ",
        crate::app::ActiveView::DisposableManager => " Disposables ",
        crate::app::ActiveView::All => " All ",
    };

    let table = Table::new(
        rows,
        [
            Constraint::Min(20),   // name
            Constraint::Length(7), // class
            Constraint::Length(5), // state
            Constraint::Length(8), // label
            Constraint::Min(14),   // template
            Constraint::Min(14),   // netvm
        ],
    )
    .header(header)
    .block(Block::default().title(title).borders(Borders::ALL))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .highlight_symbol("> ");

    frame.render_stateful_widget(table, area, state);
}

fn label_to_color(label: &str) -> Color {
    super::label_color(label)
}

// Cheap single-frame spinner — advances each render based on system time mod 4
fn spinner_char() -> char {
    let idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_millis()
        / 250)
        % 4;
    ['⠋', '⠙', '⠹', '⠸'][idx as usize]
}

// Re-export Constraint so layout.rs doesn't need to import ratatui directly
use ratatui::layout::Constraint;
