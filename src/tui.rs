use std::io;
use std::sync::mpsc::Sender;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::execute;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};

use crate::sequencer::{Control, Monitor, SEEK_STEP, SharedMonitor};
use crate::synth::family_name;

/// Lowest and highest MIDI notes drawn in each channel's keyboard strip (an
/// 88-key piano spans A0..C8).
const LOW_NOTE: u8 = 21;
const HIGH_NOTE: u8 = 108;

/// Width of the "chNN Instrument " label before each keyboard strip.
const LABEL_WIDTH: usize = 16;

const CHANNEL_COLORS: [Color; 8] = [
    Color::Cyan,
    Color::Green,
    Color::Yellow,
    Color::Magenta,
    Color::Blue,
    Color::Red,
    Color::LightCyan,
    Color::LightGreen,
];

/// Run the visualizer until playback finishes or the user quits, sending
/// transport commands to the play loop via `controls`. `title` names the file
/// and `total_s` is the song length used for the progress bar.
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
        let ctrl_c = key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL);
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

fn draw(frame: &mut ratatui::Frame, monitor: &Monitor, title: &str, total_s: f64) {
    let elapsed = monitor.song_time;
    let channels: Vec<usize> = (0..16).filter(|&c| monitor.seen & (1 << c) != 0).collect();
    let voices: u32 = monitor.active.iter().map(|m| m.count_ones()).sum();
    let active_channels = monitor.active.iter().filter(|m| **m != 0).count();

    let areas = Layout::vertical([
        Constraint::Length(3), // progress
        Constraint::Min(0),    // channels
        Constraint::Length(3), // stats
    ])
    .split(frame.area());

    let ratio = if total_s > 0.0 {
        (elapsed / total_s).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (marker, color) = if monitor.paused {
        ("⏸ PAUSED", Color::Yellow)
    } else {
        ("▶", Color::Cyan)
    };
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" ♪ {title} ")),
        )
        .gauge_style(Style::default().fg(color))
        .ratio(ratio)
        .label(format!("{marker}  {elapsed:5.1}s / {total_s:5.1}s"));
    frame.render_widget(gauge, areas[0]);

    let mut lines = vec![octave_ruler()];
    lines.extend(
        channels
            .iter()
            .map(|&ch| channel_line(ch, monitor.active[ch], monitor.programs[ch])),
    );
    let channels_widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Channels "));
    frame.render_widget(channels_widget, areas[1]);

    let stats = Paragraph::new(vec![
        Line::from(format!(
            "  voices {voices:2}     active channels {active_channels:2}/{:<2}     notes played {}",
            channels.len(),
            monitor.notes_played,
        )),
        Line::from(Span::styled(
            "  space pause    ← → seek 5s    q quit",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Stats "));
    frame.render_widget(stats, areas[2]);
}

/// A dim ruler above the strips: the octave digit sits at each C.
fn octave_ruler() -> Line<'static> {
    let mut ruler = String::new();
    for note in LOW_NOTE..=HIGH_NOTE {
        if note % 12 == 0 {
            ruler.push(char::from(b'0' + (note / 12 - 1)));
        } else {
            ruler.push(' ');
        }
    }
    Line::from(vec![
        Span::raw(" ".repeat(LABEL_WIDTH)),
        Span::styled(ruler, Style::default().fg(Color::DarkGray)),
    ])
}

fn channel_line(channel: usize, active: u128, program: u8) -> Line<'static> {
    let color = CHANNEL_COLORS[channel % CHANNEL_COLORS.len()];
    let name = family_name(channel as u8, program);
    let count = active.count_ones();

    let mut strip = String::with_capacity((HIGH_NOTE - LOW_NOTE + 1) as usize);
    for note in LOW_NOTE..=HIGH_NOTE {
        strip.push(if active & (1 << note) != 0 { '█' } else { '·' });
    }

    let label = format!("ch{channel:2} {name:<10} ");
    Line::from(vec![
        Span::styled(label, Style::default().fg(Color::Gray)),
        Span::styled(strip, Style::default().fg(color)),
        Span::styled(
            format!(" {count:2}"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}
