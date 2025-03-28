use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::collections::VecDeque;
use tokio::fs;
use anyhow::Result;
use chrono::{DateTime, Utc};

pub const VERSION: &str = "0.1.0";
const VERSIONS_DIR: &str = "versions";
const MANIFEST_URL: &str = "https://launchermeta.mojang.com/mc/game/version_manifest.json";
const RECENT_VERSIONS_LIMIT: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VersionType {
    Vanilla,
    Forge(String),    // Версия форджа
    OptiFine(String), // Версия оптифайна
    ForgeOptiFine { forge: String, optifine: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub release_type: String,
    pub url: String,
    pub time: String,
    pub release_time: String,
    #[serde(default)]
    pub version_type: VersionType,
    #[serde(skip)]
    pub last_used: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: Latest,
    pub versions: Vec<MinecraftVersion>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionHistory {
    pub recent_versions: VecDeque<String>,
    pub last_used: std::collections::HashMap<String, DateTime<Utc>>,
}

impl Default for VersionHistory {
    fn default() -> Self {
        Self {
            recent_versions: VecDeque::with_capacity(RECENT_VERSIONS_LIMIT),
            last_used: std::collections::HashMap::new(),
        }
    }
}

pub struct VersionManager {
    versions_dir: PathBuf,
    manifest: Option<VersionManifest>,
    history: VersionHistory,
    current_view: VersionView,
}

#[derive(Debug, PartialEq)]
pub enum VersionView {
    Recent,
    All,
    Modded,
}

impl VersionManager {
    pub fn new() -> Self {
        Self {
            versions_dir: PathBuf::from(VERSIONS_DIR),
            manifest: None,
            history: VersionHistory::default(),
            current_view: VersionView::Recent,
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        if !self.versions_dir.exists() {
            fs::create_dir_all(&self.versions_dir).await?;
        }
        self.load_history().await?;
        self.update_manifest().await?;
        Ok(())
    }

    async fn load_history(&mut self) -> Result<()> {
        let history_path = self.versions_dir.join("history.json");
        if history_path.exists() {
            let content = fs::read_to_string(&history_path).await?;
            self.history = serde_json::from_str(&content)?;
        }
        Ok(())
    }

    async fn save_history(&self) -> Result<()> {
        let history_path = self.versions_dir.join("history.json");
        let content = serde_json::to_string_pretty(&self.history)?;
        fs::write(&history_path, content).await?;
        Ok(())
    }

    pub async fn update_manifest(&mut self) -> Result<()> {
        let response = reqwest::get(MANIFEST_URL).await?;
        self.manifest = Some(response.json().await?);
        Ok(())
    }

    pub fn toggle_view(&mut self) {
        self.current_view = match self.current_view {
            VersionView::Recent => VersionView::All,
            VersionView::All => VersionView::Modded,
            VersionView::Modded => VersionView::Recent,
        };
    }

    pub fn get_current_versions(&self) -> Vec<MinecraftVersion> {
        match self.current_view {
            VersionView::Recent => self.get_recent_versions(),
            VersionView::All => self.get_all_versions(),
            VersionView::Modded => self.get_modded_versions(),
        }
    }

    fn get_recent_versions(&self) -> Vec<MinecraftVersion> {
        let all_versions = self.get_all_versions();
        self.history.recent_versions
            .iter()
            .filter_map(|id| {
                all_versions.iter()
                    .find(|v| &v.id == id)
                    .cloned()
            })
            .collect()
    }

    fn get_all_versions(&self) -> Vec<MinecraftVersion> {
        self.manifest
            .as_ref()
            .map(|m| m.versions.clone())
            .unwrap_or_default()
    }

    fn get_modded_versions(&self) -> Vec<MinecraftVersion> {
        // TODO: Реализовать получение модифицированных версий
        vec![]
    }

    pub async fn mark_version_used(&mut self, version_id: String) -> Result<()> {
        let now = Utc::now();
        self.history.last_used.insert(version_id.clone(), now);
        
        // Обновляем список недавно использованных версий
        if let Some(index) = self.history.recent_versions.iter().position(|x| x == &version_id) {
            self.history.recent_versions.remove(index);
        }
        self.history.recent_versions.push_front(version_id);
        
        // Ограничиваем количество недавних версий
        while self.history.recent_versions.len() > RECENT_VERSIONS_LIMIT {
            self.history.recent_versions.pop_back();
        }
        
        self.save_history().await?;
        Ok(())
    }

    pub async fn download_version(&self, version: &MinecraftVersion) -> Result<()> {
        let version_dir = self.versions_dir.join(&version.id);
        if !version_dir.exists() {
            fs::create_dir_all(&version_dir).await?;
        }

        // Скачиваем информацию о версии
        let response = reqwest::get(&version.url).await?;
        let version_info: serde_json::Value = response.json().await?;
        
        // Сохраняем информацию о версии
        let version_json = version_dir.join("version.json");
        fs::write(&version_json, serde_json::to_string_pretty(&version_info)?).await?;

        // TODO: Скачивание client.jar и других необходимых файлов
        // TODO: Для модифицированных версий - скачивание и установка модов
        
        Ok(())
    }

    pub fn is_version_installed(&self, version_id: &str) -> bool {
        self.versions_dir.join(version_id).exists()
    }
} 