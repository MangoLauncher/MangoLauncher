use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::{Error, Result};

fn default_save_logs_to_file() -> bool {
    true
}

fn default_logs_directory() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("mango-launcher")
        .join("logs")
}

fn default_log_retention_hours() -> u32 {
    24
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Language {
    Russian,
    English,
}

impl Default for Language {
    fn default() -> Self {
        Language::Russian
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub general: GeneralSettings,
    pub java: JavaSettings,
    pub minecraft: MinecraftSettings,
    pub ui: UiSettings,
    pub network: NetworkSettings,
    pub advanced: AdvancedSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub language: Language,
    pub theme: String,
    pub instances_directory: PathBuf,
    pub java_directory: PathBuf,
    pub check_for_updates: bool,
    pub send_analytics: bool,
    pub maximize_on_launch: bool,
    pub close_launcher_on_game_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaSettings {
    pub default_installation: Option<PathBuf>,
    pub memory_min: u32,
    pub memory_max: u32,
    pub permgen_size: u32,
    pub gc_args: String,
    pub additional_args: String,
    pub auto_detect_installations: bool,
    pub download_missing_java: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftSettings {
    pub default_width: u32,
    pub default_height: u32,
    pub fullscreen: bool,
    pub auto_login: bool,
    pub pre_launch_command: Option<String>,
    pub post_exit_command: Option<String>,
    pub wrapper_command: Option<String>,
    pub enable_console: bool,
    pub auto_close_console: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    pub window_width: u32,
    pub window_height: u32,
    pub window_maximized: bool,
    pub instance_view_type: String,
    pub sort_mode: String,
    pub show_console: bool,
    pub icon_size: String,
    pub group_view: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    pub use_proxy: bool,
    pub proxy_type: String,
    pub proxy_host: String,
    pub proxy_port: u16,
    pub proxy_username: Option<String>,
    pub proxy_password: Option<String>,
    pub timeout: u64,
    pub max_concurrent_downloads: u32,
    pub user_agent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSettings {
    pub enable_logging: bool,
    pub log_level: String,
    pub console_max_lines: u32,
    pub enable_profiling: bool,
    pub developer_mode: bool,
    pub custom_commands: HashMap<String, String>,
    pub environment_variables: HashMap<String, String>,
    #[serde(default = "default_save_logs_to_file")]
    pub save_logs_to_file: bool,
    #[serde(default = "default_logs_directory")]
    pub logs_directory: PathBuf,
    #[serde(default = "default_log_retention_hours")]
    pub log_retention_hours: u32,
}

impl Default for Settings {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mango-launcher");

        Self {
            general: GeneralSettings {
                language: Language::Russian,
                theme: "dark".to_string(),
                instances_directory: data_dir.join("instances"),
                java_directory: data_dir.join("java"),
                check_for_updates: true,
                send_analytics: false,
                maximize_on_launch: false,
                close_launcher_on_game_start: false,
            },
            java: JavaSettings {
                default_installation: None,
                memory_min: 512,
                memory_max: 2048,
                permgen_size: 128,
                gc_args: "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200".to_string(),
                additional_args: String::new(),
                auto_detect_installations: true,
                download_missing_java: true,
            },
            minecraft: MinecraftSettings {
                default_width: 854,
                default_height: 480,
                fullscreen: false,
                auto_login: false,
                pre_launch_command: None,
                post_exit_command: None,
                wrapper_command: None,
                enable_console: true,
                auto_close_console: false,
            },
            ui: UiSettings {
                window_width: 1024,
                window_height: 768,
                window_maximized: false,
                instance_view_type: "icons".to_string(),
                sort_mode: "name".to_string(),
                show_console: false,
                icon_size: "medium".to_string(),
                group_view: true,
            },
            network: NetworkSettings {
                use_proxy: false,
                proxy_type: "http".to_string(),
                proxy_host: String::new(),
                proxy_port: 8080,
                proxy_username: None,
                proxy_password: None,
                timeout: 30,
                max_concurrent_downloads: 4,
                user_agent: "mango-launcher/1.0".to_string(),
            },
            advanced: AdvancedSettings {
                enable_logging: true,
                log_level: "info".to_string(),
                console_max_lines: 100000,
                enable_profiling: false,
                developer_mode: false,
                custom_commands: HashMap::new(),
                environment_variables: HashMap::new(),
                save_logs_to_file: true,
                logs_directory: data_dir.join("logs"),
                log_retention_hours: 24,
            },
        }
    }
}

pub struct SettingsManager {
    settings: Settings,
    settings_path: PathBuf,
    dirty: bool,
}

impl SettingsManager {
    pub fn new(settings_path: PathBuf) -> Result<Self> {
        let mut manager = Self {
            settings: Settings::default(),
            settings_path,
            dirty: false,
        };

        manager.load()?;
        Ok(manager)
    }

    pub fn get(&self) -> &Settings {
        &self.settings
    }

    pub fn get_mut(&mut self) -> &mut Settings {
        self.dirty = true;
        &mut self.settings
    }

    pub fn set_general_setting<T>(&mut self, setter: impl FnOnce(&mut GeneralSettings) -> T) -> T {
        self.dirty = true;
        setter(&mut self.settings.general)
    }

    pub fn set_java_setting<T>(&mut self, setter: impl FnOnce(&mut JavaSettings) -> T) -> T {
        self.dirty = true;
        setter(&mut self.settings.java)
    }

    pub fn set_minecraft_setting<T>(&mut self, setter: impl FnOnce(&mut MinecraftSettings) -> T) -> T {
        self.dirty = true;
        setter(&mut self.settings.minecraft)
    }

    pub fn set_ui_setting<T>(&mut self, setter: impl FnOnce(&mut UiSettings) -> T) -> T {
        self.dirty = true;
        setter(&mut self.settings.ui)
    }

    pub fn set_network_setting<T>(&mut self, setter: impl FnOnce(&mut NetworkSettings) -> T) -> T {
        self.dirty = true;
        setter(&mut self.settings.network)
    }

    pub fn set_advanced_setting<T>(&mut self, setter: impl FnOnce(&mut AdvancedSettings) -> T) -> T {
        self.dirty = true;
        setter(&mut self.settings.advanced)
    }

    pub fn reset_to_defaults(&mut self) {
        self.settings = Settings::default();
        self.dirty = true;
    }

    pub fn reset_section(&mut self, section: &str) -> Result<()> {
        match section {
            "general" => self.settings.general = GeneralSettings::default(),
            "java" => self.settings.java = JavaSettings::default(),
            "minecraft" => self.settings.minecraft = MinecraftSettings::default(),
            "ui" => self.settings.ui = UiSettings::default(),
            "network" => self.settings.network = NetworkSettings::default(),
            "advanced" => self.settings.advanced = AdvancedSettings::default(),
            _ => return Err(Error::Settings(format!("Unknown section: {}", section))),
        }
        self.dirty = true;
        Ok(())
    }

    pub fn save(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }

        if let Some(parent) = self.settings_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(&self.settings)
            .map_err(|e| Error::Settings(format!("Failed to serialize settings: {}", e)))?;

        std::fs::write(&self.settings_path, content)?;
        self.dirty = false;

        log::info!("Settings saved to {:?}", self.settings_path);
        Ok(())
    }

    pub fn load(&mut self) -> Result<()> {
        if !self.settings_path.exists() {
            self.dirty = true;
            return self.save();
        }

        let content = std::fs::read_to_string(&self.settings_path)?;
        
        self.settings = toml::from_str(&content)
            .map_err(|e| Error::Settings(format!("Failed to parse settings: {}", e)))?;

        self.dirty = false;
        log::info!("Settings loaded from {:?}", self.settings_path);
        Ok(())
    }

    pub fn auto_save(&mut self) -> Result<()> {
        if self.dirty {
            self.save()?;
        }
        Ok(())
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn validate(&self) -> Result<()> {
        if self.settings.java.memory_min > self.settings.java.memory_max {
            return Err(Error::Settings(
                "Minimum memory cannot be greater than maximum memory".to_string()
            ));
        }

        if self.settings.network.proxy_port == 0 {
            return Err(Error::Settings("Proxy port cannot be 0".to_string()));
        }

        if !self.settings.general.instances_directory.is_absolute() {
            return Err(Error::Settings(
                "Instances directory must be an absolute path".to_string()
            ));
        }

        Ok(())
    }

    pub fn export_to_file(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(&self.settings)
            .map_err(|e| Error::Settings(format!("Failed to serialize settings: {}", e)))?;

        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn import_from_file(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        
        self.settings = toml::from_str(&content)
            .map_err(|e| Error::Settings(format!("Failed to parse settings: {}", e)))?;

        self.validate()?;
        self.dirty = true;
        Ok(())
    }
}

impl GeneralSettings {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mango-launcher");

        Self {
            language: Language::Russian,
            theme: "dark".to_string(),
            instances_directory: data_dir.join("instances"),
            java_directory: data_dir.join("java"),
            check_for_updates: true,
            send_analytics: false,
            maximize_on_launch: false,
            close_launcher_on_game_start: false,
        }
    }
}

impl JavaSettings {
    fn default() -> Self {
        Self {
            default_installation: None,
            memory_min: 512,
            memory_max: 2048,
            permgen_size: 128,
            gc_args: "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200".to_string(),
            additional_args: String::new(),
            auto_detect_installations: true,
            download_missing_java: true,
        }
    }
}

impl MinecraftSettings {
    fn default() -> Self {
        Self {
            default_width: 854,
            default_height: 480,
            fullscreen: false,
            auto_login: false,
            pre_launch_command: None,
            post_exit_command: None,
            wrapper_command: None,
            enable_console: true,
            auto_close_console: false,
        }
    }
}

impl UiSettings {
    fn default() -> Self {
        Self {
            window_width: 1024,
            window_height: 768,
            window_maximized: false,
            instance_view_type: "icons".to_string(),
            sort_mode: "name".to_string(),
            show_console: false,
            icon_size: "medium".to_string(),
            group_view: true,
        }
    }
}

impl NetworkSettings {
    fn default() -> Self {
        Self {
            use_proxy: false,
            proxy_type: "http".to_string(),
            proxy_host: String::new(),
            proxy_port: 8080,
            proxy_username: None,
            proxy_password: None,
            timeout: 30,
            max_concurrent_downloads: 4,
            user_agent: "mango-launcher/1.0".to_string(),
        }
    }
}

impl AdvancedSettings {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mango-launcher");
            
        Self {
            enable_logging: true,
            log_level: "info".to_string(),
            console_max_lines: 100000,
            enable_profiling: false,
            developer_mode: false,
            custom_commands: HashMap::new(),
            environment_variables: HashMap::new(),
            save_logs_to_file: true,
            logs_directory: data_dir.join("logs"),
            log_retention_hours: 24,
        }
    }
} 