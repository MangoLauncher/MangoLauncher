use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    Terminal,
};
use std::io;

mod app;
mod ui;

use app::{App, AppState, Focus};

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let app = App::new();
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;
        handle_events(&mut app)?;
        if app.should_quit {
            break;
        }
        app.update_motd();
        app.rotate_art();
    }
    Ok(())
}

fn handle_events(app: &mut App) -> Result<()> {
    if event::poll(std::time::Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }

            handle_key_event(key, app)?;
        }
    }
    Ok(())
}

fn handle_key_event(key_event: KeyEvent, app: &mut App) -> io::Result<()> {
    if app.current_state == AppState::ProfileEdit {
        match key_event.code {
            KeyCode::Esc => handle_escape(app),
            KeyCode::Char(c) => {
                if let Some(profile_name) = &app.current_profile {
                    if let Some(profile) = app.profiles.get_mut(profile_name) {
                        profile.username.push(c);
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(profile_name) = &app.current_profile {
                    if let Some(profile) = app.profiles.get_mut(profile_name) {
                        profile.username.pop();
                    }
                }
            }
            _ => {}
        }
        return Ok(());
    }

    match key_event.code {
        KeyCode::Char('l') | KeyCode::Char('L') => {
            app.toggle_language();
        }
        KeyCode::Left => {
            if app.current_state == AppState::Settings {
                app.adjust_left_panel(false);
            }
        }
        KeyCode::Right => {
            if app.current_state == AppState::Settings {
                app.adjust_left_panel(true);
            }
        }
        KeyCode::Tab => {
            if app.current_state == AppState::VersionSelect {
                app.version_manager.toggle_view();
                app.state.select(Some(0));
            } else {
                app.toggle_focus();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::Enter => handle_enter(app),
        KeyCode::Esc => handle_escape(app),
        _ => {}
    }
    Ok(())
}

fn handle_escape(app: &mut App) {
    match app.current_state {
        AppState::MainMenu => {
            app.should_quit = true;
        }
        AppState::ProfileEdit => {
            // Сохраняем изменения при выходе из редактирования профиля
            app.save_profile();
            app.current_state = AppState::ProfileSelect;
            app.focus = Focus::Menu;
        }
        AppState::VersionSelect | AppState::ProfileSelect | AppState::Settings | AppState::Changelog => {
            app.current_state = AppState::MainMenu;
            app.state.select(Some(0));
            app.focus = Focus::Menu;
        }
    }
}

fn handle_enter(app: &mut App) {
    match app.current_state {
        AppState::MainMenu => {
            if let Some(selected) = app.state.selected() {
                match selected {
                    0 => app.current_state = AppState::VersionSelect,
                    1 => app.current_state = AppState::ProfileSelect,
                    2 => app.current_state = AppState::Settings,
                    3 => app.current_state = AppState::Changelog,
                    4 => {
                        // Запуск игры
                        if let Some(profile) = app.current_profile.as_ref() {
                            if let Some(profile) = app.profiles.get(profile) {
                                if let Some(version) = &profile.selected_version {
                                    println!("Launching Minecraft {} with profile {}", version, profile.name);
                                }
                            }
                        }
                    }
                    _ => {}
                }
                app.state.select(Some(0));
            }
        }
        AppState::VersionSelect => {
            if let Some(selected) = app.state.selected() {
                let versions = app.version_manager.get_current_versions();
                if let Some(version) = versions.get(selected) {
                    if let Some(profile) = app.current_profile.as_ref() {
                        if let Some(profile) = app.profiles.get_mut(profile) {
                            profile.selected_version = Some(version.id.clone());
                            // Отмечаем версию как использованную
                            tokio::spawn(async move {
                                if let Err(e) = app.version_manager.mark_version_used(version.id.clone()).await {
                                    eprintln!("Failed to mark version as used: {}", e);
                                }
                            });
                            app.current_state = AppState::MainMenu;
                        }
                    }
                }
            }
        }
        AppState::ProfileSelect => {
            if let Some(selected) = app.state.selected() {
                if let Some((name, _)) = app.profiles.iter().nth(selected) {
                    app.current_profile = Some(name.clone());
                    app.current_state = AppState::ProfileEdit;
                    app.focus = Focus::Input;
                }
            }
        }
        _ => {}
    }
} 