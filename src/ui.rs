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
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(f.size());

    draw_header(f, app, chunks[0]);
    draw_content(f, app, chunks[1]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(40),
            Constraint::Min(0),
        ])
        .split(area);

    let art = Paragraph::new(app.current_motd.as_str())
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(art, chunks[0]);

    let title = Paragraph::new("Mango Launcher")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Green));
    f.render_widget(title, chunks[1]);
}

fn draw_content(f: &mut Frame, app: &mut App, area: Rect) {
    match app.current_state {
        AppState::MainMenu => draw_main_menu(f, app, area),
        AppState::VersionSelect => draw_version_select(f, app, area),
        AppState::ProfileSelect => draw_profile_select(f, app, area),
        AppState::ProfileEdit => draw_profile_edit(f, app, area),
        AppState::Settings => draw_settings(f, app, area),
        AppState::Changelog => draw_changelog(f, app, area),
    }
}

fn draw_main_menu(f: &mut Frame, app: &mut App, area: Rect) {
    let items = vec![
        ListItem::new("Launch Game"),
        ListItem::new("Select Version"),
        ListItem::new("Select Profile"),
        ListItem::new("Settings"),
        ListItem::new("Changelog"),
    ];

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Main Menu"))
        .highlight_style(Style::default().fg(Color::Yellow))
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.state);
}

fn draw_version_select(f: &mut Frame, app: &App, area: Rect) {
    let versions = app.version_manager.get_current_versions();
    let items: Vec<ListItem> = versions
        .iter()
        .map(|version| {
            let title = match &version.r#type {
                VersionType::Vanilla => version.id.clone(),
                VersionType::Forge(forge_ver) => format!("{} (Forge {})", version.id, forge_ver),
                VersionType::OptiFine(opti_ver) => format!("{} (OptiFine {})", version.id, opti_ver),
                VersionType::ForgeOptiFine { forge, optifine } => {
                    format!("{} (Forge {}, OptiFine {})", version.id, forge, optifine)
                }
            };
            let status = if version.installed {
                "✓"
            } else {
                "✗"
            };
            ListItem::new(format!("{} {}", title, status))
        })
        .collect();

    let view_title = match app.version_manager.current_view {
        VersionView::Recent => "Recent Versions",
        VersionView::All => "All Versions",
        VersionView::Modded => "Modded Versions",
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(view_title))
        .highlight_style(Style::default().fg(Color::Yellow))
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.state.clone());
}

fn draw_profile_select(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app.profiles
        .iter()
        .map(|(name, profile)| {
            let version = profile.selected_version
                .as_ref()
                .map(|v| format!(" ({})", v))
                .unwrap_or_default();
            ListItem::new(format!("{}{}", name, version))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Select Profile"))
        .highlight_style(Style::default().fg(Color::Yellow))
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.state.clone());
}

fn draw_profile_edit(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    let username = if let Some(profile) = app.profiles.get(app.current_profile.as_deref().unwrap_or("")) {
        &profile.username
    } else {
        ""
    };

    let input = Paragraph::new(username)
        .block(Block::default().borders(Borders::ALL).title("Username"))
        .style(match app.focus {
            Focus::Input => Style::default().fg(Color::Yellow),
            _ => Style::default(),
        });

    f.render_widget(input, chunks[0]);
}

fn draw_settings(f: &mut Frame, app: &App, area: Rect) {
    let items = vec![
        ListItem::new("Language"),
        ListItem::new("Java Settings"),
        ListItem::new("Interface"),
    ];

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Settings"))
        .highlight_style(Style::default().fg(Color::Yellow))
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.state.clone());
}

fn draw_changelog(f: &mut Frame, app: &App, area: Rect) {
    let text = if app.language == "ru" {
        vec![
            "Версия 0.1.0:",
            "- Первый релиз",
            "- Поддержка ванильных версий",
            "- Система профилей",
            "- Двуязычный интерфейс",
            "",
            "Планы на будущее:",
            "- Поддержка Forge и OptiFine",
            "- Управление модами",
            "- Улучшенная производительность",
        ]
    } else {
        vec![
            "Version 0.1.0:",
            "- Initial release",
            "- Vanilla version support",
            "- Profile system",
            "- Bilingual interface",
            "",
            "Future plans:",
            "- Forge and OptiFine support",
            "- Mod management",
            "- Performance improvements",
        ]
    };

    let changelog = Paragraph::new(text.join("\n"))
        .block(Block::default().borders(Borders::ALL).title("Changelog"))
        .wrap(Wrap { trim: true });

    f.render_widget(changelog, area);
}