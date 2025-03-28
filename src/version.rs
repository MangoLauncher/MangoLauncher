use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use anyhow::Result;

pub const VERSION: &str = "0.1.0";
const VERSIONS_DIR: &str = "versions";
const MANIFEST_URL: &str = "https://launchermeta.mojang.com/mc/game/version_manifest.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct MinecraftVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub release_type: String,
    pub url: String,
    pub time: String,
    pub release_time: String,
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

pub struct VersionManager {
    versions_dir: PathBuf,
    manifest: Option<VersionManifest>,
}

impl VersionManager {
    pub fn new() -> Self {
        Self {
            versions_dir: PathBuf::from(VERSIONS_DIR),
            manifest: None,
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        if !self.versions_dir.exists() {
            fs::create_dir_all(&self.versions_dir).await?;
        }
        self.update_manifest().await?;
        Ok(())
    }

    pub async fn update_manifest(&mut self) -> Result<()> {
        let response = reqwest::get(MANIFEST_URL).await?;
        self.manifest = Some(response.json().await?);
        Ok(())
    }

    pub fn get_versions(&self) -> Vec<&MinecraftVersion> {
        self.manifest
            .as_ref()
            .map(|m| m.versions.iter().collect())
            .unwrap_or_default()
    }

    pub fn get_release_versions(&self) -> Vec<&MinecraftVersion> {
        self.get_versions()
            .into_iter()
            .filter(|v| v.release_type == "release")
            .collect()
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
        
        Ok(())
    }

    pub fn is_version_installed(&self, version_id: &str) -> bool {
        self.versions_dir.join(version_id).exists()
    }
} 