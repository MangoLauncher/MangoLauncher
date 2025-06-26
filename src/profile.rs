
use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: Uuid,
    pub name: String,
    pub username: String,
    pub memory_min: u32,
    pub memory_max: u32,
    pub java_path: Option<PathBuf>,
    pub java_args: String,
    pub game_args: Option<String>,
    pub resolution_width: Option<u32>,
    pub resolution_height: Option<u32>,
    pub fullscreen: bool,
    pub demo_mode: bool,
    pub auto_connect_server: Option<String>,
    pub custom_icon: Option<PathBuf>,
    pub wrapper_command: Option<String>,
    pub pre_launch_command: Option<String>,
    pub post_exit_command: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Default".to_string(),
            username: "Player".to_string(),
            memory_min: 512,
            memory_max: 2048,
            java_path: None,
            java_args: "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200".to_string(),
            game_args: None,
            resolution_width: None,
            resolution_height: None,
            fullscreen: false,
            demo_mode: false,
            auto_connect_server: None,
            custom_icon: None,
            wrapper_command: None,
            pre_launch_command: None,
            post_exit_command: None,
            created_at: Utc::now(),
            last_used: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchProfile {
    pub minecraft_arguments: Vec<String>,
    pub jvm_arguments: Vec<String>,
    pub main_class: String,
    pub minecraft_version: String,
    pub assets_index: String,
    pub assets_dir: PathBuf,
    pub game_dir: PathBuf,
    pub libraries_dir: PathBuf,
    pub natives_dir: PathBuf,
    pub classpath: Vec<PathBuf>,
}

pub struct ProfileManager {
    profiles: HashMap<Uuid, Profile>,
    active_profile: Option<Uuid>,
    profiles_dir: PathBuf,
}

impl ProfileManager {
    pub fn new(profiles_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&profiles_dir)?;
        
        let mut manager = Self {
            profiles: HashMap::new(),
            active_profile: None,
            profiles_dir,
        };
        
        manager.load_profiles()?;
        
        if manager.profiles.is_empty() {
            let default_profile = Profile::default();
            let id = default_profile.id;
            manager.profiles.insert(id, default_profile);
            manager.active_profile = Some(id);
            manager.save_profiles()?;
        }
        
        Ok(manager)
    }

    pub fn create_profile(&mut self, name: String, username: String) -> Result<Uuid> {
        let mut profile = Profile::default();
        profile.name = name;
        profile.username = username;
        profile.id = Uuid::new_v4();
        
        let id = profile.id;
        self.profiles.insert(id, profile);
        self.save_profiles()?;
        
        Ok(id)
    }

    pub fn delete_profile(&mut self, id: Uuid) -> Result<()> {
        if self.profiles.len() <= 1 {
            return Err(Error::Profile("Cannot delete the last profile".to_string()));
        }
        
        self.profiles.remove(&id);
        
        if self.active_profile == Some(id) {
            self.active_profile = self.profiles.keys().next().copied();
        }
        
        self.save_profiles()?;
        Ok(())
    }

    pub fn get_profile(&self, id: Uuid) -> Option<&Profile> {
        self.profiles.get(&id)
    }

    pub fn get_profile_mut(&mut self, id: Uuid) -> Option<&mut Profile> {
        self.profiles.get_mut(&id)
    }

    pub fn get_active_profile(&self) -> Option<&Profile> {
        self.active_profile.and_then(|id| self.profiles.get(&id))
    }

    pub fn set_active_profile(&mut self, id: Uuid) -> Result<()> {
        if self.profiles.contains_key(&id) {
            self.active_profile = Some(id);
            if let Some(profile) = self.profiles.get_mut(&id) {
                profile.last_used = Some(Utc::now());
            }
            self.save_profiles()?;
            Ok(())
        } else {
            Err(Error::Profile("Profile not found".to_string()))
        }
    }

    pub fn update_profile(&mut self, profile: Profile) -> Result<()> {
        self.profiles.insert(profile.id, profile);
        self.save_profiles()?;
        Ok(())
    }

    pub fn list_profiles(&self) -> Vec<&Profile> {
        self.profiles.values().collect()
    }

    pub fn clone_profile(&mut self, source_id: Uuid, new_name: String) -> Result<Uuid> {
        if let Some(source) = self.profiles.get(&source_id).cloned() {
            let mut new_profile = source;
            new_profile.id = Uuid::new_v4();
            new_profile.name = new_name;
            new_profile.created_at = Utc::now();
            new_profile.last_used = None;
            
            let id = new_profile.id;
            self.profiles.insert(id, new_profile);
            self.save_profiles()?;
            
            Ok(id)
        } else {
            Err(Error::Profile("Source profile not found".to_string()))
        }
    }

    pub fn build_launch_profile(
        &self,
        profile_id: Uuid,
        minecraft_version: &str,
        game_dir: PathBuf,
        assets_dir: PathBuf,
        libraries_dir: PathBuf,
        natives_dir: PathBuf,
    ) -> Result<LaunchProfile> {
        let profile = self.get_profile(profile_id)
            .ok_or_else(|| Error::Profile("Profile not found".to_string()))?;

        let mut jvm_arguments = vec![
            format!("-Xms{}M", profile.memory_min),
            format!("-Xmx{}M", profile.memory_max),
            "-Djava.library.path=${natives_directory}".to_string(),
            "-Dminecraft.launcher.brand=mango-launcher".to_string(),
            "-Dminecraft.launcher.version=1.0.0".to_string(),
            "-cp".to_string(),
            "${classpath}".to_string(),
        ];

        let custom_jvm_args: Vec<String> = profile.java_args
            .split_whitespace()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        jvm_arguments.extend(custom_jvm_args);

        let mut minecraft_arguments = vec![
            "--username".to_string(),
            profile.username.clone(),
            "--version".to_string(),
            minecraft_version.to_string(),
            "--gameDir".to_string(),
            game_dir.to_string_lossy().to_string(),
            "--assetsDir".to_string(),
            assets_dir.to_string_lossy().to_string(),
            "--assetIndex".to_string(),
            minecraft_version.to_string(),
            "--uuid".to_string(),
            "00000000-0000-0000-0000-000000000000".to_string(),
            "--accessToken".to_string(),
            "0".to_string(),
            "--clientId".to_string(),
            "00000000-0000-0000-0000-000000000000".to_string(),
            "--xuid".to_string(),
            "0".to_string(),
            "--userType".to_string(),
            "legacy".to_string(),
        ];

        if let Some(width) = profile.resolution_width {
            minecraft_arguments.push("--width".to_string());
            minecraft_arguments.push(width.to_string());
        }

        if let Some(height) = profile.resolution_height {
            minecraft_arguments.push("--height".to_string());
            minecraft_arguments.push(height.to_string());
        }

        if profile.fullscreen {
            minecraft_arguments.push("--fullscreen".to_string());
        }

        if profile.demo_mode {
            minecraft_arguments.push("--demo".to_string());
        }

        if let Some(server) = &profile.auto_connect_server {
            let parts: Vec<&str> = server.split(':').collect();
            minecraft_arguments.push("--server".to_string());
            minecraft_arguments.push(parts[0].to_string());
            if parts.len() > 1 {
                minecraft_arguments.push("--port".to_string());
                minecraft_arguments.push(parts[1].to_string());
            }
        }

        if let Some(game_args) = &profile.game_args {
            let custom_args: Vec<String> = game_args
                .split_whitespace()
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .collect();
            minecraft_arguments.extend(custom_args);
        }

        Ok(LaunchProfile {
            minecraft_arguments,
            jvm_arguments,
            main_class: "net.minecraft.client.main.Main".to_string(),
            minecraft_version: minecraft_version.to_string(),
            assets_index: minecraft_version.to_string(),
            assets_dir,
            game_dir,
            libraries_dir,
            natives_dir,
            classpath: Vec::new(),
        })
    }

    fn load_profiles(&mut self) -> Result<()> {
        let profiles_file = self.profiles_dir.join("profiles.json");
        if profiles_file.exists() {
            let content = std::fs::read_to_string(profiles_file)?;
            let data: serde_json::Value = serde_json::from_str(&content)?;
            
            if let Some(profiles_obj) = data.get("profiles") {
                self.profiles = serde_json::from_value(profiles_obj.clone())?;
            }
            
            if let Some(active_str) = data.get("active_profile").and_then(|v| v.as_str()) {
                if let Ok(active_uuid) = Uuid::parse_str(active_str) {
                    self.active_profile = Some(active_uuid);
                }
            }
        }
        Ok(())
    }

    fn save_profiles(&self) -> Result<()> {
        let profiles_file = self.profiles_dir.join("profiles.json");
        
        let data = serde_json::json!({
            "profiles": self.profiles,
            "active_profile": self.active_profile
        });
        
        let content = serde_json::to_string_pretty(&data)?;
        std::fs::write(profiles_file, content)?;
        Ok(())
    }
} 