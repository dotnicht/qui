mod action;
mod admin;
mod app;
mod event;
mod terminal;
mod ui;

#[cfg(test)]
mod tests;

use std::sync::{mpsc, Arc};
use std::time::Duration;

use crossterm::event::{poll, read};

use action::{Action, SideEffect};
use admin::AdminClient;
use app::App;
use ui::UiState;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Arc::new(AdminClient::new());

    let (tx, rx) = mpsc::channel::<Action>();

    let mut app = App::new(client.clone());
    let mut ui = UiState::new();
    let mut term = terminal::init()?;

    spawn_effect(SideEffect::FetchQubeList, client.clone(), tx.clone());

    let tick = Duration::from_millis(200);
    let mut ticks: u32 = 0;

    loop {
        term.draw(|frame| ui::render(frame, &mut app, &mut ui))?;

        if poll(tick)? {
            if let Some(action) = read().ok().and_then(event::translate) {
                let effects = app.update(action);
                for eff in effects {
                    spawn_effect(eff, client.clone(), tx.clone());
                }
            }
        }

        ticks = ticks.wrapping_add(1);
        // Refresh stats every ~2s when on stats view
        if ticks % 10 == 0 && app.active_view == crate::app::ActiveView::StatsView {
            spawn_effect(SideEffect::FetchStats, client.clone(), tx.clone());
        }

        loop {
            match rx.try_recv() {
                Ok(action) => {
                    let effects = app.update(action);
                    for eff in effects {
                        spawn_effect(eff, client.clone(), tx.clone());
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break,
            }
        }

        if app.should_quit {
            break;
        }
    }

    terminal::restore()?;
    Ok(())
}

fn spawn_effect(eff: SideEffect, client: Arc<AdminClient>, tx: mpsc::Sender<Action>) {
    std::thread::spawn(move || match eff {
        SideEffect::FetchStats => {
            if let Ok(entries) = client.get_stats() {
                let _ = tx.send(Action::StatsLoaded(entries));
            }
        }
        SideEffect::FetchQubeList => match client.list_qubes() {
            Ok(qubes) => {
                let _ = tx.send(Action::QubeListLoaded(qubes));
            }
            Err(e) => {
                let _ = tx.send(Action::OperationFailed {
                    op_id: u64::MAX,
                    error: e.to_string(),
                });
            }
        },
        SideEffect::FetchProperties(name) => {
            if let Ok(props) = client.get_properties(&name) {
                let _ = tx.send(Action::PropertiesLoaded { name, props });
            }
        }
        SideEffect::StartVm(name) => {
            run_op(&client, &tx, &name, action::OpKind::Start, |c, n| {
                c.start(n)
            });
        }
        SideEffect::ShutdownVm(name) => {
            run_op(&client, &tx, &name, action::OpKind::Shutdown, |c, n| {
                c.shutdown(n)
            });
        }
        SideEffect::KillVm(name) => {
            run_op(&client, &tx, &name, action::OpKind::Kill, |c, n| c.kill(n));
        }
        SideEffect::PauseVm(name) => {
            run_op(&client, &tx, &name, action::OpKind::Pause, |c, n| {
                c.pause(n)
            });
        }
        SideEffect::DeleteVm(name) => {
            run_op(&client, &tx, &name, action::OpKind::Delete, |c, n| {
                c.remove(n)
            });
        }
        SideEffect::OpenTerminal(name) => {
            let _ = std::process::Command::new("qvm-run")
                .args([
                    "--service",
                    "--user=user",
                    &name,
                    "qubes.StartApp+qubes-run-terminal",
                ])
                .spawn();
        }
        SideEffect::SetProperty {
            vm,
            property,
            value,
        } => {
            if let Err(e) = client.set_property(&vm, &property, &value) {
                let _ = tx.send(Action::OperationFailed {
                    op_id: u64::MAX,
                    error: format!("set {property}: {e}"),
                });
            }
        }
    });
}

fn run_op<F>(
    client: &Arc<AdminClient>,
    tx: &mpsc::Sender<Action>,
    name: &str,
    kind: action::OpKind,
    f: F,
) where
    F: Fn(&AdminClient, &str) -> admin::AdminResult<()>,
{
    let op_id: u64 = name.bytes().fold(0u64, |acc, b| acc.wrapping_add(b as u64));
    match f(client, name) {
        Ok(()) => {
            let _ = tx.send(Action::OperationCompleted { op_id });
            if let Ok(qubes) = client.list_qubes() {
                let _ = tx.send(Action::QubeListLoaded(qubes));
            }
        }
        Err(e) => {
            let _ = tx.send(Action::OperationFailed {
                op_id,
                error: e.to_string(),
            });
        }
    }
    let _ = kind;
}
