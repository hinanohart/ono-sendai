//! Frame drawing. Layout: title bar (1 line) / chat log (flex) / input
//! line (1 line) / status line (1 line).

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(f.area());

    let title = Paragraph::new(Line::from("─── ono-sendai ─── Console Cowboy deck ───"))
        .style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(title, chunks[0]);

    let log_text: Vec<Line> = app.log.iter().map(|l| Line::from(l.clone())).collect();
    let log =
        Paragraph::new(log_text).block(Block::default().borders(Borders::ALL).title("session"));
    f.render_widget(log, chunks[1]);

    let input = Paragraph::new(format!("> {}", app.input))
        .block(Block::default().borders(Borders::ALL).title("input"));
    f.render_widget(input, chunks[2]);

    let status =
        Paragraph::new(app.status.clone()).style(Style::default().add_modifier(Modifier::DIM));
    f.render_widget(status, chunks[3]);
}
