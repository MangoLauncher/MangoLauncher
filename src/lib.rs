pub mod app;
pub mod ui;
pub mod version;
pub mod java;

use std::collections::VecDeque;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use app::Focus;
use version::{Version, VersionManager, VersionType};
use java::JavaManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub username: String,
    pub ram: u16,
    pub java_args: String,
    pub selected_version: Option<Version>,
    pub last_used: Option<DateTime<Utc>>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            username: String::new(),
            ram: 2048,
            java_args: String::from("-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200"),
            selected_version: None,
            last_used: None,
        }
    }
}

pub struct App {
    pub focus: Focus,
    pub profiles: Vec<Profile>,
    pub selected_profile: usize,
    pub editing_profile: Option<Profile>,
    pub version_manager: VersionManager,
    pub java_manager: JavaManager,
    pub current_motd: String,
    pub motd_rotation: u8,
    pub language: String,
}

impl App {
    pub fn new() -> Self {
        Self {
            focus: Focus::MainMenu,
            profiles: vec![Profile::default()],
            selected_profile: 0,
            editing_profile: None,
            version_manager: VersionManager::new(),
            java_manager: JavaManager::new(),
            current_motd: String::new(),
            motd_rotation: 0,
            language: String::from("ru"),
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        self.version_manager.init().await?;
        self.java_manager.init().await?;
        self.update_motd();
        Ok(())
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
        self.motd_rotation = self.motd_rotation.wrapping_add(1);
        self.update_motd();
    }

    pub fn toggle_language(&mut self) {
        self.language = if self.language == "ru" {
            String::from("en")
        } else {
            String::from("ru")
        };
        self.update_motd();
    }

    pub fn save_profile(&mut self) {
        if let Some(profile) = self.editing_profile.take() {
            if self.selected_profile < self.profiles.len() {
                self.profiles[self.selected_profile] = profile;
            }
        }
    }
} 