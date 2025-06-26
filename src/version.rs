#![allow(unused)]
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::collections::VecDeque;
use tokio::fs;
use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::network::NetworkManager;
use std::collections::HashMap;

const MANIFEST_URL: &str = "https://launchermeta.mojang.com/mc/game/version_manifest.json";
const RECENT_VERSIONS_LIMIT: usize = 5;



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftVersion {
    pub id: String,
    pub r#type: String,
    pub url: String,
    pub time: Option<String>,
    pub release_time: Option<String>,
    pub compliance_level: Option<i32>,
    pub sha1: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDetails {
    pub id: String,
    pub r#type: String,
    pub time: Option<String>,
    pub release_time: Option<String>,
    pub main_class: Option<String>,
    pub minecraft_arguments: Option<String>,
    pub arguments: Option<Arguments>,
    pub libraries: Option<Vec<Library>>,
    pub downloads: Option<Downloads>,
    pub assets: Option<String>,
    #[serde(rename = "assetIndex")]
    pub asset_index: Option<AssetIndexInfo>,
    pub java_version: Option<JavaVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndexInfo {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub total_size: Option<u64>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arguments {
    pub game: Vec<Argument>,
    pub jvm: Vec<Argument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Argument {
    String(String),
    Object {
        rules: Vec<Rule>,
        value: ArgumentValue,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    String(String),
    Array(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    pub rules: Option<Vec<Rule>>,
    pub natives: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<Artifact>,
    pub classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub path: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Downloads {
    pub client: Option<DownloadInfo>,
    pub server: Option<DownloadInfo>,
    pub client_mappings: Option<DownloadInfo>,
    pub server_mappings: Option<DownloadInfo>,
    pub windows_server: Option<DownloadInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInfo {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaVersion {
    pub component: String,
    pub major_version: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: Option<VersionLatest>,
    pub versions: Vec<MinecraftVersion>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionLatest {
    pub release: Option<String>,
    pub snapshot: Option<String>,
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
    network: NetworkManager,
    cached_manifest: Option<VersionManifest>,
    history: VersionHistory,
    current_view: VersionView,
    versions: Vec<MinecraftVersion>,
    max_concurrent_downloads: usize,
}

#[derive(Debug, PartialEq)]
pub enum VersionView {
    Recent,
    All,
    Modded,
}

impl VersionManager {
    pub fn new(versions_dir: PathBuf, network: NetworkManager, max_concurrent_downloads: usize) -> Result<Self> {
        std::fs::create_dir_all(&versions_dir)?;
        
        Ok(Self {
            versions_dir,
            network,
            cached_manifest: None,
            history: VersionHistory::default(),
            current_view: VersionView::Recent,
            versions: Vec::new(),
            max_concurrent_downloads,
        })
    }

    pub async fn init(&mut self) -> Result<()> {
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
        self.cached_manifest = Some(response.json().await?);
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
        self.cached_manifest
            .as_ref()
            .map(|m| m.versions.clone())
            .unwrap_or_default()
    }

    fn get_modded_versions(&self) -> Vec<MinecraftVersion> {

        vec![]
    }

    pub async fn mark_version_used(&mut self, version_id: String) -> Result<()> {
        let now = Utc::now();
        self.history.last_used.insert(version_id.clone(), now);
        

        if let Some(index) = self.history.recent_versions.iter().position(|x| x == &version_id) {
            self.history.recent_versions.remove(index);
        }
        self.history.recent_versions.push_front(version_id);
        

        while self.history.recent_versions.len() > RECENT_VERSIONS_LIMIT {
            self.history.recent_versions.pop_back();
        }
        
        self.save_history().await?;
        Ok(())
    }

    pub async fn download_version(&self, version: &MinecraftVersion) -> Result<()> {
        let version_dir = self.versions_dir.join(&version.id);
        std::fs::create_dir_all(&version_dir)?;

        let version_details: VersionDetails = self.network.get_json(&version.url).await?;
        
        let version_file = version_dir.join(format!("{}.json", version.id));
        let version_json = serde_json::to_string_pretty(&version_details)?;
        std::fs::write(version_file, version_json)?;

        if let Some(downloads) = &version_details.downloads {
            if let Some(client) = &downloads.client {
                let client_path = version_dir.join(format!("{}.jar", version.id));
                let filename = format!("minecraft-{}.jar", version.id);
                
                let success = self.network.download_with_progress_dialog(
                    &client.url,
                    &client_path,
                    Some(&client.sha1),
                    filename,
                ).await?;
                
                if !success {
                    return Err(crate::Error::Other("Загрузка отменена пользователем".to_string()).into());
                }

                if !self.verify_jar_integrity(&client_path).await? {
                    std::fs::remove_file(&client_path).ok();
                    return Err(crate::Error::Other("JAR файл поврежден или не является корректным архивом".to_string()).into());
                }
            }
        }

        self.download_libraries_with_settings(&version_details).await?;

        Ok(())
    }



    pub async fn download_libraries_with_settings(&self, version_details: &VersionDetails) -> Result<()> {
        if let Some(libraries) = &version_details.libraries {
            let libraries_dir = self.get_libraries_dir();
            std::fs::create_dir_all(&libraries_dir)?;

            let mut download_tasks = Vec::new();
            
            for library in libraries {
                if let Some(downloads) = &library.downloads {
                    if let Some(artifact) = &downloads.artifact {
                        let lib_path = libraries_dir.join(&artifact.path);
                        
                        if !lib_path.exists() {
                            download_tasks.push((
                                artifact.url.clone(),
                                lib_path,
                                Some(artifact.sha1.clone()),
                            ));
                        }
                    }
                    

                    if let Some(classifiers) = &downloads.classifiers {
                        for (classifier, artifact) in classifiers {
                            let lib_path = libraries_dir.join(&artifact.path);
                            
                            if !lib_path.exists() {
                                download_tasks.push((
                                    artifact.url.clone(),
                                    lib_path,
                                    Some(artifact.sha1.clone()),
                                ));
                            }
                        }
                    }
                }
            }

            if !download_tasks.is_empty() {
                let results = self.network.download_files_concurrent(download_tasks).await?;
                
                for success in results {
                    if !success {
                        return Err(crate::Error::Other("Загрузка библиотеки отменена".to_string()).into());
                    }
                }
            }
        }
        Ok(())
    }

    pub fn set_max_concurrent_downloads(&mut self, max_concurrent: usize) {
        self.max_concurrent_downloads = max_concurrent;
    }

    async fn verify_jar_integrity(&self, jar_path: &Path) -> Result<bool> {
        if !jar_path.exists() {
            return Ok(false);
        }


        let metadata = std::fs::metadata(jar_path)?;
        if metadata.len() == 0 {
            return Ok(false);
        }


        match std::fs::File::open(jar_path) {
            Ok(file) => {
                match zip::ZipArchive::new(file) {
                    Ok(mut archive) => {
                
                        let _expected_files = [
                            "META-INF/MANIFEST.MF",
                            "net/minecraft/client/main/Main.class",
                        ];
                        
                        let mut found_main = false;
                        for i in 0..archive.len() {
                            if let Ok(file) = archive.by_index(i) {
                                let name = file.name();
                                if name.contains("Main.class") || name.contains("MinecraftServer.class") {
                                    found_main = true;
                                    break;
                                }
                            }
                        }
                        
                        if !found_main {
                                    return Ok(archive.len() > 100);
                        }
                        
                        Ok(true)
                    }
                    Err(_) => Ok(false)
                }
            }
            Err(_) => Ok(false)
        }
    }

    pub fn is_version_installed(&self, version_id: &str) -> bool {
        let version_dir = self.versions_dir.join(version_id);
        let version_json = version_dir.join(format!("{}.json", version_id));
        let version_jar = version_dir.join(format!("{}.jar", version_id));
        
        if !version_json.exists() || !version_jar.exists() {
            return false;
        }

        if let Ok(version_details) = self.get_version_details(version_id) {
            if let Some(libraries) = &version_details.libraries {
                let libraries_dir = self.get_libraries_dir();
                
                for library in libraries {
                    if let Some(downloads) = &library.downloads {
                        if let Some(artifact) = &downloads.artifact {
                            let lib_path = libraries_dir.join(&artifact.path);
                            if !lib_path.exists() {
                                return false;
                            }
                        }
                        
                        if let Some(classifiers) = &downloads.classifiers {
                            for (_, artifact) in classifiers {
                                let lib_path = libraries_dir.join(&artifact.path);
                                if !lib_path.exists() {
                                    return false;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        true
    }

    pub fn get_version_details(&self, version_id: &str) -> Result<VersionDetails> {
        let version_file = self.versions_dir.join(version_id).join(format!("{}.json", version_id));
        
        if !version_file.exists() {
            return Err(crate::Error::Version(format!("Version {} not installed", version_id)).into());
        }

        let content = std::fs::read_to_string(version_file)?;
        let details: VersionDetails = serde_json::from_str(&content)?;
        Ok(details)
    }

    pub fn get_version_jar_path(&self, version_id: &str) -> PathBuf {
        self.versions_dir
            .join(version_id)
            .join(format!("{}.jar", version_id))
    }

    pub fn get_libraries_dir(&self) -> PathBuf {
        self.versions_dir.join("libraries")
    }

    pub async fn get_version_manifest(&mut self) -> Result<&VersionManifest> {
        if self.cached_manifest.is_none() {
            self.update_manifest().await?;
        }
        
        Ok(self.cached_manifest.as_ref().unwrap())
    }

    pub async fn load_versions(&mut self) -> Result<()> {
        let manifest_path = self.versions_dir.join("version_manifest.json");
        let cache_time_path = self.versions_dir.join("manifest_cache_time");
        
        let should_update = if manifest_path.exists() && cache_time_path.exists() {
            if let Ok(cache_time_str) = std::fs::read_to_string(&cache_time_path) {
                if let Ok(cache_time) = cache_time_str.parse::<i64>() {
                    let cache_datetime = DateTime::from_timestamp(cache_time, 0).unwrap_or_default();
                    let now = Utc::now();
                    let hours_diff = (now - cache_datetime).num_hours();
                    hours_diff >= 4
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        };

        if should_update {
            let manifest_url = "https://launchermeta.mojang.com/mc/game/version_manifest.json";
            let manifest: VersionManifest = self.network.get_json(manifest_url).await?;
            
            let manifest_json = serde_json::to_string_pretty(&manifest)?;
            std::fs::write(&manifest_path, manifest_json)?;
            std::fs::write(&cache_time_path, Utc::now().timestamp().to_string())?;
            
            self.versions = manifest.versions.clone();
            self.cached_manifest = Some(manifest);
        } else {
            let manifest_content = std::fs::read_to_string(&manifest_path)?;
            let manifest: VersionManifest = serde_json::from_str(&manifest_content)?;
            self.versions = manifest.versions.clone();
            self.cached_manifest = Some(manifest);
        }
        
        Ok(())
    }

    pub async fn force_refresh_manifest(&mut self) -> Result<()> {
        let manifest_url = "https://launchermeta.mojang.com/mc/game/version_manifest.json";
        let manifest: VersionManifest = self.network.get_json(manifest_url).await?;
        
        let manifest_path = self.versions_dir.join("version_manifest.json");
        let cache_time_path = self.versions_dir.join("manifest_cache_time");
        
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        std::fs::write(&manifest_path, manifest_json)?;
        std::fs::write(&cache_time_path, Utc::now().timestamp().to_string())?;
        
        self.versions = manifest.versions.clone();
        self.cached_manifest = Some(manifest);
        Ok(())
    }

    pub fn get_versions(&self) -> &[MinecraftVersion] {
        &self.versions
    }

    pub fn get_installed_versions(&self) -> Vec<MinecraftVersion> {
        self.versions.iter()
            .filter(|version| self.is_version_installed(&version.id))
            .cloned()
            .collect()
    }
}