use std::io;
use crossterm::{execute, terminal};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

pub type Tui = Terminal<CrosstermBackend<io::Stderr>>;

pub fn init() -> io::Result<Tui> {
    terminal::enable_raw_mode()?;
    execute!(io::stderr(), terminal::EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(io::stderr()))
}

pub fn restore() -> io::Result<()> {
    terminal::disable_raw_mode()?;
    execute!(io::stderr(), terminal::LeaveAlternateScreen)
}
