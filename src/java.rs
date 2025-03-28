use std::path::{Path, PathBuf};
use tokio::fs;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::Command;

const JAVA_DIR: &str = "mangoenv/java";
const MINECRAFT_DIR: &str = "mangoenv/.minecraft";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaVersion {
    pub major: u8,
    pub path: PathBuf,
    pub default: bool,
}

pub struct JavaManager {
    java_dir: PathBuf,
    minecraft_dir: PathBuf,
    versions: Vec<JavaVersion>,
}

impl JavaManager {
    pub fn new() -> Self {
        Self {
            java_dir: PathBuf::from(JAVA_DIR),
            minecraft_dir: PathBuf::from(MINECRAFT_DIR),
            versions: Vec::new(),
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        // Создаем структуру директорий
        fs::create_dir_all(&self.java_dir).await?;
        fs::create_dir_all(&self.minecraft_dir).await?;
        
        // Сканируем установленные версии Java
        self.scan_versions().await?;
        Ok(())
    }

    async fn scan_versions(&mut self) -> Result<()> {
        self.versions.clear();
        
        if self.java_dir.exists() {
            let mut entries = fs::read_dir(&self.java_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                if entry.file_type().await?.is_dir() {
                    if let Some(java_version) = self.check_java_version(&entry.path()).await? {
                        self.versions.push(java_version);
                    }
                }
            }
        }
        Ok(())
    }

    async fn check_java_version(&self, path: &Path) -> Result<Option<JavaVersion>> {
        let java_path = if cfg!(windows) {
            path.join("bin").join("java.exe")
        } else {
            path.join("bin").join("java")
        };

        if !java_path.exists() {
            return Ok(None);
        }

        // Проверяем версию Java
        let output = Command::new(&java_path)
            .arg("-version")
            .output()?;

        let version_str = String::from_utf8_lossy(&output.stderr);
        if let Some(major) = parse_java_version(&version_str) {
            Ok(Some(JavaVersion {
                major,
                path: java_path,
                default: false,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_java_for_minecraft(&mut self, minecraft_version: &str) -> Result<Option<JavaVersion>> {
        let required_java = get_required_java_version(minecraft_version);
        
        // Ищем подходящую установленную версию
        if let Some(java) = self.versions.iter()
            .find(|j| j.major == required_java)
            .cloned() {
            return Ok(Some(java));
        }

        // TODO: Если не нашли, предложить скачать
        Ok(None)
    }
}

fn parse_java_version(version_str: &str) -> Option<u8> {
    // Примеры строк:
    // openjdk version "1.8.0_292"
    // openjdk version "11.0.12"
    // openjdk version "17.0.1"
    let v = version_str.lines().next()?;
    if v.contains("1.8.") {
        Some(8)
    } else {
        v.split('"')
            .nth(1)?
            .split('.')
            .next()?
            .parse()
            .ok()
    }
}

fn get_required_java_version(minecraft_version: &str) -> u8 {
    // Правила:
    // - До 1.16 включительно: Java 8
    // - 1.17+: Java 17
    // - Некоторые снапшоты 1.20+: Java 20
    let version_num = minecraft_version
        .split('.')
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);

    match version_num {
        0..=16 => 8,
        17..=19 => 17,
        _ => 20,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_java_version() {
        assert_eq!(parse_java_version("openjdk version \"1.8.0_292\""), Some(8));
        assert_eq!(parse_java_version("openjdk version \"11.0.12\""), Some(11));
        assert_eq!(parse_java_version("openjdk version \"17.0.1\""), Some(17));
    }

    #[test]
    fn test_get_required_java_version() {
        assert_eq!(get_required_java_version("1.8.9"), 8);
        assert_eq!(get_required_java_version("1.16.5"), 8);
        assert_eq!(get_required_java_version("1.17"), 17);
        assert_eq!(get_required_java_version("1.17.1"), 17);
        assert_eq!(get_required_java_version("1.20.1"), 20);
    }
} 