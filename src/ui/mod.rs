mod detail_popup;
mod help_popup;
mod layout;
mod netvm_popup;
mod qube_table;
mod stats_view;
mod status_bar;

use ratatui::style::Color;
use ratatui::widgets::TableState;
use ratatui::Frame;

use crate::app::{ActiveView, App, Modal};

pub(super) fn label_color(label: &str) -> Color {
    match label {
        "red" => Color::Red,
        "orange" => Color::Rgb(245, 121, 0),
        "yellow" => Color::Yellow,
        "green" => Color::Green,
        "gray" | "grey" => Color::DarkGray,
        "black" => Color::DarkGray,
        "white" => Color::White,
        "blue" => Color::Blue,
        "purple" => Color::Magenta,
        _ => Color::Cyan,
    }
}

pub struct UiState {
    pub table_state: TableState,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            table_state: TableState::default(),
        }
    }
}

pub fn render(frame: &mut Frame, app: &mut App, ui: &mut UiState) {
    let areas = layout::compute(frame.area());

    // Sync table selection
    ui.table_state.select(if app.filtered_indices.is_empty() {
        None
    } else {
        Some(app.selected_index)
    });

    // Tab bar
    status_bar::render_tabs(frame, areas.tab_bar, app);

    // Main table — stats view gets its own renderer
    if app.active_view == ActiveView::StatsView {
        stats_view::render(frame, areas.main_table, app, &mut ui.table_state);
    } else {
        qube_table::render(frame, areas.main_table, app, &mut ui.table_state);
    }

    // Bottom key hints + status
    status_bar::render_bottom(frame, areas.key_hints, areas.status_bar, app);

    // Overlays (drawn last — appear on top)
    match app.modal.clone() {
        Modal::Help => help_popup::render(frame, frame.area()),
        Modal::Detail => {
            if let Some(qube) = app.selected_qube().cloned() {
                detail_popup::render(frame, frame.area(), &qube, app);
            }
        }
        Modal::EditProperty {
            ref vm_name,
            ref property,
            ref input,
        } => {
            // Render the detail popup underneath first (re-use detail render)
            if let Some(qube) = app.selected_qube().cloned() {
                detail_popup::render(frame, frame.area(), &qube, app);
            }
            // Then the edit input on top
            detail_popup::render_edit(frame, frame.area(), vm_name, property, input);
        }
        Modal::ConfirmDelete { ref vm_name } => {
            help_popup::render_confirm(
                frame,
                frame.area(),
                &format!("Delete '{vm_name}'? This cannot be undone. [y/N]"),
            );
        }
        Modal::ConfirmKill { ref vm_name } => {
            help_popup::render_confirm(
                frame,
                frame.area(),
                &format!("Force-kill '{vm_name}'? [y/N]"),
            );
        }
        Modal::ChangeNetvm {
            ref vm_name,
            ref candidates,
            selected,
        } => {
            netvm_popup::render(frame, frame.area(), &format!("NetVM for {vm_name}"), candidates, selected, |_| Color::Cyan);
        }
        Modal::ChangeLabel { ref vm_name, selected } => {
            let candidates: Vec<String> = crate::app::LABELS.iter().map(|s| s.to_string()).collect();
            netvm_popup::render(frame, frame.area(), &format!("Label for {vm_name}"), &candidates, selected, label_color);
        }
        Modal::ChangeTemplate {
            ref vm_name,
            ref candidates,
            selected,
        } => {
            netvm_popup::render(frame, frame.area(), &format!("Template for {vm_name}"), candidates, selected, |_| Color::Cyan);
        }
        Modal::None => {}
    }
}
