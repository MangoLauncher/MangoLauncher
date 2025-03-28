use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Alignment,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, AppState, Focus, Language, MANGO_ART};

pub fn draw(f: &mut Frame, app: &mut App) {
    // Разделяем экран на левую и правую части
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([
            Constraint::Ratio(1, 3),  // Левая треть экрана
            Constraint::Ratio(2, 3),  // Правая часть для основного контента
        ])
        .split(f.size());

    // Левая часть - ASCII и MOTD
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),      // ASCII арт
            Constraint::Length(3),   // MOTD
        ])
        .split(main_chunks[0]);

    // ASCII арт с эффектом вращения
    let art = Paragraph::new(MANGO_ART.join("\n"))
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(art, left_chunks[0]);

    // MOTD
    let motd = Paragraph::new(app.current_motd.as_str())
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(motd, left_chunks[1]);

    // Правая часть - основной контент и футер
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),     // Основной контент
            Constraint::Length(3),  // Футер с управлением
        ])
        .split(main_chunks[1]);

    // Основной контент
    match app.current_state {
        AppState::MainMenu => draw_main_menu(f, app, right_chunks[0]),
        AppState::VersionSelect => draw_version_select(f, app, right_chunks[0]),
        AppState::ProfileSelect => draw_profile_select(f, app, right_chunks[0]),
        AppState::ProfileEdit => draw_profile_edit(f, app, right_chunks[0]),
        AppState::Settings => draw_settings(f, app, right_chunks[0]),
        AppState::Changelog => draw_changelog(f, app, right_chunks[0]),
    }

    // Footer with controls
    let controls = match app.current_state {
        AppState::MainMenu => {
            if app.settings.language == Language::Russian {
                "↑↓: Навигация | Tab: Переключение | Enter: Выбрать | L: Язык | Esc: Выход"
            } else {
                "↑↓: Navigate | Tab: Switch | Enter: Select | L: Language | Esc: Exit"
            }
        }
        AppState::VersionSelect => {
            if app.settings.language == Language::Russian {
                "↑↓: Навигация | Enter: Установить | Esc: Назад"
            } else {
                "↑↓: Navigate | Enter: Install | Esc: Back"
            }
        }
        AppState::ProfileSelect => {
            if app.settings.language == Language::Russian {
                "↑↓: Навигация | Enter: Редактировать | Esc: Назад"
            } else {
                "↑↓: Navigate | Enter: Edit | Esc: Back"
            }
        }
        AppState::ProfileEdit => {
            if app.settings.language == Language::Russian {
                "Введите имя пользователя | Esc: Сохранить и выйти"
            } else {
                "Enter username | Esc: Save and exit"
            }
        }
        AppState::Settings => {
            if app.settings.language == Language::Russian {
                "L: Язык | ←/→: Размер панели | Esc: Назад"
            } else {
                "L: Language | ←/→: Panel size | Esc: Back"
            }
        }
        AppState::Changelog => {
            if app.settings.language == Language::Russian {
                "Esc: Назад"
            } else {
                "Esc: Back"
            }
        }
    };

    let footer = Paragraph::new(controls)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, right_chunks[1]);
}

fn draw_main_menu(f: &mut Frame, app: &mut App, area: Rect) {
    let menu_items = if app.settings.language == Language::Russian {
        vec![
            "Выбрать версию",
            "Профили",
            "Настройки",
            "Чейнджлог",
            "Запустить игру",
        ]
    } else {
        vec![
            "Select Version",
            "Profiles",
            "Settings",
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

fn draw_settings(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(area);

    let language = Paragraph::new(format!(
        "{}: {} (L)",
        if app.settings.language == Language::Russian {
            "Язык"
        } else {
            "Language"
        },
        if app.settings.language == Language::Russian {
            "Русский"
        } else {
            "English"
        }
    ))
    .block(Block::default().borders(Borders::ALL));

    let panel_width = Paragraph::new(format!(
        "{}: {} ({}/{})",
        if app.settings.language == Language::Russian {
            "Ширина левой панели"
        } else {
            "Left Panel Width"
        },
        app.settings.left_panel_width,
        if app.settings.language == Language::Russian {
            "←/→"
        } else {
            "Left/Right"
        },
        if app.settings.language == Language::Russian {
            "стрелки"
        } else {
            "arrows"
        }
    ))
    .block(Block::default().borders(Borders::ALL));

    f.render_widget(language, chunks[0]);
    f.render_widget(panel_width, chunks[1]);
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