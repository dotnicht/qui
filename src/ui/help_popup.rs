use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn render(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(60, 70, area);
    frame.render_widget(Clear, popup);

    let lines = vec![
        Line::raw(""),
        Line::styled(
            "  Navigation",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        binding("j / ↓", "Move down"),
        binding("k / ↑", "Move up"),
        binding("g / Home", "Jump to top"),
        binding("G / End", "Jump to bottom"),
        Line::raw(""),
        Line::styled(
            "  VM Operations",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        binding("s", "Start VM"),
        binding("S", "Shutdown VM (graceful)"),
        binding("K", "Kill VM (force — confirms first)"),
        binding("p", "Pause / unpause VM"),
        binding("t", "Open terminal in VM"),
        binding("d", "Delete VM (confirms first)"),
        Line::raw(""),
        Line::styled("  Views", Style::default().add_modifier(Modifier::BOLD)),
        binding("1", "Qube Manager"),
        binding("2", "Template Manager"),
        binding("Enter", "Show detail popup"),
        Line::raw(""),
        Line::styled("  Other", Style::default().add_modifier(Modifier::BOLD)),
        binding("?", "This help"),
        binding("Esc", "Close popup / cancel"),
        binding("q / Ctrl-C", "Quit"),
        Line::raw(""),
    ];

    let para = Paragraph::new(lines).block(
        Block::default()
            .title(" Help — press Esc to close ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(para, popup);
}

pub fn render_confirm(frame: &mut Frame, area: Rect, message: &str) {
    let popup = centered_rect(50, 20, area);
    frame.render_widget(Clear, popup);

    let lines = vec![
        Line::raw(""),
        Line::from(vec![Span::raw(format!("  {message}"))]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "y",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" = confirm   "),
            Span::styled("Esc / n", Style::default().fg(Color::Red)),
            Span::raw(" = cancel"),
        ]),
    ];

    let para = Paragraph::new(lines).block(
        Block::default()
            .title(" Confirm ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(para, popup);
}

fn binding<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("{key:<14}"), Style::default().fg(Color::Yellow)),
        Span::raw(desc),
    ])
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vert[1])[1]
}
