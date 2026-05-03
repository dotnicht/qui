use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::admin::QubeInfo;
use crate::app::{App, Modal};

pub fn render(frame: &mut Frame, area: Rect, qube: &QubeInfo, app: &mut App) {
    let popup = centered_rect(70, 80, area);
    frame.render_widget(Clear, popup);

    // Build the editable property rows.
    // Each entry: (api_key, display_label, current_value, editable)
    let mut rows: Vec<(String, String, String, bool)> = Vec::new();

    // Non-editable info rows first
    rows.push(("".into(), "Class".into(),    qube.class.short_label().to_string(), false));
    rows.push(("".into(), "State".into(),    qube.state.short_label().to_string(), false));

    // Editable rows
    rows.push(("label".into(),    "Label".into(),
        qube.label.clone(), true));
    rows.push(("template".into(), "Template".into(),
        qube.template.clone().unwrap_or_else(|| "-".into()), true));
    rows.push(("netvm".into(),    "NetVM".into(),
        qube.netvm.clone().unwrap_or_else(|| "-".into()), true));

    if let Some(props) = app.properties_cache.get(&qube.name) {
        if let Some(v) = props.memory   { rows.push(("memory".into(),  "Memory (MiB)".into(), v.to_string(), true)); }
        if let Some(v) = props.maxmem   { rows.push(("maxmem".into(),  "Max memory".into(),   v.to_string(), true)); }
        if let Some(v) = props.vcpus    { rows.push(("vcpus".into(),   "vCPUs".into(),        v.to_string(), true)); }
        if let Some(v) = props.autostart { rows.push(("autostart".into(), "Autostart".into(), v.to_string(), true)); }
        if let Some(v) = props.provides_network {
            rows.push(("provides_network".into(), "Provides network".into(), v.to_string(), true));
        }
        if let Some(ref v) = props.kernel {
            rows.push(("kernel".into(), "Kernel".into(), v.clone(), true));
        }
        if let Some(ref v) = props.default_dispvm {
            rows.push(("default_dispvm".into(), "Default DispVM".into(), v.clone(), true));
        }

        let typed = ["memory","maxmem","vcpus","autostart","provides_network","kernel","default_dispvm"];
        let mut extra: Vec<_> = props.raw.iter()
            .filter(|(k, _)| !typed.contains(&k.as_str()))
            .collect();
        extra.sort_by_key(|(k, _)| k.as_str());
        for (k, v) in extra {
            rows.push((k.clone(), k.clone(), v.clone(), true));
        }
    }

    // Write the editable row index back to app so update() can look up by detail_row
    app.detail_rows = rows.iter()
        .filter(|(key, _, _, editable)| *editable && !key.is_empty())
        .map(|(key, label, _, _)| (key.clone(), label.clone()))
        .collect();

    // Clamp detail_row
    if !app.detail_rows.is_empty() {
        app.detail_row = app.detail_row.min(app.detail_rows.len() - 1);
    }

    // Determine which global row index corresponds to detail_row
    // (detail_row indexes into the editable subset; we need the full row index for highlight)
    let editable_indices: Vec<usize> = rows.iter().enumerate()
        .filter(|(_, (key, _, _, editable))| *editable && !key.is_empty())
        .map(|(i, _)| i)
        .collect();
    let highlighted_row = editable_indices.get(app.detail_row).copied();

    let selected_style   = Style::default().add_modifier(Modifier::REVERSED);
    let key_style        = Style::default().add_modifier(Modifier::BOLD);
    let editable_style   = Style::default().fg(Color::Cyan);
    let hint_style       = Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::raw(""));

    for (i, (_, label, value, editable)) in rows.iter().enumerate() {
        let is_highlighted = Some(i) == highlighted_row && matches!(app.modal, Modal::Detail);

        let label_span = Span::styled(format!("{label:<18}"), key_style);
        let value_span = if *editable {
            Span::styled(value.clone(), editable_style)
        } else {
            Span::raw(value.clone())
        };

        let hint_span = if is_highlighted {
            Span::styled("  [e]dit", hint_style)
        } else {
            Span::raw("")
        };

        let line = if is_highlighted {
            Line::styled(
                format!("  {label:<18}{value}  [e]dit"),
                selected_style,
            )
        } else {
            Line::from(vec![
                Span::raw("  "),
                label_span,
                value_span,
                hint_span,
            ])
        };
        lines.push(line);
    }

    // Loading indicator
    if app.properties_cache.get(&qube.name).is_none() {
        lines.push(Line::raw(""));
        lines.push(Line::raw("  Loading properties…"));
    }

    // Pending ops
    let pending: Vec<_> = app.pending_ops.iter()
        .filter(|op| op.vm_name == qube.name)
        .collect();
    if !pending.is_empty() {
        lines.push(Line::raw(""));
        for op in pending {
            lines.push(Line::styled(
                format!("  ⟳ {:?} in progress…", op.kind),
                Style::default().fg(Color::Yellow),
            ));
        }
    }

    lines.push(Line::raw(""));
    lines.push(Line::styled("  j/k navigate  e edit  Esc close",
        Style::default().fg(Color::DarkGray)));

    let title = format!(" {} ", qube.name);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    let para = Paragraph::new(lines)
        .block(block)
        .scroll((app.detail_scroll, 0))
        .wrap(Wrap { trim: false });

    frame.render_widget(para, popup);
}

/// Render the inline edit input box on top of the detail popup.
pub fn render_edit(frame: &mut Frame, area: Rect, vm: &str, property: &str, input: &str) {
    let popup = centered_rect(60, 25, area);
    frame.render_widget(Clear, popup);

    let title = format!(" Edit {property} ");
    let hint  = format!("  VM: {vm}\n\n  Value: {input}█\n\n  Enter=confirm  Esc=cancel");

    let para = Paragraph::new(hint)
        .block(Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow)));

    frame.render_widget(para, popup);
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
