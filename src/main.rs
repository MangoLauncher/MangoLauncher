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

use app::{App, AppState};

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
                KeyCode::Enter => handle_enter(app),
                KeyCode::Backspace => handle_backspace(app),
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
                    1 => app.current_state = AppState::ProfileEdit,
                    2 => app.current_state = AppState::Changelog,
                    _ => {}
                }
                app.state.select(Some(0));
            }
        }
        AppState::VersionSelect => {
            // TODO: Implement version installation
        }
        AppState::ProfileEdit => {
            // TODO: Save profile
            app.current_state = AppState::MainMenu;
        }
        AppState::Changelog => {}
    }
}

fn handle_backspace(app: &mut App) {
    match app.current_state {
        AppState::MainMenu => {}
        AppState::VersionSelect | AppState::ProfileEdit | AppState::Changelog => {
            app.current_state = AppState::MainMenu;
            app.state.select(Some(0));
        }
    }
}

fn handle_char_input(app: &mut App, c: char) {
    if app.current_state == AppState::ProfileEdit {
        app.username.push(c);
    }
} 