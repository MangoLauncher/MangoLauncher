use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    Terminal,
};

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

            match key.code {
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Char('j') | KeyCode::Down => app.next(),
                KeyCode::Char('k') | KeyCode::Up => app.previous(),
                KeyCode::Char('l') => app.toggle_language(),
                KeyCode::Tab => app.toggle_focus(),
                KeyCode::Enter => handle_enter(app),
                KeyCode::Esc => handle_escape(app),
                KeyCode::Char(c) => handle_char_input(app, c),
                _ => {}
            }
        }
    }
    Ok(())
}

fn handle_enter(app: &mut App) {
    match app.current_state {
        AppState::MainMenu => {
            if let Some(selected) = app.state.selected() {
                match selected {
                    0 => app.current_state = AppState::VersionSelect,
                    1 => app.current_state = AppState::ProfileSelect,
                    2 => app.current_state = AppState::Changelog,
                    3 => {
                        // TODO: Implement game launch
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
                if let Some(version) = app.versions.get(selected) {
                    if let Some(profile) = app.current_profile.as_ref() {
                        if let Some(profile) = app.profiles.get_mut(profile) {
                            profile.selected_version = Some(version.id.clone());
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
                }
            }
        }
        AppState::ProfileEdit => {
            app.current_state = AppState::MainMenu;
        }
        AppState::Changelog => {}
    }
}

fn handle_escape(app: &mut App) {
    match app.current_state {
        AppState::MainMenu => {}
        AppState::VersionSelect | AppState::ProfileSelect | AppState::ProfileEdit | AppState::Changelog => {
            app.current_state = AppState::MainMenu;
            app.state.select(Some(0));
            app.focus = Focus::Menu;
        }
    }
}

fn handle_char_input(app: &mut App, c: char) {
    if app.current_state == AppState::ProfileEdit && app.focus == Focus::Input {
        if let Some(profile) = app.current_profile.as_ref() {
            if let Some(profile) = app.profiles.get_mut(profile) {
                profile.username.push(c);
            }
        }
    }
} 