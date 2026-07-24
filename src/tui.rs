use std::io;
use std::sync::mpsc::Sender;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use midi::sequencer::{Control, SEEK_STEP, SharedMonitor};
use midi::viz::draw;

/// Run the visualizer until playback finishes or the user quits, sending
/// transport commands to the play loop via `controls`.
pub fn run(
    monitor: &SharedMonitor,
    controls: &Sender<Control>,
    title: &str,
    total_s: f64,
) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let result = loop {
        let snapshot = *monitor.lock().unwrap();

        if let Err(e) = terminal.draw(|f| draw(f, &snapshot, title, total_s)) {
            break Err(e);
        }
        if snapshot.finished {
            break Ok(());
        }
        if handle_input(controls)?.is_some() {
            break Ok(());
        }
    };

    let _ = controls.send(Control::Stop);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

/// Poll for a keypress; returns `Some(())` when the user asks to quit.
fn handle_input(controls: &Sender<Control>) -> io::Result<Option<()>> {
    if !event::poll(Duration::from_millis(33))? {
        return Ok(None);
    }
    if let Event::Key(key) = event::read()? {
        let ctrl_c =
            key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(Some(())),
            _ if ctrl_c => return Ok(Some(())),
            KeyCode::Char(' ') => {
                let _ = controls.send(Control::TogglePause);
            }
            KeyCode::Left => {
                let _ = controls.send(Control::Seek(-SEEK_STEP));
            }
            KeyCode::Right => {
                let _ = controls.send(Control::Seek(SEEK_STEP));
            }
            _ => {}
        }
    }
    Ok(None)
}
