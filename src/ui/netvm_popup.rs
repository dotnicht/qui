use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn render(frame: &mut Frame, area: Rect, vm_name: &str, candidates: &[String], selected: usize) {
    let height = (candidates.len() as u16 + 6).min(area.height.saturating_sub(4));
    let popup = centered_rect_abs(50, height, area);
    frame.render_widget(Clear, popup);

    let mut lines: Vec<Line> = vec![Line::raw("")];
    for (i, name) in candidates.iter().enumerate() {
        if i == selected {
            lines.push(Line::styled(
                format!("  ▶ {name}"),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(name.clone(), Style::default().fg(Color::Cyan)),
            ]));
        }
    }
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        "  j/k navigate  Enter=confirm  Esc=cancel",
        Style::default().fg(Color::DarkGray),
    ));

    let para = Paragraph::new(lines).block(
        Block::default()
            .title(format!(" NetVM for {} ", vm_name))
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(para, popup);
}

fn centered_rect_abs(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}
