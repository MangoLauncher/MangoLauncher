use std::collections::HashMap;

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::Result;
use crate::network::NetworkManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
    pub virtual_: Option<bool>,
    pub map_to_resources: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

pub struct AssetsManager {
    assets_dir: PathBuf,
    network: NetworkManager,
    indices_cache: HashMap<String, AssetIndex>,
}

impl AssetsManager {
    pub fn new(assets_dir: PathBuf, network: NetworkManager) -> Self {
        std::fs::create_dir_all(&assets_dir).ok();
        std::fs::create_dir_all(assets_dir.join("indexes")).ok();
        std::fs::create_dir_all(assets_dir.join("objects")).ok();
        std::fs::create_dir_all(assets_dir.join("virtual")).ok();

        Self {
            assets_dir,
            network,
            indices_cache: HashMap::new(),
        }
    }

    pub async fn download_assets(&mut self, version: &str, asset_index_url: &str) -> Result<()> {
        let asset_index = self.download_asset_index(version, asset_index_url).await?;
        

        let objects = asset_index.objects.clone();
        
        for (_name, object) in objects {
            let hash = &object.hash;
            let asset_path = self.get_asset_path(hash);
            
            if !asset_path.exists() {
                let download_url = format!(
                    "https://resources.download.minecraft.net/{}/{}",
                    &hash[..2],
                    hash
                );
                
                self.network.download_file(
                    &download_url,
                    &asset_path,
                    Some(hash),
                    None,
                ).await?;
            }
        }

        if asset_index.virtual_.unwrap_or(false) || asset_index.map_to_resources.unwrap_or(false) {
            self.create_virtual_assets(version, &asset_index).await?;
        }

        Ok(())
    }

    async fn download_asset_index(&mut self, version: &str, index_url: &str) -> Result<AssetIndex> {
        if let Some(cached) = self.indices_cache.get(version) {
            return Ok(cached.clone());
        }

        let index_path = self.assets_dir.join("indexes").join(format!("{}.json", version));
        
        if !index_path.exists() {
            self.network.download_file(index_url, &index_path, None, None).await?;
        }

        let index_content = std::fs::read_to_string(&index_path)?;
        let asset_index: AssetIndex = serde_json::from_str(&index_content)?;
        
        self.indices_cache.insert(version.to_string(), asset_index.clone());
        Ok(asset_index)
    }

    async fn create_virtual_assets(&self, version: &str, asset_index: &AssetIndex) -> Result<()> {
        let virtual_dir = self.assets_dir.join("virtual").join(version);
        std::fs::create_dir_all(&virtual_dir)?;

        for (name, object) in &asset_index.objects {
            let asset_path = self.get_asset_path(&object.hash);
            let virtual_path = virtual_dir.join(name);
            
            if let Some(parent) = virtual_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            
            if asset_path.exists() && !virtual_path.exists() {
                std::fs::copy(&asset_path, &virtual_path)?;
            }
        }

        Ok(())
    }

    fn get_asset_path(&self, hash: &str) -> PathBuf {
        self.assets_dir
            .join("objects")
            .join(&hash[..2])
            .join(hash)
    }

    pub fn get_virtual_assets_dir(&self, version: &str) -> PathBuf {
        self.assets_dir.join("virtual").join(version)
    }

    pub fn cleanup_unused_assets(&self, active_versions: &[String]) -> Result<()> {
        let virtual_dir = self.assets_dir.join("virtual");
        
        if virtual_dir.exists() {
            for entry in std::fs::read_dir(&virtual_dir)? {
                let entry = entry?;
                let version_name = entry.file_name().to_string_lossy().to_string();
                
                if !active_versions.contains(&version_name) {
                    std::fs::remove_dir_all(entry.path())?;
                }
            }
        }

        Ok(())
    }

    pub fn get_assets_size(&self) -> Result<u64> {
        let mut total_size = 0;
        
        fn dir_size(path: &Path) -> std::io::Result<u64> {
            let mut size = 0;
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                if metadata.is_dir() {
                    size += dir_size(&entry.path())?;
                } else {
                    size += metadata.len();
                }
            }
            Ok(size)
        }

        if self.assets_dir.exists() {
            total_size = dir_size(&self.assets_dir)?;
        }

        Ok(total_size)
    }
} 