 
use std::collections::HashMap;

use std::io::Read;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mod {
    pub id: Uuid,
    pub name: String,
    pub filename: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub mod_id: Option<String>,
    pub enabled: bool,
    pub mod_loader: ModLoader,
    pub minecraft_versions: Vec<String>,
    pub dependencies: Vec<ModDependency>,
    pub size: u64,
    pub hash: String,
    pub source: ModSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModLoader {
    Forge,
    Fabric,
    Quilt,
    NeoForge,
    LiteLoader,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModDependency {
    pub mod_id: String,
    pub version_range: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModSource {
    CurseForge { project_id: u32, file_id: u32 },
    Modrinth { project_id: String, version_id: String },
    Local,
    Unknown,
}

pub struct ModManager {
    mods_dir: PathBuf,
    mods: HashMap<Uuid, Mod>,
    disabled_dir: PathBuf,
}

impl ModManager {
    pub fn new(mods_dir: PathBuf) -> Result<Self> {
        let disabled_dir = mods_dir.join(".disabled");
        
        std::fs::create_dir_all(&mods_dir)?;
        std::fs::create_dir_all(&disabled_dir)?;
        
        let mut manager = Self {
            mods_dir,
            mods: HashMap::new(),
            disabled_dir,
        };
        
        manager.scan_mods()?;
        Ok(manager)
    }

    pub fn scan_mods(&mut self) -> Result<()> {
        self.mods.clear();
        
        let mods_dir = self.mods_dir.clone();
        let disabled_dir = self.disabled_dir.clone();
        
        self.scan_directory(&mods_dir, true)?;
        self.scan_directory(&disabled_dir, false)?;
        
        Ok(())
    }

    fn scan_directory(&mut self, dir: &Path, enabled: bool) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && self.is_mod_file(&path) {
                if let Ok(mod_info) = self.parse_mod_file(&path, enabled) {
                    self.mods.insert(mod_info.id, mod_info);
                }
            }
        }
        
        Ok(())
    }

    fn is_mod_file(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension() {
            extension == "jar" || extension == "zip"
        } else {
            false
        }
    }

    fn parse_mod_file(&self, path: &Path, enabled: bool) -> Result<Mod> {
        let file = std::fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        
        let metadata = std::fs::metadata(path)?;
        let hash = self.calculate_file_hash(path)?;
        
        let mut mod_info = Mod {
            id: Uuid::new_v4(),
            name: path.file_stem().unwrap().to_string_lossy().to_string(),
            filename: path.file_name().unwrap().to_string_lossy().to_string(),
            version: "Unknown".to_string(),
            description: None,
            authors: Vec::new(),
            mod_id: None,
            enabled,
            mod_loader: ModLoader::Forge,
            minecraft_versions: Vec::new(),
            dependencies: Vec::new(),
            size: metadata.len(),
            hash,
            source: ModSource::Local,
        };
        
        let mut found = false;
        if let Ok(mut fabric_file) = archive.by_name("fabric.mod.json") {
            let mut content = String::new();
            fabric_file.read_to_string(&mut content)?;
            drop(fabric_file); 
            self.parse_fabric_mod_from_content(&content, &mut mod_info)?;
            found = true;
        }
        
        if !found {
            if let Ok(mut forge_file) = archive.by_name("mcmod.info") {
                let mut content = String::new();
                forge_file.read_to_string(&mut content)?;
                drop(forge_file);   
                self.parse_forge_mod_from_content(&content, &mut mod_info)?;
                found = true;
            }
        }
        
        if !found {
            if let Ok(mut neoforge_file) = archive.by_name("META-INF/mods.toml") {
                let mut content = String::new();
                neoforge_file.read_to_string(&mut content)?;
                drop(neoforge_file); 
                self.parse_neoforge_mod_from_content(&content, &mut mod_info)?;
            }
        }
        
        Ok(mod_info)
    }

    fn parse_fabric_mod_from_content(&self, content: &str, mod_info: &mut Mod) -> Result<()> {
        let json: serde_json::Value = serde_json::from_str(content)?;
        
        if let Some(name) = json["name"].as_str() {
            mod_info.name = name.to_string();
        }
        
        if let Some(version) = json["version"].as_str() {
            mod_info.version = version.to_string();
        }
        
        if let Some(description) = json["description"].as_str() {
            mod_info.description = Some(description.to_string());
        }
        
        if let Some(id) = json["id"].as_str() {
            mod_info.mod_id = Some(id.to_string());
        }
        
        if let Some(authors) = json["authors"].as_array() {
            for author in authors {
                if let Some(author_str) = author.as_str() {
                    mod_info.authors.push(author_str.to_string());
                }
            }
        }
        
        mod_info.mod_loader = ModLoader::Fabric;
        
        Ok(())
    }

    fn parse_forge_mod_from_content(&self, content: &str, mod_info: &mut Mod) -> Result<()> {
        let json: serde_json::Value = serde_json::from_str(content)?;
        
        if let Some(mods) = json.as_array() {
            if let Some(mod_data) = mods.first() {
                if let Some(name) = mod_data["name"].as_str() {
                    mod_info.name = name.to_string();
                }
                
                if let Some(version) = mod_data["version"].as_str() {
                    mod_info.version = version.to_string();
                }
                
                if let Some(description) = mod_data["description"].as_str() {
                    mod_info.description = Some(description.to_string());
                }
                
                if let Some(id) = mod_data["modid"].as_str() {
                    mod_info.mod_id = Some(id.to_string());
                }
                
                if let Some(authors) = mod_data["authorList"].as_array() {
                    for author in authors {
                        if let Some(author_str) = author.as_str() {
                            mod_info.authors.push(author_str.to_string());
                        }
                    }
                }
            }
        }
        
        mod_info.mod_loader = ModLoader::Forge;
        
        Ok(())
    }

    fn parse_neoforge_mod_from_content(&self, _content: &str, mod_info: &mut Mod) -> Result<()> {
        mod_info.mod_loader = ModLoader::NeoForge;
        Ok(())
    }

    fn calculate_file_hash(&self, path: &Path) -> Result<String> {
        use sha2::{Sha256, Digest};
        
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)?;
        Ok(hex::encode(hasher.finalize()))
    }

    pub fn enable_mod(&mut self, mod_id: Uuid) -> Result<()> {
        if let Some(mod_info) = self.mods.get_mut(&mod_id) {
            if !mod_info.enabled {
                let old_path = self.disabled_dir.join(&mod_info.filename);
                let new_path = self.mods_dir.join(&mod_info.filename);
                
                std::fs::rename(old_path, new_path)?;
                mod_info.enabled = true;
            }
        }
        Ok(())
    }

    pub fn disable_mod(&mut self, mod_id: Uuid) -> Result<()> {
        if let Some(mod_info) = self.mods.get_mut(&mod_id) {
            if mod_info.enabled {
                let old_path = self.mods_dir.join(&mod_info.filename);
                let new_path = self.disabled_dir.join(&mod_info.filename);
                
                std::fs::rename(old_path, new_path)?;
                mod_info.enabled = false;
            }
        }
        Ok(())
    }

    pub fn delete_mod(&mut self, mod_id: Uuid) -> Result<()> {
        if let Some(mod_info) = self.mods.remove(&mod_id) {
            let mod_path = if mod_info.enabled {
                self.mods_dir.join(&mod_info.filename)
            } else {
                self.disabled_dir.join(&mod_info.filename)
            };
            
            std::fs::remove_file(mod_path)?;
        }
        Ok(())
    }

    pub fn install_mod(&mut self, mod_path: &Path) -> Result<Uuid> {
        let target_path = self.mods_dir.join(mod_path.file_name().unwrap());
        std::fs::copy(mod_path, &target_path)?;
        
        let mod_info = self.parse_mod_file(&target_path, true)?;
        let mod_id = mod_info.id;
        self.mods.insert(mod_id, mod_info);
        
        Ok(mod_id)
    }

    pub fn list_mods(&self) -> Vec<&Mod> {
        self.mods.values().collect()
    }

    pub fn get_enabled_mods(&self) -> Vec<&Mod> {
        self.mods.values().filter(|m| m.enabled).collect()
    }

    pub fn get_disabled_mods(&self) -> Vec<&Mod> {
        self.mods.values().filter(|m| !m.enabled).collect()
    }

    pub fn get_mod(&self, mod_id: Uuid) -> Option<&Mod> {
        self.mods.get(&mod_id)
    }

    pub fn search_mods(&self, query: &str) -> Vec<&Mod> {
        let query_lower = query.to_lowercase();
        
        self.mods
            .values()
            .filter(|m| {
                m.name.to_lowercase().contains(&query_lower)
                    || m.description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                    || m.authors.iter().any(|a| a.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    pub fn check_dependencies(&self) -> HashMap<Uuid, Vec<String>> {
        let mut missing_deps = HashMap::new();
        
        for (mod_id, mod_info) in &self.mods {
            if !mod_info.enabled {
                continue;
            }
            
            let mut missing = Vec::new();
            
            for dep in &mod_info.dependencies {
                if dep.required {
                    let found = self.mods.values().any(|m| {
                        m.enabled && m.mod_id.as_ref() == Some(&dep.mod_id)
                    });
                    
                    if !found {
                        missing.push(dep.mod_id.clone());
                    }
                }
            }
            
            if !missing.is_empty() {
                missing_deps.insert(*mod_id, missing);
            }
        }
        
        missing_deps
    }

    pub fn get_mods_by_loader(&self, loader: &ModLoader) -> Vec<&Mod> {
        self.mods
            .values()
            .filter(|m| std::mem::discriminant(&m.mod_loader) == std::mem::discriminant(loader))
            .collect()
    }
} 