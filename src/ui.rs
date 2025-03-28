use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Alignment,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, AppState, Focus, Language, MANGO_ART};

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(8),  // ASCII art + MOTD
            Constraint::Min(0),     // Main content
            Constraint::Length(3),  // Controls
        ])
        .split(f.size());

    // ASCII art and MOTD
    let art_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(3),
        ])
        .split(chunks[0]);

    // ASCII art with rotation effect
    let art = Paragraph::new(MANGO_ART.join("\n"))
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(art, art_chunks[0]);

    // MOTD
    let motd = Paragraph::new(&app.current_motd)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(motd, art_chunks[1]);

    // Main content
    match app.current_state {
        AppState::MainMenu => draw_main_menu(f, app, chunks[1]),
        AppState::VersionSelect => draw_version_select(f, app, chunks[1]),
        AppState::ProfileSelect => draw_profile_select(f, app, chunks[1]),
        AppState::ProfileEdit => draw_profile_edit(f, app, chunks[1]),
        AppState::Changelog => draw_changelog(f, app, chunks[1]),
    }

    // Footer with controls
    let controls = match app.current_state {
        AppState::MainMenu => {
            if app.language == Language::Russian {
                "↑↓: Навигация | Tab: Переключение | Enter: Выбрать | L: Язык | Esc: Назад | Q: Выход"
            } else {
                "↑↓: Navigate | Tab: Switch | Enter: Select | L: Language | Esc: Back | Q: Quit"
            }
        }
        AppState::VersionSelect => {
            if app.language == Language::Russian {
                "↑↓: Навигация | Tab: Переключение | Enter: Установить | Esc: Назад | Q: Выход"
            } else {
                "↑↓: Navigate | Tab: Switch | Enter: Install | Esc: Back | Q: Quit"
            }
        }
        AppState::ProfileSelect => {
            if app.language == Language::Russian {
                "↑↓: Навигация | Tab: Переключение | Enter: Выбрать | Esc: Назад | Q: Выход"
            } else {
                "↑↓: Navigate | Tab: Switch | Enter: Select | Esc: Back | Q: Quit"
            }
        }
        AppState::ProfileEdit => {
            if app.language == Language::Russian {
                "Tab: Переключение | Enter: Сохранить | Esc: Назад | Q: Выход"
            } else {
                "Tab: Switch | Enter: Save | Esc: Back | Q: Quit"
            }
        }
        AppState::Changelog => {
            if app.language == Language::Russian {
                "Esc: Назад | Q: Выход"
            } else {
                "Esc: Back | Q: Quit"
            }
        }
    };

    let footer = Paragraph::new(controls)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
}

fn draw_main_menu(f: &mut Frame, app: &mut App, area: Rect) {
    let menu_items = if app.language == Language::Russian {
        vec![
            "Выбрать версию",
            "Профили",
            "Чейнджлог",
            "Запустить игру",
        ]
    } else {
        vec![
            "Select Version",
            "Profiles",
            "Changelog",
            "Play Game",
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

fn draw_version_select(f: &mut Frame, app: &mut App, area: Rect) {
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

fn draw_profile_select(f: &mut Frame, app: &mut App, area: Rect) {
    let profiles: Vec<ListItem> = app
        .profiles
        .iter()
        .map(|(name, profile)| {
            ListItem::new(format!("{} ({})", name, profile.username))
                .style(Style::default().fg(Color::White))
        })
        .collect();

    let profiles = List::new(profiles)
        .block(Block::default().borders(Borders::ALL).title(if app.language == Language::Russian {
            "Профили"
        } else {
            "Profiles"
        }))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    f.render_stateful_widget(profiles, area, &mut app.state);
}

fn draw_profile_edit(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(area);

    let profile = app.current_profile.as_ref().and_then(|name| app.profiles.get(name)).unwrap();

    let username = Paragraph::new(format!(
        "{}: {}",
        if app.language == Language::Russian {
            "Имя пользователя"
        } else {
            "Username"
        },
        profile.username
    ))
    .block(Block::default().borders(Borders::ALL).style(match app.focus {
        Focus::Input => Style::default().fg(Color::Yellow),
        _ => Style::default(),
    }));

    let ram = Paragraph::new(format!(
        "{}: {}",
        if app.language == Language::Russian {
            "Память"
        } else {
            "RAM"
        },
        profile.ram
    ))
    .block(Block::default().borders(Borders::ALL));

    let java_args = Paragraph::new(format!(
        "{}: {}",
        if app.language == Language::Russian {
            "Аргументы Java"
        } else {
            "Java Arguments"
        },
        profile.java_args
    ))
    .block(Block::default().borders(Borders::ALL));

    f.render_widget(username, chunks[0]);
    f.render_widget(ram, chunks[1]);
    f.render_widget(java_args, chunks[2]);
}

fn draw_changelog(f: &mut Frame, app: &App, area: Rect) {
    let changelog = if app.language == Language::Russian {
        vec![
            "v0.1.0:",
            "  - Базовый интерфейс",
            "  - Поддержка русского и английского языков",
            "  - Навигация по меню",
            "  - Улучшенное управление (Tab, стрелки)",
            "  - ASCII-арт и MOTD",
            "  - Система профилей",
        ]
    } else {
        vec![
            "v0.1.0:",
            "  - Basic interface",
            "  - Russian and English language support",
            "  - Menu navigation",
            "  - Improved controls (Tab, arrows)",
            "  - ASCII art and MOTD",
            "  - Profile system",
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