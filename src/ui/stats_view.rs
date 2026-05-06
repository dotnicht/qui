use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};
use ratatui::Frame;

use crate::app::App;
use crate::admin::QubeState;

pub fn render(frame: &mut Frame, area: Rect, app: &App, state: &mut TableState) {
    let header = Row::new([
        Cell::from("NAME").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("STATE").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("CPU%").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("CPU BAR").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("MEM").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("MEM BAR").style(Style::default().add_modifier(Modifier::BOLD)),
    ]);

    // Collect all running VMs with stats, sorted by CPU desc
    let mut entries: Vec<(&crate::admin::QubeInfo, f32, u64)> = app
        .qubes
        .iter()
        .filter(|q| q.state == QubeState::Running)
        .map(|q| {
            let (cpu, mem) = app
                .stats
                .get(&q.name)
                .map(|s| (s.cpu_pct, s.mem_kb))
                .unwrap_or((0.0, 0));
            (q, cpu, mem)
        })
        .collect();
    entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Max mem across all VMs for relative bar scaling
    let max_mem = entries.iter().map(|e| e.2).max().unwrap_or(1).max(1);

    let rows: Vec<Row> = entries
        .iter()
        .map(|(q, cpu, mem)| {
            let state_style = Style::default().fg(Color::Green);
            let cpu_color = if *cpu > 80.0 {
                Color::Red
            } else if *cpu > 50.0 {
                Color::Yellow
            } else {
                Color::Green
            };
            let mem_color = Color::Cyan;

            Row::new([
                Cell::from(q.name.clone()),
                Cell::from(Span::styled("RUN", state_style)),
                Cell::from(Span::styled(
                    format!("{:5.1}%", cpu),
                    Style::default().fg(cpu_color),
                )),
                Cell::from(Span::styled(
                    bar(*cpu / 100.0, 10),
                    Style::default().fg(cpu_color),
                )),
                Cell::from(Span::styled(
                    fmt_mem(*mem),
                    Style::default().fg(mem_color),
                )),
                Cell::from(Span::styled(
                    bar(*mem as f32 / max_mem as f32, 10),
                    Style::default().fg(mem_color),
                )),
            ])
        })
        .collect();

    let loading = if app.stats.is_empty() { " (loading…)" } else { "" };

    let table = Table::new(
        rows,
        [
            Constraint::Min(20),    // name
            Constraint::Length(5),  // state
            Constraint::Length(7),  // cpu%
            Constraint::Length(12), // cpu bar
            Constraint::Length(10), // mem
            Constraint::Length(12), // mem bar
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(format!(" Resources{loading} "))
            .borders(Borders::ALL),
    )
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .highlight_symbol("> ");

    frame.render_stateful_widget(table, area, state);
}

fn bar(frac: f32, width: usize) -> String {
    let filled = ((frac.clamp(0.0, 1.0) * width as f32).round() as usize).min(width);
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn fmt_mem(kb: u64) -> String {
    if kb >= 1024 * 1024 {
        format!("{:.1}G", kb as f64 / (1024.0 * 1024.0))
    } else if kb >= 1024 {
        format!("{:.0}M", kb as f64 / 1024.0)
    } else {
        format!("{kb}K")
    }
}
