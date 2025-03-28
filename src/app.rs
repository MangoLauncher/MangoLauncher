use std::collections::{HashMap, VecDeque};
use ratatui::widgets::ListState;
use chrono::{DateTime, Local};

#[derive(Debug, Clone)]
pub struct MinecraftVersion {
    pub id: String,
    pub r#type: String,
    pub url: String,
    pub time: String,
    pub release_time: String,
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub username: String,
    pub selected_version: Option<String>,
    pub ram: String,
    pub java_args: String,
}

#[derive(PartialEq, Clone)]
pub enum Language {
    Russian,
    English,
}

#[derive(PartialEq)]
pub enum AppState {
    MainMenu,
    VersionSelect,
    ProfileSelect,
    ProfileEdit,
    Settings,
    Changelog,
}

#[derive(PartialEq)]
pub enum Focus {
    Menu,
    Input,
    List,
}

pub const MANGO_ART: &[&str] = &[
    r"    ╔══════════════════╗    ",
    r"    ║   ,,,,,,,,,,,,   ║    ",
    r"    ║ ,'          '. ║    ",
    r"    ║/    ______    \║    ",
    r"    ║    /      \    ║    ",
    r"    ║   |  MANGO |   ║    ",
    r"    ║    \      /    ║    ",
    r"    ║     '....'     ║    ",
    r"    ║   ╭──────╮   ║    ",
    r"    ║   │ 🥭  │   ║    ",
    r"    ║   ╰──────╯   ║    ",
    r"    ╚══════════════════╝    ",
    r"",
    r"     MANGO LAUNCHER     ",
    r"    ═══════════════    ",
];

pub const MOTDS: &[&str] = &[
    "Добро пожаловать в Mango Launcher! 🥭",
    "Welcome to Mango Launcher! 🥭",
    "Mango - вкусный лаунчер для Minecraft 🥭",
    "Mango - delicious Minecraft launcher 🥭",
    "Создано с любовью к Minecraft ❤️",
    "Made with love for Minecraft ❤️",
    "Сладкий как манго, быстрый как молния ⚡",
    "Sweet as mango, fast as lightning ⚡",
    "Ваш любимый лаунчер для Minecraft 🎮",
    "Your favorite Minecraft launcher 🎮",
    "Свежий как манго, надежный как камень 🪨",
    "Fresh as mango, reliable as stone 🪨",
    "Создан для геймеров, от геймеров 🎮",
    "Made by gamers, for gamers 🎮",
    "Сладкий вкус Minecraft 🍯",
    "Sweet taste of Minecraft 🍯",
];

pub struct Settings {
    pub left_panel_width: u16,
    pub language: Language,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            left_panel_width: 25,
            language: Language::Russian,
        }
    }
}

pub struct App {
    pub should_quit: bool,
    pub versions: VecDeque<MinecraftVersion>,
    pub state: ListState,
    pub current_state: AppState,
    pub language: Language,
    pub profiles: HashMap<String, Profile>,
    pub current_profile: Option<String>,
    pub loading: bool,
    pub focus: Focus,
    pub last_motd_update: DateTime<Local>,
    pub current_motd: String,
    pub art_rotation: f32,
    pub settings: Settings,
}

impl App {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        
        let mut profiles = HashMap::new();
        profiles.insert(
            "Default".to_string(),
            Profile {
                name: "Default".to_string(),
                username: String::new(),
                selected_version: None,
                ram: "2G".to_string(),
                java_args: "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 -XX:+UnlockExperimentalVMOptions -XX:+DisableAttachMechanism -XX:+UnlockDiagnosticVMOptions -XX:+AlwaysActAsNonStickyUnlockExperimentalVMOptions -XX:G1NewSizePercent=40 -XX:G1MaxNewSizePercent=50 -XX:G1HeapRegionSize=16M -XX:G1ReservePercent=15 -XX:G1HeapWastePercent=5 -XX:G1MixedGCCountTarget=4 -XX:InitiatingHeapOccupancyPercent=20 -XX:G1MixedGCLiveThresholdPercent=90 -XX:G1RSetUpdatingPauseTimePercent=5 -XX:SurvivorRatio=32 -XX:+PerfDisableSharedMem -XX:MaxTenuringThreshold=1".to_string(),
            },
        );

        let mut app = Self {
            should_quit: false,
            versions: VecDeque::new(),
            state,
            current_state: AppState::MainMenu,
            language: Language::Russian,
            profiles,
            current_profile: Some("Default".to_string()),
            loading: false,
            focus: Focus::List,
            last_motd_update: Local::now(),
            current_motd: MOTDS[0].to_string(),
            art_rotation: 0.0,
            settings: Settings::default(),
        };
        app.update_motd();
        app
    }

    pub fn next(&mut self) {
        match self.current_state {
            AppState::MainMenu | AppState::VersionSelect | AppState::ProfileSelect => {
                if let Some(selected) = self.state.selected() {
                    let max = match self.current_state {
                        AppState::MainMenu => 3,
                        AppState::VersionSelect => self.versions.len().saturating_sub(1),
                        AppState::ProfileSelect => self.profiles.len().saturating_sub(1),
                        _ => 0,
                    };
                    if selected < max {
                        self.state.select(Some(selected + 1));
                    }
                }
            }
            AppState::ProfileEdit => {
                self.focus = Focus::Input;
            }
            _ => {}
        }
    }

    pub fn previous(&mut self) {
        match self.current_state {
            AppState::MainMenu | AppState::VersionSelect | AppState::ProfileSelect => {
                if let Some(selected) = self.state.selected() {
                    if selected > 0 {
                        self.state.select(Some(selected - 1));
                    }
                }
            }
            AppState::ProfileEdit => {
                self.focus = Focus::Menu;
            }
            _ => {}
        }
    }

    pub fn toggle_language(&mut self) {
        self.language = match self.language {
            Language::Russian => Language::English,
            Language::English => Language::Russian,
        };
        self.settings.language = self.language.clone();
    }

    pub fn toggle_focus(&mut self) {
        if self.current_state == AppState::ProfileEdit {
            self.focus = match self.focus {
                Focus::Menu => Focus::Input,
                Focus::Input => Focus::Menu,
                Focus::List => Focus::Menu,
            };
        }
    }

    pub fn update_motd(&mut self) {
        let now = Local::now();
        if (now - self.last_motd_update).num_days() >= 1 {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            self.current_motd = MOTDS.choose(&mut rng).unwrap().to_string();
            self.last_motd_update = now;
        }
    }

    pub fn rotate_art(&mut self) {
        self.art_rotation += 0.1;
        if self.art_rotation >= 360.0 {
            self.art_rotation = 0.0;
        }
    }

    pub fn adjust_left_panel(&mut self, increase: bool) {
        if increase && self.settings.left_panel_width < 50 {
            self.settings.left_panel_width += 1;
        } else if !increase && self.settings.left_panel_width > 20 {
            self.settings.left_panel_width -= 1;
        }
    }

    pub fn save_profile(&mut self) {
        if let Some(profile_name) = &self.current_profile {
            if let Some(_) = self.profiles.get(profile_name) {
                // Профиль сохранен
            }
        }
    }
} 