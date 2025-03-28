use std::collections::{HashMap, VecDeque};
use ratatui::widgets::ListState;
use chrono::{DateTime, Local};
use anyhow::Result;
use crossterm::event::KeyEvent;
use crate::{Profile, VersionManager, JavaManager};

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

pub enum Focus {
    Menu,
    Input,
}

pub enum AppState {
    MainMenu,
    VersionSelect,
    ProfileSelect,
    ProfileEdit,
    Settings,
    Changelog,
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
    pub language: String,
    pub profiles: HashMap<String, Profile>,
    pub current_profile: Option<String>,
    pub loading: bool,
    pub focus: Focus,
    pub last_motd_update: DateTime<Local>,
    pub current_motd: String,
    pub art_rotation: f32,
    pub settings: Settings,
    pub version_manager: VersionManager,
    pub java_manager: JavaManager,
    pub motd_rotation: u8,
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
            language: String::from("ru"),
            profiles,
            current_profile: Some("Default".to_string()),
            loading: false,
            focus: Focus::Menu,
            last_motd_update: Local::now(),
            current_motd: MOTDS[0].to_string(),
            art_rotation: 0.0,
            settings: Settings::default(),
            version_manager: VersionManager::new(),
            java_manager: JavaManager::new(),
            motd_rotation: 0,
        };
        app.update_motd();
        app
    }

    pub async fn init(&mut self) -> Result<()> {
        self.version_manager.init().await?;
        self.java_manager.init().await?;
        self.update_motd();
        Ok(())
    }

    pub fn next(&mut self) {
        let len = match self.current_state {
            AppState::MainMenu => 5,
            AppState::VersionSelect => self.versions.len(),
            AppState::ProfileSelect => self.profiles.len(),
            AppState::ProfileEdit => 0,
            AppState::Settings => 3,
            AppState::Changelog => 0,
        };

        if len == 0 {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i >= len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let len = match self.current_state {
            AppState::MainMenu => 5,
            AppState::VersionSelect => self.versions.len(),
            AppState::ProfileSelect => self.profiles.len(),
            AppState::ProfileEdit => 0,
            AppState::Settings => 3,
            AppState::Changelog => 0,
        };

        if len == 0 {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn toggle_language(&mut self) {
        self.language = if self.language == "ru" {
            String::from("en")
        } else {
            String::from("ru")
        };
        self.settings.language = match self.language.as_str() {
            "ru" => Language::Russian,
            "en" => Language::English,
            _ => panic!("Unknown language"),
        };
        self.update_motd();
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Menu => Focus::Input,
            Focus::Input => Focus::Menu,
        };
    }

    pub fn update_motd(&mut self) {
        let motds = if self.language == "ru" {
            vec![
                "Манго - это вкусно!",
                "Самый сочный лаунчер",
                "Спелый и сладкий",
                "Витамин C для вашего Minecraft",
                "Тропический вкус игры",
                "Сделано с любовью к манго",
                "Манго - король фруктов",
                "Сочная оптимизация",
                "Свежий взгляд на Minecraft",
                "Экзотический лаунчер",
            ]
        } else {
            vec![
                "Mango is delicious!",
                "The juiciest launcher",
                "Ripe and sweet",
                "Vitamin C for your Minecraft",
                "Tropical taste of gaming",
                "Made with mango love",
                "Mango - king of fruits",
                "Juicy optimization",
                "Fresh look at Minecraft",
                "Exotic launcher",
            ]
        };

        let index = (self.motd_rotation as usize) % motds.len();
        self.current_motd = motds[index].to_string();
    }

    pub fn rotate_art(&mut self) {
        self.art_rotation += 0.1;
        if self.art_rotation >= 360.0 {
            self.art_rotation = 0.0;
        }
        self.motd_rotation = self.motd_rotation.wrapping_add(1);
        self.update_motd();
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