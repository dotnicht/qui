mod detail_popup;
mod help_popup;
mod layout;
mod netvm_popup;
mod qube_table;
mod status_bar;

use ratatui::widgets::TableState;
use ratatui::Frame;

use crate::app::{App, Modal};

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

    // Main qube/template table (always rendered as background)
    qube_table::render(frame, areas.main_table, app, &mut ui.table_state);

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
            netvm_popup::render(frame, frame.area(), vm_name, candidates, selected);
        }
        Modal::None => {}
    }
}
