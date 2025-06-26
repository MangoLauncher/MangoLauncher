 
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::{Error, Result};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: Uuid,
    pub name: String,
    pub group: Option<String>,
    pub path: PathBuf,
    pub minecraft_version: String,
    pub mod_loader: Option<ModLoader>,
    pub mod_loader_version: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_played: Option<DateTime<Utc>>,
    pub play_time: u64,
    pub icon: Option<String>,
    pub notes: Option<String>,
    pub java_path: Option<PathBuf>,
    pub java_args: Option<String>,
    pub memory_min: Option<u32>,
    pub memory_max: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fullscreen: bool,
    pub auto_connect: Option<String>,
    pub pre_launch_command: Option<String>,
    pub post_launch_command: Option<String>,
    pub disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModLoader {
    Forge,
    Fabric,
    Quilt,
    NeoForge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceGroup {
    pub name: String,
    pub collapsed: bool,
    pub instances: Vec<Uuid>,
}

pub struct InstanceManager {
    instances: HashMap<Uuid, Instance>,
    groups: HashMap<String, InstanceGroup>,
    instances_dir: PathBuf,
}

impl InstanceManager {
    pub fn new(instances_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&instances_dir)?;
        
        let mut manager = Self {
            instances: HashMap::new(),
            groups: HashMap::new(),
            instances_dir,
        };
        
        manager.load_instances()?;
        Ok(manager)
    }

    pub fn create_instance(&mut self, name: String, minecraft_version: String) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let instance_path = self.instances_dir.join(id.to_string());
        
        std::fs::create_dir_all(&instance_path)?;
        std::fs::create_dir_all(instance_path.join(".minecraft"))?;
        std::fs::create_dir_all(instance_path.join("mods"))?;
        std::fs::create_dir_all(instance_path.join("resourcepacks"))?;
        std::fs::create_dir_all(instance_path.join("shaderpacks"))?;
        std::fs::create_dir_all(instance_path.join("saves"))?;
        
        let instance = Instance {
            id,
            name,
            group: None,
            path: instance_path,
            minecraft_version,
            mod_loader: None,
            mod_loader_version: None,
            created_at: Utc::now(),
            last_played: None,
            play_time: 0,
            icon: None,
            notes: None,
            java_path: None,
            java_args: None,
            memory_min: None,
            memory_max: None,
            width: None,
            height: None,
            fullscreen: false,
            auto_connect: None,
            pre_launch_command: None,
            post_launch_command: None,
            disabled: false,
        };
        
        self.save_instance(&instance)?;
        self.instances.insert(id, instance);
        
        Ok(id)
    }

    pub fn delete_instance(&mut self, id: Uuid) -> Result<()> {
        if let Some(instance) = self.instances.remove(&id) {
            std::fs::remove_dir_all(&instance.path)?;
        }
        Ok(())
    }

    pub fn get_instance(&self, id: Uuid) -> Option<&Instance> {
        self.instances.get(&id)
    }

    pub fn get_instance_mut(&mut self, id: Uuid) -> Option<&mut Instance> {
        self.instances.get_mut(&id)
    }

    pub fn list_instances(&self) -> Vec<&Instance> {
        self.instances.values().collect()
    }

    pub fn update_instance(&mut self, instance: Instance) -> Result<()> {
        self.save_instance(&instance)?;
        self.instances.insert(instance.id, instance);
        Ok(())
    }

    pub fn create_group(&mut self, name: String) -> Result<()> {
        if self.groups.contains_key(&name) {
            return Err(Error::Instance(format!("Group '{}' already exists", name)));
        }
        
        let group = InstanceGroup {
            name: name.clone(),
            collapsed: false,
            instances: Vec::new(),
        };
        
        self.groups.insert(name, group);
        self.save_groups()?;
        Ok(())
    }

    pub fn delete_group(&mut self, name: &str) -> Result<()> {
        if let Some(group) = self.groups.remove(name) {
            for instance_id in group.instances {
                if let Some(instance) = self.instances.get_mut(&instance_id) {
                    instance.group = None;
                }
            }
        }
        self.save_groups()?;
        Ok(())
    }

    pub fn add_instance_to_group(&mut self, instance_id: Uuid, group_name: &str) -> Result<()> {
        let old_group = if let Some(instance) = self.instances.get(&instance_id) {
            instance.group.clone()
        } else {
            return Err(Error::Instance("Instance not found".to_string()));
        };

        if let Some(old_group_name) = &old_group {
            if let Some(group) = self.groups.get_mut(old_group_name) {
                group.instances.retain(|&id| id != instance_id);
            }
        }
        
        if let Some(instance) = self.instances.get_mut(&instance_id) {
            instance.group = Some(group_name.to_string());
        }
        
        if let Some(group) = self.groups.get_mut(group_name) {
            if !group.instances.contains(&instance_id) {
                group.instances.push(instance_id);
            }
        }
        
        if let Some(instance) = self.instances.get(&instance_id) {
            self.save_instance(instance)?;
        }
        self.save_groups()?;
        
        Ok(())
    }

    pub fn get_grouped_instances(&self) -> HashMap<Option<String>, Vec<&Instance>> {
        let mut grouped = HashMap::new();
        
        for instance in self.instances.values() {
            let group = instance.group.clone();
            grouped.entry(group).or_insert_with(Vec::new).push(instance);
        }
        
        grouped
    }

    fn load_instances(&mut self) -> Result<()> {
        if !self.instances_dir.exists() {
            return Ok(());
        }
        
        for entry in std::fs::read_dir(&self.instances_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                let config_path = path.join("instance.json");
                if config_path.exists() {
                    match self.load_instance(&config_path) {
                        Ok(instance) => {
                            self.instances.insert(instance.id, instance);
                        }
                        Err(e) => {
                            log::warn!("Failed to load instance from {:?}: {}", config_path, e);
                        }
                    }
                }
            }
        }
        
        self.load_groups()?;
        Ok(())
    }

    fn load_instance(&self, config_path: &Path) -> Result<Instance> {
        let content = std::fs::read_to_string(config_path)?;
        let instance: Instance = serde_json::from_str(&content)?;
        Ok(instance)
    }

    fn save_instance(&self, instance: &Instance) -> Result<()> {
        let config_path = instance.path.join("instance.json");
        let content = serde_json::to_string_pretty(instance)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    fn load_groups(&mut self) -> Result<()> {
        let groups_path = self.instances_dir.join("groups.json");
        if groups_path.exists() {
            let content = std::fs::read_to_string(groups_path)?;
            self.groups = serde_json::from_str(&content)?;
        }
        Ok(())
    }

    fn save_groups(&self) -> Result<()> {
        let groups_path = self.instances_dir.join("groups.json");
        let content = serde_json::to_string_pretty(&self.groups)?;
        std::fs::write(groups_path, content)?;
        Ok(())
    }

    pub fn get_instance_mods_dir(&self, instance_id: Uuid) -> Option<PathBuf> {
        self.get_instance(instance_id).map(|i| i.path.join("mods"))
    }

    pub fn get_instance_resourcepacks_dir(&self, instance_id: Uuid) -> Option<PathBuf> {
        self.get_instance(instance_id).map(|i| i.path.join("resourcepacks"))
    }

    pub fn get_instance_saves_dir(&self, instance_id: Uuid) -> Option<PathBuf> {
        self.get_instance(instance_id).map(|i| i.path.join("saves"))
    }

    pub fn import_instance(&mut self, _import_path: &Path) -> Result<Uuid> {
        Err(Error::Instance("Import not implemented yet".to_string()))
    }

    pub fn export_instance(&self, _instance_id: Uuid, _export_path: &Path) -> Result<()> {
        Err(Error::Instance("Export not implemented yet".to_string()))
    }
} 