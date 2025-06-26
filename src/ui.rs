
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Alignment,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, ListState},
    Frame,
};
use std::io::stdout;
use ratatui::prelude::*;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use chrono::Utc;

use crate::app::{App, AppState};
use crate::settings::Language;

use crate::Result;

const MANGO_ART: [&str; 8] = [
    "  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░",
    "      ███╗   ███╗ ██████╗ ███╗   ██╗ ██████╗  ██████╗ ",
    "      ████╗ ████║██╔══██╗████╗  ██║██╔════╝ ██╔═══██╗",
    "      ██╔████╔██║███████║██╔██╗ ██║██║  ███╗██║   ██║",
    "      ██║╚██╔╝██║██╔══██║██║╚██╗██║██║   ██║██║   ██║",
    "      ██║ ╚═╝ ██║██║  ██║██║ ╚████║╚██████╔╝╚██████╔╝",
    "      ╚═╝     ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝ ╚═════╝  ╚═════╝ ",
    "  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░",
];

pub async fn run_ui(mut app: App) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut list_state = ListState::default();
    list_state.select(Some(0));

    loop {
        terminal.draw(|f| draw(f, &mut app, &mut list_state))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    match app.state {
                        AppState::MainMenu => break,
                        AppState::EditInstance => {
                            app.cancel_instance_editing();
                            app.current_state = "Редактирование отменено".to_string();
                            list_state.select(Some(0));
                        }
                        _ => {
                            app.state = AppState::MainMenu;
                            list_state.select(Some(0));
                        }
                    }
                }
                KeyCode::Down => {
                    let max_items = match app.state {
                        AppState::MainMenu => 3,
                        AppState::InstanceList => {
                            let instances = app.instance_manager.list_instances().len();
                            if instances == 0 { 0 } else { instances.saturating_sub(1) }
                        },
                        AppState::EditInstance => 10,
                        AppState::Settings => 7, 
                        AppState::Launcher => {
                            let versions = app.get_displayed_versions().len();
                            if versions == 0 { 0 } else { versions.saturating_sub(1) }
                        },
                        AppState::AccountManager => {
                            let accounts = app.auth_manager.list_accounts().len();
                            if accounts == 0 { 0 } else { accounts.saturating_sub(1) }
                        },
                    };
                    if let Some(selected) = list_state.selected() {
                        if selected < max_items {
                            list_state.select(Some(selected + 1));
                        }
                    }
                }
                KeyCode::Up => {
                    if let Some(selected) = list_state.selected() {
                        if selected > 0 {
                            list_state.select(Some(selected - 1));
                        }
                    }
                }
                KeyCode::Enter => {
                    if let Some(selected) = list_state.selected() {
                        match app.state {
                            AppState::MainMenu => {
                                match selected {
                                    0 => app.state = AppState::InstanceList,
                                    1 => app.state = AppState::Settings,
                                    2 => app.state = AppState::Launcher,
                                    3 => app.state = AppState::AccountManager,
                                    _ => {}
                                }
                                list_state.select(Some(0));
                            }
                            AppState::InstanceList => {
                                let instances = app.instance_manager.list_instances();
                                if let Some(instance) = instances.get(selected) {
                                    app.current_state = format!("Запуск {}...", instance.name);
                                    if let Err(e) = app.launch_instance(instance.id).await {
                                        app.current_state = format!("Ошибка запуска: {}", e);
                                    }
                                }
                            }
                            AppState::EditInstance => {
                                let versions = app.version_manager.get_installed_versions();
                                let java_installations: Vec<_> = app.get_java_installations().values().cloned().collect();
                                
                                if let Some(instance) = app.get_editing_instance_mut() {
                                    match selected {
                                        0 => {
                                            let new_name = format!("Экземпляр_{}", Utc::now().format("%H%M%S"));
                                            instance.name = new_name.clone();
                                            app.current_state = format!("Название изменено на: {}", new_name);
                                        }
                                        1 => {
                                            if !versions.is_empty() {
                                                let current_index = versions.iter()
                                                    .position(|v| v.id == instance.minecraft_version)
                                                    .unwrap_or(0);
                                                let next_index = (current_index + 1) % versions.len();
                                                instance.minecraft_version = versions[next_index].id.clone();
                                                app.current_state = format!("Версия изменена на: {}", instance.minecraft_version);
                                            } else {
                                                app.current_state = "Нет скачанных версий! Скачайте версии в лаунчере".to_string();
                                            }
                                        }
                                        2 => {
                                            use crate::instance::ModLoader;
                                            instance.mod_loader = match &instance.mod_loader {
                                                None => Some(ModLoader::Fabric),
                                                Some(ModLoader::Fabric) => Some(ModLoader::Forge),
                                                Some(ModLoader::Forge) => Some(ModLoader::Quilt),
                                                Some(ModLoader::Quilt) => Some(ModLoader::NeoForge),
                                                Some(ModLoader::NeoForge) => None,
                                            };
                                            let loader_name = instance.mod_loader.as_ref()
                                                .map(|ml| format!("{:?}", ml))
                                                .unwrap_or_else(|| "Нет".to_string());
                                            app.current_state = format!("Модлоадер: {}", loader_name);
                                        }
                                        3 => {
                                            let versions = ["latest", "recommended", "1.0.0", "0.15.11", "47.2.0"];
                                            let current = instance.mod_loader_version.as_deref().unwrap_or("latest");
                                            let current_index = versions.iter().position(|&v| v == current).unwrap_or(0);
                                            let next_index = (current_index + 1) % versions.len();
                                            instance.mod_loader_version = Some(versions[next_index].to_string());
                                            app.current_state = format!("Версия модлоадера: {}", versions[next_index]);
                                        }
                                        4 => {
                                            if !java_installations.is_empty() {
                                                let current_path = instance.java_path.as_ref();
                                                let current_index = current_path
                                                    .and_then(|cp| java_installations.iter().position(|j| &j.path == cp))
                                                    .unwrap_or(0);
                                                let next_index = (current_index + 1) % (java_installations.len() + 1);
                                                
                                                if next_index == java_installations.len() {
                                                    instance.java_path = None;
                                                    app.current_state = "Java: По умолчанию".to_string();
                                                } else {
                                                    instance.java_path = Some(java_installations[next_index].path.clone());
                                                    app.current_state = format!("Java: {} {}", 
                                                        java_installations[next_index].vendor, 
                                                        java_installations[next_index].version);
                                                }
                                            } else {
                                                app.current_state = "Запустите автопоиск Java в настройках (J)".to_string();
                                            }
                                        }
                                        5 => {
                                            let args_options = [
                                                "По умолчанию",
                                                "-XX:+UseG1GC",
                                                "-XX:+UseZGC", 
                                                "-XX:+UseParallelGC",
                                                "-Xmx4G -XX:+UseG1GC -XX:+UnlockExperimentalVMOptions"
                                            ];
                                            let current = instance.java_args.as_deref().unwrap_or("По умолчанию");
                                            let current_index = args_options.iter().position(|&v| v == current).unwrap_or(0);
                                            let next_index = (current_index + 1) % args_options.len();
                                            
                                            if args_options[next_index] == "По умолчанию" {
                                                instance.java_args = None;
                                            } else {
                                                instance.java_args = Some(args_options[next_index].to_string());
                                            }
                                            app.current_state = format!("Аргументы Java: {}", args_options[next_index]);
                                        }
                                        6 => {
                                            let memory_options = [512, 1024, 2048, 4096, 6144, 8192];
                                            let current = instance.memory_min.unwrap_or(1024);
                                            let current_index = memory_options.iter().position(|&v| v == current).unwrap_or(1);
                                            let next_index = (current_index + 1) % memory_options.len();
                                            instance.memory_min = Some(memory_options[next_index]);
                                            app.current_state = format!("Минимальная память: {} MB", memory_options[next_index]);
                                        }
                                        7 => {
                                            let memory_options = [1024, 2048, 4096, 6144, 8192, 12288, 16384];
                                            let current = instance.memory_max.unwrap_or(4096);
                                            let current_index = memory_options.iter().position(|&v| v == current).unwrap_or(2);
                                            let next_index = (current_index + 1) % memory_options.len();
                                            instance.memory_max = Some(memory_options[next_index]);
                                            app.current_state = format!("Максимальная память: {} MB", memory_options[next_index]);
                                        }
                                        8 => {
                                            let resolutions = [(854, 480), (1280, 720), (1920, 1080), (2560, 1440), (3840, 2160)];
                                            let current = (instance.width.unwrap_or(854), instance.height.unwrap_or(480));
                                            let current_index = resolutions.iter().position(|&v| v == current).unwrap_or(0);
                                            let next_index = (current_index + 1) % resolutions.len();
                                            let (new_width, new_height) = resolutions[next_index];
                                            instance.width = Some(new_width);
                                            instance.height = Some(new_height);
                                            app.current_state = format!("Разрешение: {}x{}", new_width, new_height);
                                        }
                                        9 => {
                                            instance.fullscreen = !instance.fullscreen;
                                            app.current_state = format!("Полноэкранный режим: {}", 
                                                if instance.fullscreen { "Включен" } else { "Отключен" });
                                        }
                                        10 => {
                                            let groups = ["Нет", "Модпаки", "Ванилла", "Снапшоты", "Тестирование"];
                                            let current = instance.group.as_deref().unwrap_or("Нет");
                                            let current_index = groups.iter().position(|&v| v == current).unwrap_or(0);
                                            let next_index = (current_index + 1) % groups.len();
                                            
                                            if groups[next_index] == "Нет" {
                                                instance.group = None;
                                            } else {
                                                instance.group = Some(groups[next_index].to_string());
                                            }
                                            app.current_state = format!("Группа: {}", groups[next_index]);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            AppState::Settings => {
                                match selected {
                                    0 => {
                                        app.language = if app.language == Language::Russian {
                                            Language::English
                                        } else {
                                            Language::Russian
                                        };
                                        app.current_state = "Язык изменен".to_string();
                                    }
                                    2 => {
                                        let settings = app.get_settings_mut();
                                        if settings.java.memory_min >= settings.java.memory_max {
                                            settings.java.memory_max = ((settings.java.memory_max + 1024) % 16384).max(2048);
                                            app.current_state = format!("Максимальная память: {}MB", settings.java.memory_max);
                                        } else {
                                            settings.java.memory_min = ((settings.java.memory_min + 512) % 8192).max(512);
                                            app.current_state = format!("Минимальная память: {}MB", settings.java.memory_min);
                                        }
                                    }
                                    3 => {
                                        let java_dirs = vec![
                                            "/usr/bin".to_string(),
                                            "/Library/Java/JavaVirtualMachines".to_string(),
                                            "/opt/homebrew/opt".to_string(),
                                            "/System/Library/Frameworks/JavaVM.framework/Versions".to_string(),
                                        ];
                                        let settings = app.get_settings_mut();
                                        let current_dir = settings.general.java_directory.to_string_lossy().to_string();
                                        let current_index = java_dirs.iter().position(|d| current_dir.contains(d)).unwrap_or(0);
                                        let next_index = (current_index + 1) % java_dirs.len();
                                        settings.general.java_directory = std::path::PathBuf::from(&java_dirs[next_index]);
                                        app.current_state = format!("Java директория изменена, сканирую...");
                                        let _ = app.save_settings();
                                        if let Err(e) = app.scan_java_installations().await {
                                            app.current_state = format!("Ошибка сканирования Java: {}", e);
                                        } else {
                                            let count = app.get_java_installations().len();
                                            app.current_state = format!("Java директория: {} (найдено {})", java_dirs[next_index], count);
                                        }
                                    }
                                    5 => {
                                        let thread_options = [1, 2, 3, 4, 6, 8, 12, 16];
                                        let settings = app.get_settings_mut();
                                        let current = settings.network.max_concurrent_downloads;
                                        let current_index = thread_options.iter().position(|&t| t == current).unwrap_or(3);
                                        let next_index = (current_index + 1) % thread_options.len();
                                        settings.network.max_concurrent_downloads = thread_options[next_index];
                                        let _ = app.save_settings();
                                        app.update_network_settings();
                                        app.current_state = format!("Потоки загрузки: {}", thread_options[next_index]);
                                    }
                                    6 => {
                                        let new_value = {
                                            let settings = app.get_settings_mut();
                                            settings.advanced.save_logs_to_file = !settings.advanced.save_logs_to_file;
                                            settings.advanced.save_logs_to_file
                                        };
                                        let _ = app.save_settings();
                                        app.update_file_logging();
                                        app.current_state = format!("Сохранение логов: {}", 
                                            if new_value { "Включено" } else { "Отключено" });
                                    }
                                    _ => {}
                                }
                            }
                            AppState::AccountManager => {
                                let accounts = app.get_accounts();
                                if let Some(account) = accounts.get(selected) {
                                    let account_id = account.id;
                                    match app.set_default_account(account_id) {
                                        Ok(_) => {
                                            app.current_state = "Аккаунт установлен как основной".to_string();
                                        },
                                        Err(e) => {
                                            app.current_state = format!("Ошибка: {}", e);
                                        }
                                    }
                                }
                            }
                            AppState::Launcher => {
                                let versions = app.get_displayed_versions();
                                if let Some(version) = versions.get(selected) {
                                    let version_id = version.id.clone();
                                    if app.show_installed_only {
                                        app.current_state = format!("Версия {} уже скачана", version_id);
                                    } else {
                                        app.current_state = format!("Загрузка версии {}...", version_id);
                                        if let Err(e) = app.download_version(&version_id).await {
                                            app.current_state = format!("Ошибка загрузки: {}", e);
                                        } else {
                                            app.current_state = format!("Версия {} загружена!", version_id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                KeyCode::Char('n') => {
                    match app.state {
                        AppState::InstanceList => {
                            let name = format!("Экземпляр {}", Utc::now().format("%H-%M-%S"));
                            match app.create_instance(name.clone(), "1.21".to_string()) {
                                Ok(_) => {
                                    app.current_state = format!("Создан экземпляр: {}", name);
                                },
                                Err(e) => {
                                    app.current_state = format!("Ошибка создания: {}", e);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                KeyCode::Char('d') => {
                    match app.state {
                        AppState::InstanceList => {
                            if let Some(selected) = list_state.selected() {
                                let instances = app.instance_manager.list_instances();
                                if let Some(instance) = instances.get(selected) {
                                    let instance_id = instance.id;
                                    match app.delete_instance(instance_id) {
                                        Ok(_) => {
                                            app.current_state = "Экземпляр удален".to_string();
                                            let remaining = app.instance_manager.list_instances().len();
                                            if remaining == 0 {
                                                list_state.select(Some(0));
                                            } else if selected >= remaining {
                                                list_state.select(Some(remaining.saturating_sub(1)));
                                            }
                                        },
                                        Err(e) => {
                                            app.current_state = format!("Ошибка удаления: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        AppState::AccountManager => {
                            if let Some(selected) = list_state.selected() {
                                let accounts = app.auth_manager.list_accounts();
                                if let Some(account) = accounts.get(selected) {
                                    let account_id = account.id;
                                    match app.remove_account(account_id) {
                                        Ok(_) => {
                                            app.current_state = "Аккаунт удален".to_string();
                                            let remaining = app.auth_manager.list_accounts().len();
                                            if remaining == 0 {
                                                list_state.select(Some(0));
                                            } else if selected >= remaining {
                                                list_state.select(Some(remaining.saturating_sub(1)));
                                            }
                                        },
                                        Err(e) => {
                                            app.current_state = format!("Ошибка удаления: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                KeyCode::Char('s') => {
                    match app.state {
                        AppState::AccountManager => {
                            if let Some(selected) = list_state.selected() {
                                let accounts = app.auth_manager.list_accounts();
                                if let Some(account) = accounts.get(selected) {
                                    let account_id = account.id;
                                    match app.set_default_account(account_id) {
                                        Ok(_) => {
                                            app.current_state = "Аккаунт установлен как основной".to_string();
                                        },
                                        Err(e) => {
                                            app.current_state = format!("Ошибка: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        AppState::EditInstance => {
                            match app.save_instance_changes() {
                                Ok(_) => {
                                    app.state = AppState::InstanceList;
                                    app.current_state = "Изменения сохранены".to_string();
                                    list_state.select(Some(0));
                                },
                                Err(e) => {
                                    app.current_state = format!("Ошибка сохранения: {}", e);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                KeyCode::Char('r') => {
                    match app.state {
                        AppState::Launcher => {
                            app.current_state = "Обновление списка версий...".to_string();
                            if let Err(e) = app.init().await {
                                app.current_state = format!("Ошибка обновления: {}", e);
                            } else {
                                app.current_state = "Список версий обновлен!".to_string();
                            }
                        }
                        _ => {}
                    }
                }
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    match app.state {
                        AppState::Launcher => {
                            app.current_state = "Принудительное обновление списка версий...".to_string();
                            if let Err(e) = app.force_refresh_versions().await {
                                app.current_state = format!("Ошибка принудительного обновления: {}", e);
                            } else {
                                app.current_state = "Список версий принудительно обновлен!".to_string();
                            }
                        }
                        _ => {}
                    }
                }
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    app.toggle_logs();
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    app.state = AppState::AccountManager;
                }
                KeyCode::Char('o') => {
                    match app.state {
                        AppState::AccountManager => {
                            let username = format!("Player_{}", Utc::now().format("%H%M%S"));
                            match app.add_offline_account(username.clone()) {
                                Ok(_) => {
                                    app.current_state = format!("Добавлен offline аккаунт: {}", username);
                                },
                                Err(e) => {
                                    app.current_state = format!("Ошибка добавления: {}", e);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    match app.state {
                        AppState::InstanceList => {
                            if let Some(selected) = list_state.selected() {
                                let instances = app.instance_manager.list_instances();
                                if let Some(instance) = instances.get(selected) {
                                    let instance_id = instance.id;
                                    let instance_name = instance.name.clone();
                                    match app.start_editing_instance(instance_id) {
                                        Ok(_) => {
                                            app.current_state = format!("Редактирование экземпляра '{}'", instance_name);
                                            list_state.select(Some(0));
                                        },
                                        Err(e) => {
                                            app.current_state = format!("Ошибка: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                KeyCode::Char('j') | KeyCode::Char('J') => {
                    match app.state {
                        AppState::Settings => {
                            app.current_state = "Сканирование Java...".to_string();
                            if let Err(e) = app.scan_java_installations().await {
                                app.current_state = format!("Ошибка сканирования Java: {}", e);
                            } else {
                                let count = app.get_java_installations().len();
                                app.current_state = format!("Найдено {} установок Java", count);
                            }
                        }
                        _ => {}
                    }
                }
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    match app.state {
                        AppState::Launcher => {
                            app.toggle_version_mode();
                            list_state.select(Some(0));
                        }
                        _ => {}
                    }
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    match app.state {
                        AppState::AccountManager => {
                            if let Some(selected) = list_state.selected() {
                                let accounts = app.auth_manager.list_accounts();
                                if let Some(account) = accounts.get(selected) {
                                    let account_id = account.id;
                                    let new_name = format!("Player_{}", Utc::now().format("%H%M%S"));
                                    match app.change_account_name(account_id, new_name.clone()) {
                                        Ok(_) => {
                                            app.current_state = format!("Ник изменен на: {}", new_name);
                                        },
                                        Err(e) => {
                                            app.current_state = format!("Ошибка изменения ника: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

pub fn draw(f: &mut Frame, app: &mut App, list_state: &mut ListState) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(2, 3),
        ])
        .split(f.size());

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(main_chunks[0]);

    if app.show_logs {
        draw_logs_panel(f, app, left_chunks[0]);
        
        let toggle_hint = Paragraph::new("L: Переключить логи")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::NONE));
        f.render_widget(toggle_hint, left_chunks[1]);
    } else {
    let art = Paragraph::new(MANGO_ART.join("\n"))
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(art, left_chunks[0]);

        let motd_with_toggle = format!("{}\n\nL: Показать логи", app.current_motd);
        let motd = Paragraph::new(motd_with_toggle)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(motd, left_chunks[1]);
    }

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(main_chunks[1]);

    match app.state {
        AppState::MainMenu => draw_main_menu(f, app, right_chunks[0], list_state),
        AppState::InstanceList => draw_instance_list(f, app, right_chunks[0], list_state),
        AppState::Settings => draw_settings(f, app, right_chunks[0], list_state),
        AppState::Launcher => draw_launcher(f, app, right_chunks[0], list_state),
        AppState::AccountManager => draw_account_manager(f, app, right_chunks[0], list_state),
        AppState::EditInstance => draw_edit_instance(f, app, right_chunks[0], list_state),
    }

    let controls = match app.state {
        AppState::MainMenu => {
            if app.language == Language::Russian {
                "↑↓: Навигация | Enter: Выбрать | Esc: Выход"
            } else {
                "↑↓: Navigate | Enter: Select | Esc: Exit"
            }
        }
        AppState::InstanceList => {
            if app.language == Language::Russian {
                "↑↓: Навигация | Enter: Запустить | E: Изменить | N: Создать | D: Удалить | Esc: Назад"
            } else {
                "↑↓: Navigate | Enter: Launch | E: Edit | N: Create | D: Delete | Esc: Back"
            }
        }
        AppState::Settings => {
            if app.language == Language::Russian {
                "↑↓: Навигация | Enter: Изменить | J: Найти Java | Esc: Назад"
            } else {
                "↑↓: Navigate | Enter: Change | J: Find Java | Esc: Back"
            }
        }
        AppState::Launcher => {
            if app.language == Language::Russian {
                if app.show_installed_only {
                    "↑↓: Навигация | T: Все версии | R: Обновить | F: Принуд. обн. | Esc: Назад"
                } else {
                    "↑↓: Навигация | Enter: Скачать | T: Скачанные | R: Обновить | F: Принуд. | Esc: Назад"
                }
            } else {
                if app.show_installed_only {
                    "↑↓: Navigate | T: All Versions | R: Refresh | F: Force | Esc: Back"
                } else {
                    "↑↓: Navigate | Enter: Download | T: Downloaded | R: Refresh | F: Force | Esc: Back"
                }
            }
        }
        AppState::AccountManager => {
            if app.language == Language::Russian {
                "↑↓: Навигация | Enter: Выбрать | S: Установить | C: Изменить ник | O: Добавить | D: Удалить | Esc: Назад"
            } else {
                "↑↓: Navigate | Enter: Select | S: Set Default | C: Change Name | O: Add Offline | D: Delete | Esc: Back"
            }
        }
        AppState::EditInstance => {
            if app.language == Language::Russian {
                "↑↓: Навигация | Enter: Изменить поле | S: Сохранить | Esc: Отмена"
            } else {
                "↑↓: Navigate | Enter: Cycle Field | S: Save | Esc: Cancel"
            }
        }
    };

    let footer = Paragraph::new(controls)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, right_chunks[1]);
}

fn draw_main_menu(f: &mut Frame, app: &mut App, area: Rect, list_state: &mut ListState) {
    let menu_items = if app.language == Language::Russian {
        vec![
            "Экземпляры игры",
            "Настройки",
            "Лаунчер",
            "Аккаунты",
        ]
    } else {
        vec![
            "Game Instances",
            "Settings",
            "Launcher",
            "Accounts",
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

    f.render_stateful_widget(menu, area, list_state);
}

fn draw_instance_list(f: &mut Frame, app: &mut App, area: Rect, list_state: &mut ListState) {
    let instances = app.instance_manager.list_instances();
    
    if instances.is_empty() {
        let empty_message = if app.language == Language::Russian {
            "Нет экземпляров игры.\nНажмите 'N' для создания нового экземпляра."
        } else {
            "No game instances.\nPress 'N' to create a new instance."
        };

        let empty_paragraph = Paragraph::new(empty_message)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default()
                .title(if app.language == Language::Russian {
                    "Экземпляры игры"
            } else {
                    "Game Instances"
                })
                .borders(Borders::ALL));

        f.render_widget(empty_paragraph, area);
    } else {
        let items: Vec<ListItem> = instances
            .iter()
            .map(|instance| {
                ListItem::new(format!("{} (v{})", instance.name, instance.minecraft_version))
                    .style(Style::default().fg(Color::White))
        })
        .collect();

        let instances_list = List::new(items)
            .block(Block::default()
                .title(if app.language == Language::Russian {
                    format!("Экземпляры игры ({})", instances.len())
            } else {
                    format!("Game Instances ({})", instances.len())
                })
                .borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        f.render_stateful_widget(instances_list, area, list_state);
    }
}

fn draw_settings(f: &mut Frame, app: &App, area: Rect, list_state: &mut ListState) {
    let settings_items = if app.language == Language::Russian {
        vec![
            format!("Язык: {}", match app.language {
                Language::Russian => "Русский",
                Language::English => "English",
            }),
            format!("Статус: {}", app.current_state),
            format!("Память: {}MB - {}MB", 
                app.get_settings().java.memory_min,
                app.get_settings().java.memory_max
            ),
            format!("Java директория: {}", 
                app.get_settings().general.java_directory.display()
            ),
            format!("Директория экземпляров: {}", 
                app.get_settings().general.instances_directory.display()
            ),
            format!("Потоки загрузки: {}", 
                app.get_settings().network.max_concurrent_downloads
            ),
            format!("Сохранение логов: {}", 
                if app.get_settings().advanced.save_logs_to_file { "Включено" } else { "Отключено" }
            ),
            format!("Директория логов: {}", 
                app.get_settings().advanced.logs_directory.display()
            ),
        ]
            } else {
        vec![
            format!("Language: {}", match app.language {
                Language::Russian => "Русский",
                Language::English => "English",
            }),
            format!("Status: {}", app.current_state),
            format!("Memory: {}MB - {}MB", 
                app.get_settings().java.memory_min,
                app.get_settings().java.memory_max
            ),
            format!("Java directory: {}", 
                app.get_settings().general.java_directory.display()
            ),
            format!("Instances directory: {}", 
                app.get_settings().general.instances_directory.display()
            ),
            format!("Download threads: {}", 
                app.get_settings().network.max_concurrent_downloads
            ),
            format!("Save logs: {}", 
                if app.get_settings().advanced.save_logs_to_file { "Enabled" } else { "Disabled" }
            ),
            format!("Logs directory: {}", 
                app.get_settings().advanced.logs_directory.display()
            ),
        ]
    };

    let items: Vec<ListItem> = settings_items
        .iter()
        .map(|item| {
            ListItem::new(item.clone())
                .style(Style::default().fg(Color::White))
        })
        .collect();

    let settings_list = List::new(items)
        .block(Block::default()
            .title(if app.language == Language::Russian {
                "Настройки"
        } else {
                "Settings"
            })
            .borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    f.render_stateful_widget(settings_list, area, list_state);
}

fn draw_launcher(f: &mut Frame, app: &App, area: Rect, list_state: &mut ListState) {
    let versions = app.get_displayed_versions();
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    if versions.is_empty() {
        let empty_message = if app.show_installed_only {
            if app.language == Language::Russian {
                "Нет скачанных версий.\nНажмите 'T' для переключения или 'R' для обновления списка."
            } else {
                "No downloaded versions.\nPress 'T' to toggle or 'R' to refresh list."
            }
        } else {
            if app.language == Language::Russian {
                "Список версий пуст.\nНажмите 'R' для обновления."
            } else {
                "Version list is empty.\nPress 'R' to refresh."
            }
        };

        let empty_paragraph = Paragraph::new(empty_message)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default()
                .title(if app.language == Language::Russian {
                    if app.show_installed_only {
                        "Скачанные версии Minecraft"
                    } else {
                        "Версии Minecraft"
                    }
                } else {
                    if app.show_installed_only {
                        "Downloaded Minecraft Versions"
                    } else {
                        "Minecraft Versions"
                    }
                })
                .borders(Borders::ALL));

        f.render_widget(empty_paragraph, chunks[0]);
    } else {
        let items: Vec<ListItem> = versions
            .iter()
            .take(20)
            .map(|version| {
                let is_installed = app.version_manager.is_version_installed(&version.id);
                let installed_marker = if is_installed { " ✓" } else { "" };
                
                let version_text = format!("{}{} ({})", 
                    version.id, 
                    installed_marker,
                    version.r#type
                );
                
                let color = if is_installed {
                    Color::Green
                } else {
                    match version.r#type.as_str() {
                        "release" => Color::Yellow,
                        "snapshot" => Color::Cyan,
                        "old_beta" => Color::Blue,
                        "old_alpha" => Color::Magenta,
                        _ => Color::White,
                    }
                };
                ListItem::new(version_text).style(Style::default().fg(color))
            })
            .collect();

        let mode_text = if app.show_installed_only {
        if app.language == Language::Russian {
                "скачанных"
            } else {
                "downloaded"
            }
        } else {
            if app.language == Language::Russian {
                "доступно"
            } else {
                "available"
            }
        };

        let versions_list = List::new(items)
            .block(Block::default()
                .title(if app.language == Language::Russian {
                    if app.show_installed_only {
                        format!("Скачанные версии Minecraft ({} {})", versions.len(), mode_text)
                    } else {
                        format!("Версии Minecraft ({} {})", versions.len(), mode_text)
                    }
                } else {
                    if app.show_installed_only {
                        format!("Downloaded Minecraft Versions ({} {})", versions.len(), mode_text)
                    } else {
                        format!("Minecraft Versions ({} {})", versions.len(), mode_text)
                    }
                })
                .borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        f.render_stateful_widget(versions_list, chunks[0], list_state);
    }

    let status = Paragraph::new(format!(
        "{}: {}",
        if app.language == Language::Russian {
            "Статус"
        } else {
            "Status"
        },
        app.current_state
    ))
    .style(Style::default().fg(Color::Cyan))
    .block(Block::default().borders(Borders::ALL));

    f.render_widget(status, chunks[1]);
}

fn draw_logs_panel(f: &mut Frame, app: &App, area: Rect) {
    
    let logs = app.log_manager.get_recent_entries(50);
    
    if logs.is_empty() {
        let empty_message = "Логи пусты\nСобытия будут отображаться здесь";
        let empty_paragraph = Paragraph::new(empty_message)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default()
                .title("Логи лаунчера")
                .borders(Borders::ALL));
        f.render_widget(empty_paragraph, area);
        return;
    }

    let log_items: Vec<ListItem> = logs
        .iter()
        .map(|entry| {
            let time_str = entry.timestamp.format("%H:%M:%S").to_string();
            let source_str = entry.source.as_ref()
                .map(|s| format!("[{}]", s))
                .unwrap_or_default();
            
            let formatted = format!("{} {} {} {}", 
                time_str, 
                entry.level.as_str(), 
                source_str, 
                entry.message
            );
            
            ListItem::new(formatted)
                .style(Style::default().fg(entry.level.color()))
        })
        .collect();

    let logs_list = List::new(log_items)
        .block(Block::default()
            .title(format!("Логи лаунчера ({})", logs.len()))
            .borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    f.render_widget(logs_list, area);
}

fn draw_account_manager(f: &mut Frame, app: &App, area: Rect, list_state: &mut ListState) {
    use crate::auth::AccountType;
    
    let accounts = app.auth_manager.list_accounts();
    
    if accounts.is_empty() {
        let empty_message = if app.language == Language::Russian {
            "Нет аккаунтов.\nНажмите 'O' для создания offline аккаунта."
        } else {
            "No accounts.\nPress 'O' to create an offline account."
        };

        let empty_paragraph = Paragraph::new(empty_message)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default()
                .title(if app.language == Language::Russian {
                    "Управление аккаунтами"
                } else {
                    "Account Management"
                })
                .borders(Borders::ALL));

        f.render_widget(empty_paragraph, area);
    } else {
        let default_account = app.auth_manager.get_default_account();
        
        let items: Vec<ListItem> = accounts
            .iter()
            .map(|account| {
                let account_type_str = match account.account_type {
                    AccountType::Offline => if app.language == Language::Russian { "Offline" } else { "Offline" },
                    AccountType::Microsoft => if app.language == Language::Russian { "Microsoft" } else { "Microsoft" },
                };
                
                let is_default = default_account.map(|def| def.id == account.id).unwrap_or(false);
                let default_indicator = if is_default { " [★]" } else { "" };
                
                let display_text = format!("{} ({}){}", 
                    account.display_name, 
                    account_type_str,
                    default_indicator
                );
                
                let color = match account.account_type {
                    AccountType::Offline => Color::Cyan,
                    AccountType::Microsoft => {
                        if account.is_valid() {
                            Color::Green
                        } else {
                            Color::Yellow
                        }
                    }
                };
                
                ListItem::new(display_text)
                    .style(Style::default().fg(color))
            })
            .collect();

        let accounts_list = List::new(items)
            .block(Block::default()
                .title(if app.language == Language::Russian {
                    format!("Управление аккаунтами ({})", accounts.len())
                } else {
                    format!("Account Management ({})", accounts.len())
                })
                .borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        f.render_stateful_widget(accounts_list, area, list_state);
    }
}

fn draw_edit_instance(f: &mut Frame, app: &App, area: Rect, list_state: &mut ListState) {
    if let Some(instance) = app.get_editing_instance() {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
                Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

        let fields = vec![
            format!("Название: {} ⚡", instance.name),
            format!("Версия Minecraft: {} ⚡", instance.minecraft_version),
            format!("Модлоадер: {} ⚡", instance.mod_loader.as_ref()
                .map(|ml| format!("{:?}", ml))
                .unwrap_or_else(|| "Нет".to_string())),
            format!("Версия модлоадера: {} ⚡", instance.mod_loader_version.as_deref().unwrap_or("latest")),
            format!("Путь к Java: {} ⚡", instance.java_path.as_ref()
                .map(|p| {
            
                    p.file_name().and_then(|n| n.to_str()).unwrap_or("java")
                })
                .unwrap_or_else(|| "По умолчанию")),
            format!("Аргументы Java: {} ⚡", instance.java_args.as_deref().unwrap_or("По умолчанию")),
            format!("Память мин: {} MB ⚡", instance.memory_min.unwrap_or(1024)),
            format!("Память макс: {} MB ⚡", instance.memory_max.unwrap_or(4096)),
            format!("Разрешение: {}x{} ⚡", 
                instance.width.unwrap_or(854), 
                instance.height.unwrap_or(480)),
            format!("Полноэкранный режим: {} ⚡", if instance.fullscreen { "Да" } else { "Нет" }),
            format!("Группа: {} ⚡", instance.group.as_deref().unwrap_or("Нет")),
        ];

        let items: Vec<ListItem> = fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let style = if i < 5 {
                    Style::default().fg(Color::White)
                } else if i < 8 {
                    Style::default().fg(Color::Yellow)
        } else {
                    Style::default().fg(Color::Cyan)
                };
                ListItem::new(field.clone()).style(style)
            })
            .collect();

        let instance_settings = List::new(items)
            .block(Block::default()
                .title(if app.language == Language::Russian {
                    format!("Редактирование экземпляра: {}", instance.name)
        } else {
                    format!("Editing Instance: {}", instance.name)
                })
                .borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        f.render_stateful_widget(instance_settings, chunks[0], list_state);

            
        let help_text = if app.language == Language::Russian {
            format!(
                "Используйте Enter для циклического изменения полей\n\
                Текущая Java: {}\n\
                Не забудьте сохранить изменения клавишей S",
                if let Some(java) = app.get_default_java() {
                    format!("{} {}", java.vendor, java.version)
        } else {
                    "Не найдена (J для поиска)".to_string()
                }
            )
        } else {
            format!(
                "Use Enter to cycle through field values\n\
                Current Java: {}\n\
                Don't forget to save changes with S",
                if let Some(java) = app.get_default_java() {
                    format!("{} {}", java.vendor, java.version)
        } else {
                    "Not found (J to search)".to_string()
                }
            )
        };

        let info = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Cyan))
            .wrap(ratatui::widgets::Wrap { trim: true })
            .block(Block::default()
                .title("Справка")
                .borders(Borders::ALL));

        f.render_widget(info, chunks[1]);
    } else {
        let error_text = if app.language == Language::Russian {
            "Ошибка: экземпляр не найден"
        } else {
            "Error: instance not found"
        };

        let error_paragraph = Paragraph::new(error_text)
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center)
            .block(Block::default()
                .title("Ошибка")
                .borders(Borders::ALL));

        f.render_widget(error_paragraph, area);
    }
}