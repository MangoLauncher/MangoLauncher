use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::process::Command as AsyncCommand;
use crate::{Result, Error};



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaInstallation {
    pub path: PathBuf,
    pub version: String,
    pub vendor: String,
    pub architecture: String,
    pub is_64bit: bool,
    pub is_default: bool,
    pub capabilities: JavaCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaCapabilities {
    pub supports_minecraft: bool,
    pub max_memory: Option<u64>,
    pub supports_javafx: bool,
    pub supports_awt: bool,
}

pub struct JavaManager {
    installations: HashMap<String, JavaInstallation>,
    java_directory: Option<PathBuf>,
    default_installation: Option<String>,
}

impl JavaManager {
    pub fn new(java_directory: Option<PathBuf>) -> Result<Self> {
        Ok(Self {
            installations: HashMap::new(),
            java_directory,
            default_installation: None,
        })
    }

    pub async fn scan_java_installations(&mut self) -> Result<()> {
        self.installations.clear();
        
        let search_paths = self.get_search_paths();
        
        for path in search_paths {
            if path.exists() {
                self.scan_directory_recursive(&path).await?;
            }
        }
        
        if self.installations.is_empty() {
            return Err(Error::Java("No Java installations found".to_string()));
        }
        
        self.select_default_installation();
        Ok(())
    }

    async fn scan_directory_recursive(&mut self, dir_path: &PathBuf) -> Result<()> {
        let mut dirs_to_scan = vec![dir_path.clone()];
        
        while let Some(current_dir) = dirs_to_scan.pop() {
            if current_dir.components().count() > 8 {
                continue;
            }
            
            if let Ok(entries) = std::fs::read_dir(&current_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    
                    if self.is_java_executable(&path) {
                        if let Ok(installation) = self.create_java_installation(path).await {
                            let key = format!("{} {}", installation.vendor, installation.version);
                            self.installations.insert(key, installation);
                        }
                    } else if path.is_dir() {
                        #[cfg(target_os = "macos")]
                        {
                            if path.extension().and_then(|s| s.to_str()) == Some("jdk") {
                                let java_home = path.join("Contents").join("Home");
                                if java_home.exists() {
                                    let java_bin = java_home.join("bin");
                                    if java_bin.exists() {
                                        dirs_to_scan.push(java_bin);
                                    }
                                }
                            }
                        }
                        dirs_to_scan.push(path);
                    }
                }
            }
        }
        
        if cfg!(target_os = "macos") && self.installations.is_empty() {
            self.scan_system_java().await?;
        }
        
        Ok(())
    }
    
    #[cfg(target_os = "macos")]
    async fn scan_system_java(&mut self) -> Result<()> {
        let system_java = PathBuf::from("/usr/bin/java");
        if system_java.exists() {
            if let Ok(installation) = self.create_java_installation(system_java).await {
                let key = format!("{} {}", installation.vendor, installation.version);
                self.installations.insert(key, installation);
            }
        }
        Ok(())
    }
    
    #[cfg(not(target_os = "macos"))]
    async fn scan_system_java(&mut self) -> Result<()> {
        Ok(())
    }

    fn select_default_installation(&mut self) {
        let mut best_installation: Option<(String, u32)> = None;
        
        for (key, installation) in &self.installations {
            let score = self.calculate_java_score(installation);
            
            match &best_installation {
                None => best_installation = Some((key.clone(), score)),
                Some((_, best_score)) => {
                    if score > *best_score {
                        best_installation = Some((key.clone(), score));
                    }
                }
            }
        }
        
        if let Some((key, _)) = best_installation {
            self.default_installation = Some(key.clone());
            if let Some(installation) = self.installations.get_mut(&key) {
                installation.is_default = true;
            }
        }
    }

    fn calculate_java_score(&self, installation: &JavaInstallation) -> u32 {
        let mut score = 0u32;
        
        if let Some(major_version) = installation.version.split('.').next()
            .and_then(|v| v.parse::<u32>().ok()) {
            score += match major_version {
                21 => 100,
                17 => 90,
                11 => 70,
                8 => 50,
                _ if major_version > 21 => 80,
                _ => 20,
            };
        }
        
        if installation.is_64bit {
            score += 25;
        }
        
        score += match installation.vendor.to_lowercase().as_str() {
            vendor if vendor.contains("adoptium") || vendor.contains("eclipse") => 30,
            vendor if vendor.contains("openjdk") => 25,
            vendor if vendor.contains("oracle") => 20,
            vendor if vendor.contains("amazon") => 25,
            vendor if vendor.contains("azul") => 20,
            vendor if vendor.contains("ibm") => 15,
            _ => 10,
        };

        score
    }

    fn is_java_executable(&self, path: &PathBuf) -> bool {
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            #[cfg(windows)]
            return file_name == "java.exe" || file_name == "javaw.exe";
            
            #[cfg(not(windows))]
            return file_name == "java";
        }
        false
    }

    async fn create_java_installation(&self, java_path: PathBuf) -> Result<JavaInstallation> {
        let version_output = AsyncCommand::new(&java_path)
            .arg("-version")
            .output()
            .await?;

        let version_str = String::from_utf8_lossy(&version_output.stderr);
        let properties_output = AsyncCommand::new(&java_path)
            .args(["-XshowSettings:properties", "-version"])
            .output()
            .await?;

        let properties_str = String::from_utf8_lossy(&properties_output.stderr);
        
        let version = self.parse_java_version(&version_str)?;
        let vendor = self.parse_java_vendor(&version_str, &properties_str);
        let architecture = self.parse_java_architecture(&properties_str);
        let is_64bit = architecture.contains("64");

        let capabilities = JavaCapabilities {
            supports_minecraft: self.check_minecraft_compatibility(&version),
            max_memory: self.detect_max_memory(&properties_str),
            supports_javafx: self.check_javafx_support(&java_path).await,
            supports_awt: self.check_awt_support(&properties_str),
        };

        Ok(JavaInstallation {
            path: java_path,
            version,
            vendor,
            architecture,
            is_64bit,
            is_default: false,
            capabilities,
        })
    }

    fn parse_java_vendor(&self, version_output: &str, properties_output: &str) -> String {
        for line in version_output.lines() {
            let line_lower = line.to_lowercase();
            if line_lower.contains("adoptium") || line_lower.contains("eclipse") {
                return "Eclipse Adoptium".to_string();
            } else if line_lower.contains("openjdk") {
                return "OpenJDK".to_string();
            } else if line_lower.contains("oracle") {
                return "Oracle".to_string();
            } else if line_lower.contains("amazon") {
                return "Amazon Corretto".to_string();
            } else if line_lower.contains("azul") {
                return "Azul Zulu".to_string();
            } else if line_lower.contains("ibm") {
                return "IBM Semeru".to_string();
            }
        }
        
        for line in properties_output.lines() {
            if line.contains("java.vendor") {
                if let Some(vendor) = line.split('=').nth(1) {
                    return vendor.trim().to_string();
                }
            }
        }
        
        "Unknown".to_string()
    }

    fn parse_java_version(&self, output: &str) -> Result<String> {
        for line in output.lines() {
            if line.contains("version") {
                if let Some(version_part) = line.split("version").nth(1) {
                    if let Some(quoted) = version_part.split('"').nth(1) {
                        return Ok(quoted.to_string());
                    }
                    
                    if let Some(unquoted) = version_part.trim().split_whitespace().next() {
                        return Ok(unquoted.to_string());
                    }
                }
            }
        }
        Err(Error::Java("Could not parse Java version".to_string()))
    }

    fn parse_java_architecture(&self, properties_output: &str) -> String {
        for line in properties_output.lines() {
            if line.contains("os.arch") {
                if let Some(arch) = line.split('=').nth(1) {
                    return arch.trim().to_string();
                }
            }
        }
        
        #[cfg(target_arch = "x86_64")]
        return "amd64".to_string();
        
        #[cfg(target_arch = "aarch64")]
        return "aarch64".to_string();
        
        #[cfg(target_arch = "x86")]
        return "x86".to_string();
        
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "x86")))]
        "unknown".to_string()
    }

    fn detect_max_memory(&self, properties_output: &str) -> Option<u64> {
        for line in properties_output.lines() {
            if line.contains("java.runtime.version") {
                return Some(if cfg!(target_pointer_width = "64") {
                    32 * 1024 * 1024 * 1024
                } else {
                    3 * 1024 * 1024 * 1024
                });
            }
        }
        None
    }

    async fn check_javafx_support(&self, java_path: &PathBuf) -> bool {
        let result = AsyncCommand::new(java_path)
            .args(["-cp", ".", "-version"])
            .output()
            .await;
        
        if let Ok(output) = result {
            let output_str = String::from_utf8_lossy(&output.stderr);
            return output_str.contains("javafx") || output_str.contains("JavaFX");
        }
        false
    }

    fn check_awt_support(&self, properties_output: &str) -> bool {
        for line in properties_output.lines() {
            if line.contains("java.awt.headless") {
                return !line.contains("true");
            }
        }
        true
    }

    fn get_search_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        
        if let Some(custom_dir) = &self.java_directory {
            paths.push(custom_dir.clone());
        }
        
        #[cfg(target_os = "windows")]
        {
            paths.extend(vec![
                PathBuf::from("C:\\Program Files\\Java"),
                PathBuf::from("C:\\Program Files (x86)\\Java"),
                PathBuf::from("C:\\Program Files\\Eclipse Adoptium"),
                PathBuf::from("C:\\Program Files (x86)\\Eclipse Adoptium"),
                PathBuf::from("C:\\Program Files\\OpenJDK"),
                PathBuf::from("C:\\Program Files (x86)\\OpenJDK"),
                PathBuf::from("C:\\Program Files\\Amazon Corretto"),
                PathBuf::from("C:\\Program Files (x86)\\Amazon Corretto"),
                PathBuf::from("C:\\Program Files\\Azul\\Zulu"),
                PathBuf::from("C:\\Program Files (x86)\\Azul\\Zulu"),
                PathBuf::from("C:\\Program Files\\BellSoft\\LibericaJDK"),
                PathBuf::from("C:\\Program Files (x86)\\BellSoft\\LibericaJDK"),
            ]);
            
            if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
                paths.push(PathBuf::from(localappdata).join("Programs").join("Eclipse Adoptium"));
            }
            
            if let Ok(programdata) = std::env::var("PROGRAMDATA") {
                paths.push(PathBuf::from(programdata).join("minecraft").join("runtime"));
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            paths.extend(vec![
                PathBuf::from("/Library/Java/JavaVirtualMachines"),
                PathBuf::from("/System/Library/Frameworks/JavaVM.framework/Versions"),
                PathBuf::from("/usr/local/Cellar/openjdk"),
                PathBuf::from("/usr/local/Cellar/adoptopenjdk"),
                PathBuf::from("/opt/homebrew/Cellar/openjdk"),
                PathBuf::from("/opt/homebrew/opt"),
                PathBuf::from("/usr/bin"),
            ]);
            
            if let Ok(home) = std::env::var("HOME") {
                let home_path = PathBuf::from(home);
                paths.push(home_path.join(".sdkman/candidates/java"));
                paths.push(home_path.join("Library/Application Support/minecraft/runtime"));
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            paths.extend(vec![
                PathBuf::from("/usr/lib/jvm"),
                PathBuf::from("/usr/java"),
                PathBuf::from("/opt/java"),
                PathBuf::from("/opt/jdk"),
                PathBuf::from("/opt/openjdk"),
                PathBuf::from("/opt/adoptium"),
                PathBuf::from("/opt/oracle"),
                PathBuf::from("/opt/amazon-corretto"),
                PathBuf::from("/opt/azul"),
                PathBuf::from("/opt/ibm"),
                PathBuf::from("/app/jdk"),
            ]);
            
            paths.extend(vec![
                PathBuf::from("/usr/lib/jvm/java-8-openjdk"),
                PathBuf::from("/usr/lib/jvm/java-11-openjdk"),
                PathBuf::from("/usr/lib/jvm/java-17-openjdk"),
                PathBuf::from("/usr/lib/jvm/java-21-openjdk"),
            ]);
            
            paths.extend(vec![
                PathBuf::from("/usr/lib/jvm/icedtea-8"),
                PathBuf::from("/usr/lib/jvm/icedtea-7"),
                PathBuf::from("/usr/lib/jvm/icedtea-6"),
                PathBuf::from("/usr/lib/jvm/openjdk-8"),
                PathBuf::from("/usr/lib/jvm/openjdk-11"),
                PathBuf::from("/usr/lib/jvm/openjdk-17"),
                PathBuf::from("/usr/lib/jvm/openjdk-21"),
            ]);
            
            if let Ok(home) = std::env::var("HOME") {
                let home_path = PathBuf::from(home);
                paths.push(home_path.join(".sdkman/candidates/java"));
                paths.push(home_path.join(".local/share/JetBrains/Toolbox/apps/IDEA-*/ch-*/*/jbr"));
                paths.push(home_path.join(".gradle/jdks"));
                paths.push(home_path.join(".minecraft/runtime"));
            }
            
            paths.extend(vec![
                PathBuf::from("/var/lib/snapd/snap/openjdk/current"),
                PathBuf::from("/snap/openjdk/current"),
                PathBuf::from("/snap/adoptopenjdk/current"),
            ]);
        }
        
        paths.into_iter().filter(|p| p.exists()).collect()
    }

    pub async fn download_java(&mut self, version: u8) -> Result<JavaInstallation> {
        let os = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "mac"
        } else {
            "linux"
        };
        
        let arch = if cfg!(target_arch = "x86_64") {
            "x64"
        } else if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else {
            "x86"
        };
        
        let _url = format!(
            "https://api.adoptium.net/v3/assets/latest/{}/hotspot?architecture={}&image_type=jdk&os={}",
            version, arch, os
        );
        
        Err(Error::Java("Java download not implemented yet".to_string()))
    }

    pub fn get_installations(&self) -> &HashMap<String, JavaInstallation> {
        &self.installations
    }

    pub fn get_installation(&self, key: &str) -> Option<&JavaInstallation> {
        self.installations.get(key)
    }

    pub fn get_default_installation(&self) -> Option<&JavaInstallation> {
        self.default_installation.as_ref()
            .and_then(|key| self.installations.get(key))
    }

    pub fn set_default_installation(&mut self, key: &str) -> Result<()> {
        if !self.installations.contains_key(key) {
            return Err(Error::Java("Installation not found".to_string()));
        }
        
        if let Some(old_default) = &self.default_installation {
            if let Some(installation) = self.installations.get_mut(old_default) {
                installation.is_default = false;
            }
        }
        
        if let Some(installation) = self.installations.get_mut(key) {
            installation.is_default = true;
        }
        
        self.default_installation = Some(key.to_string());
        Ok(())
    }

    pub fn update_java_directory(&mut self, new_directory: Option<PathBuf>) {
        self.java_directory = new_directory;
    }

    pub fn validate_java_for_minecraft(&self, installation: &JavaInstallation, _minecraft_version: &str) -> bool {
        self.check_minecraft_compatibility(&installation.version)
    }

    fn check_minecraft_compatibility(&self, java_version: &str) -> bool {
        if let Some(major_version) = java_version.split('.').next()
            .and_then(|v| v.parse::<u8>().ok()) {
            return major_version >= 8;
        }
        false
    }

    pub fn get_recommended_java_for_minecraft(&self, minecraft_version: &str) -> Option<u8> {
        let version_parts: Vec<&str> = minecraft_version.split('.').collect();
        
        if version_parts.len() >= 2 {
            let major: u8 = version_parts[1].parse().unwrap_or(0);
            let minor: u8 = if version_parts.len() > 2 {
                version_parts[2].parse().unwrap_or(0)
            } else { 0 };
            
            if major > 1 || (major == 1 && minor >= 17) {
                return Some(17);
            }
        }
        
        Some(8)
    }
}

impl Default for JavaCapabilities {
    fn default() -> Self {
        Self {
            supports_minecraft: true,
            max_memory: None,
            supports_javafx: false,
            supports_awt: true,
        }
    }
} 