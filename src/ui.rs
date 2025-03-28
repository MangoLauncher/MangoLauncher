use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::Alignment,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::App;

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(f.size());

    let title = Paragraph::new("Mango Launcher")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White)),
        );
    f.render_widget(title, chunks[0]);

    let versions: Vec<ListItem> = app
        .versions
        .iter()
        .map(|v| {
            ListItem::new(format!("{} ({})", v.id, v.r#type))
                .style(Style::default().fg(Color::White))
        })
        .collect();

    let versions = List::new(versions)
        .block(Block::default().borders(Borders::ALL).title("Versions"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    f.render_stateful_widget(
        versions,
        chunks[1],
        &mut app.state,
    );
} 