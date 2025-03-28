use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Alignment,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame,
};

use crate::app::{App, AppState, Language};

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.size());

    // Title
    let title = Paragraph::new("Mango Launcher")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White)),
        );
    f.render_widget(title, chunks[0]);

    // Main content
    match app.current_state {
        AppState::MainMenu => draw_main_menu(f, app, chunks[1]),
        AppState::VersionSelect => draw_version_select(f, app, chunks[1]),
        AppState::ProfileEdit => draw_profile_edit(f, app, chunks[1]),
        AppState::Changelog => draw_changelog(f, app, chunks[1]),
    }

    // Footer with controls
    let controls = match app.current_state {
        AppState::MainMenu => {
            if app.language == Language::Russian {
                "↑↓: Навигация | Enter: Выбрать | L: Язык | Q: Выход"
            } else {
                "↑↓: Navigate | Enter: Select | L: Language | Q: Quit"
            }
        }
        AppState::VersionSelect => {
            if app.language == Language::Russian {
                "↑↓: Навигация | Enter: Установить | Backspace: Назад | Q: Выход"
            } else {
                "↑↓: Navigate | Enter: Install | Backspace: Back | Q: Quit"
            }
        }
        AppState::ProfileEdit => {
            if app.language == Language::Russian {
                "Enter: Сохранить | Backspace: Назад | Q: Выход"
            } else {
                "Enter: Save | Backspace: Back | Q: Quit"
            }
        }
        AppState::Changelog => {
            if app.language == Language::Russian {
                "Backspace: Назад | Q: Выход"
            } else {
                "Backspace: Back | Q: Quit"
            }
        }
    };

    let footer = Paragraph::new(controls)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
}

fn draw_main_menu(f: &mut Frame, app: &App, area: Rect) {
    let menu_items = if app.language == Language::Russian {
        vec![
            "Выбрать версию",
            "Профиль",
            "Чейнджлог",
        ]
    } else {
        vec![
            "Select Version",
            "Profile",
            "Changelog",
        ]
    };

    let items: Vec<ListItem> = menu_items
        .iter()
        .map(|&i| ListItem::new(i.to_string()))
        .collect();

    let menu = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(if app.language == Language::Russian {
            "Главное меню"
        } else {
            "Main Menu"
        }))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    f.render_stateful_widget(menu, area, &mut app.state);
}

fn draw_version_select(f: &mut Frame, app: &App, area: Rect) {
    let versions: Vec<ListItem> = app
        .versions
        .iter()
        .map(|v| {
            ListItem::new(format!("{} ({})", v.id, v.r#type))
                .style(Style::default().fg(Color::White))
        })
        .collect();

    let versions = List::new(versions)
        .block(Block::default().borders(Borders::ALL).title(if app.language == Language::Russian {
            "Версии"
        } else {
            "Versions"
        }))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    f.render_stateful_widget(versions, area, &mut app.state);
}

fn draw_profile_edit(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3)])
        .split(area);

    let username = Paragraph::new(format!(
        "{}: {}",
        if app.language == Language::Russian {
            "Имя пользователя"
        } else {
            "Username"
        },
        app.username
    ))
    .block(Block::default().borders(Borders::ALL));

    f.render_widget(username, chunks[0]);
}

fn draw_changelog(f: &mut Frame, app: &App, area: Rect) {
    let changelog = if app.language == Language::Russian {
        vec![
            "v0.1.0:",
            "  - Базовый интерфейс",
            "  - Поддержка русского и английского языков",
            "  - Навигация по меню",
        ]
    } else {
        vec![
            "v0.1.0:",
            "  - Basic interface",
            "  - Russian and English language support",
            "  - Menu navigation",
        ]
    };

    let changelog = Paragraph::new(changelog.join("\n"))
        .block(Block::default().borders(Borders::ALL).title(if app.language == Language::Russian {
            "История изменений"
        } else {
            "Changelog"
        }));

    f.render_widget(changelog, area);
}