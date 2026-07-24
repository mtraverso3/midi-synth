use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::execute;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};

use crate::sequencer::{Monitor, SharedMonitor};

/// Lowest and highest MIDI notes drawn in each channel's keyboard strip (an
/// 88-key piano spans A0..C8).
const LOW_NOTE: u8 = 21;
const HIGH_NOTE: u8 = 108;

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

/// Run the visualizer until playback finishes or the user quits. `total_s` is
/// the song length used for the progress bar.
pub fn run(monitor: &SharedMonitor, total_s: f64) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let start = Instant::now();
    let result = loop {
        let snapshot = *monitor.lock().unwrap();
        let elapsed = start.elapsed().as_secs_f64();

        if let Err(e) = terminal.draw(|f| draw(f, &snapshot, elapsed, total_s)) {
            break Err(e);
        }
        if snapshot.finished && elapsed >= total_s {
            break Ok(());
        }
        if quit_requested()? {
            break Ok(());
        }
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn quit_requested() -> io::Result<bool> {
    if !event::poll(Duration::from_millis(33))? {
        return Ok(false);
    }
    if let Event::Key(key) = event::read()? {
        let ctrl_c = key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL);
        if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) || ctrl_c {
            return Ok(true);
        }
    }
    Ok(false)
}

fn draw(frame: &mut ratatui::Frame, monitor: &Monitor, elapsed: f64, total_s: f64) {
    let channels: Vec<usize> = (0..16).filter(|&c| monitor.seen & (1 << c) != 0).collect();
    let voices: u32 = monitor.active.iter().map(|m| m.count_ones()).sum();

    let areas = Layout::vertical([
        Constraint::Length(3),                          // progress
        Constraint::Min(0),                             // channels
        Constraint::Length(1),                          // footer
    ])
    .split(frame.area());

    let ratio = if total_s > 0.0 {
        (elapsed / total_s).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" MIDI Player "))
        .gauge_style(Style::default().fg(Color::Cyan))
        .ratio(ratio)
        .label(format!("{elapsed:5.1}s / {total_s:5.1}s"));
    frame.render_widget(gauge, areas[0]);

    let lines: Vec<Line> = channels
        .iter()
        .map(|&ch| channel_line(ch, monitor.active[ch]))
        .collect();
    let channels_widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Channels "),
    );
    frame.render_widget(channels_widget, areas[1]);

    let footer = Paragraph::new(format!("  voices: {voices:2}    press q to quit"))
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, areas[2]);
}

fn channel_line(channel: usize, active: u128) -> Line<'static> {
    let color = CHANNEL_COLORS[channel % CHANNEL_COLORS.len()];
    let mut strip = String::with_capacity((HIGH_NOTE - LOW_NOTE + 1) as usize);
    for note in LOW_NOTE..=HIGH_NOTE {
        strip.push(if active & (1 << note) != 0 { '█' } else { '·' });
    }
    Line::from(vec![
        Span::styled(format!("ch{channel:2} "), Style::default().fg(Color::DarkGray)),
        Span::styled(strip, Style::default().fg(color)),
    ])
}
