use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{ActiveView, App, MessageLevel};

pub fn render_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans = vec![Span::raw(" ")];

    let tab_style_active = Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED);
    let tab_style_inactive = Style::default().fg(Color::DarkGray);

    let tabs = [
        (ActiveView::QubeManager, "[1] Qubes"),
        (ActiveView::ServiceManager, "[2] Services"),
        (ActiveView::TemplateManager, "[3] Templates"),
        (ActiveView::WhonixManager, "[4] Whonix"),
        (ActiveView::DisposableManager, "[5] Disposables"),
        (ActiveView::All, "[6] All"),
    ];

    for (view, label) in &tabs {
        let style = if app.active_view == *view {
            tab_style_active
        } else {
            tab_style_inactive
        };
        spans.push(Span::styled(format!(" {label} "), style));
        spans.push(Span::raw("  "));
    }

    // Right-align hints
    let hint = " ?=help  q=quit ";
    let hint_width = hint.len() as u16;
    if area.width > hint_width + 20 {
        let pad_width = area
            .width
            .saturating_sub(spans.iter().map(|s| s.content.len() as u16).sum::<u16>() + hint_width);
        spans.push(Span::raw(" ".repeat(pad_width as usize)));
        spans.push(Span::styled(hint, Style::default().fg(Color::DarkGray)));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

pub fn render_bottom(frame: &mut Frame, hints_area: Rect, status_area: Rect, app: &App) {
    // Key hints line
    let hints_text = " [s]tart [S]shutdown [K]ill [p]ause [t]terminal [d]delete [n]etvm [Enter]detail";
    frame.render_widget(
        Paragraph::new(hints_text).style(Style::default().fg(Color::DarkGray)),
        hints_area,
    );

    // Status line
    let (text, color) = match &app.status {
        Some(msg) => {
            let c = match msg.level {
                MessageLevel::Success => Color::Green,
                MessageLevel::Warning => Color::Yellow,
                MessageLevel::Error => Color::Red,
                MessageLevel::Info => Color::Cyan,
            };
            (format!(" {}", msg.text), c)
        }
        None => (String::new(), Color::Reset),
    };

    // Pending ops count on the right
    let pending_str = if app.pending_ops.is_empty() {
        String::new()
    } else {
        format!(" ops: {} pending", app.pending_ops.len())
    };

    let status_line = if pending_str.is_empty() {
        Line::from(vec![Span::styled(text, Style::default().fg(color))])
    } else {
        let pad = status_area
            .width
            .saturating_sub(text.len() as u16 + pending_str.len() as u16);
        Line::from(vec![
            Span::styled(text, Style::default().fg(color)),
            Span::raw(" ".repeat(pad as usize)),
            Span::styled(pending_str, Style::default().fg(Color::Yellow)),
        ])
    };

    frame.render_widget(Paragraph::new(status_line), status_area);
}
