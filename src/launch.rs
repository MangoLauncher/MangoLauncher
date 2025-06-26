use std::collections::HashMap;

use std::path::{Path, PathBuf};
use std::process::Stdio;

use uuid::Uuid;
use tokio::process::{Child, Command};
use crate::Result;
use crate::instance::Instance;
use crate::profile::{Profile, LaunchProfile};
use crate::java::JavaInstallation;
use crate::logs::{LogManager, LogLevel};
use tokio::io::{AsyncBufReadExt, BufReader};


#[derive(Debug, Clone)]
pub struct LaunchContext {
    pub instance: Instance,
    pub profile: Profile,
    pub java_installation: JavaInstallation,
    pub game_directory: PathBuf,
    pub assets_directory: PathBuf,
    pub libraries_directory: PathBuf,
    pub natives_directory: PathBuf,
    pub version_jar_path: PathBuf,
    pub offline_mode: bool,
    pub demo_mode: bool,
}

pub struct LaunchTask {
    pub context: LaunchContext,
    pub steps: Vec<Box<dyn LaunchStep>>,
    pub current_step: usize,
    pub process: Option<Child>,
}

impl std::fmt::Debug for LaunchTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LaunchTask")
            .field("context", &self.context)
            .field("current_step", &self.current_step)
            .field("steps_count", &self.steps.len())
            .finish()
    }
}

#[async_trait::async_trait]
pub trait LaunchStep: Send + Sync {
    async fn execute(&mut self, context: &LaunchContext) -> Result<()>;
    fn name(&self) -> &str;
}

pub struct CreateDirectoriesStep;

#[async_trait::async_trait]
impl LaunchStep for CreateDirectoriesStep {
    async fn execute(&mut self, context: &LaunchContext) -> Result<()> {
        tokio::fs::create_dir_all(&context.game_directory).await?;
        tokio::fs::create_dir_all(&context.assets_directory).await?;
        tokio::fs::create_dir_all(&context.libraries_directory).await?;
        tokio::fs::create_dir_all(&context.natives_directory).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "Создание директорий"
    }
}

pub struct ExtractNativesStep {
    pub libraries: Vec<PathBuf>,
}

#[async_trait::async_trait]
impl LaunchStep for ExtractNativesStep {
    async fn execute(&mut self, context: &LaunchContext) -> Result<()> {
        for library_path in &self.libraries {
            if library_path.extension().and_then(|s| s.to_str()) == Some("jar") {
                let file = std::fs::File::open(library_path)?;
                let mut archive = zip::ZipArchive::new(file)?;
                
                for i in 0..archive.len() {
                    let mut file = archive.by_index(i)?;
                    if file.name().ends_with(".dll") || 
                       file.name().ends_with(".so") || 
                       file.name().ends_with(".dylib") {
                        
                        let output_path = context.natives_directory.join(
                            Path::new(file.name()).file_name().unwrap()
                        );
                        
                        let mut output_file = std::fs::File::create(output_path)?;
                        std::io::copy(&mut file, &mut output_file)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Извлечение нативных библиотек"
    }
}

pub struct BuildClasspathStep {
    pub libraries: Vec<PathBuf>,
    pub classpath: Vec<PathBuf>,
}

#[async_trait::async_trait]
impl LaunchStep for BuildClasspathStep {
    async fn execute(&mut self, context: &LaunchContext) -> Result<()> {
        self.classpath.clear();
        
        for library in &self.libraries {
            self.classpath.push(library.clone());
        }
        
        self.classpath.push(context.version_jar_path.clone());
        
        Ok(())
    }

    fn name(&self) -> &str {
        "Построение classpath"
    }
}

pub struct LaunchMinecraftStep {
    pub launch_profile: LaunchProfile,
}

#[async_trait::async_trait]
impl LaunchStep for LaunchMinecraftStep {
    async fn execute(&mut self, context: &LaunchContext) -> Result<()> {
        let java_path = &context.java_installation.path;
        
        let mut command = std::process::Command::new(java_path);
        
        let classpath_str = self.launch_profile.classpath
            .iter()
            .map(|p| p.to_string_lossy())
            .collect::<Vec<_>>()
            .join(if cfg!(windows) { ";" } else { ":" });
        
        let mut jvm_args = self.launch_profile.jvm_arguments.clone();
        
        for arg in &mut jvm_args {
            *arg = arg
                .replace("${natives_directory}", &context.natives_directory.to_string_lossy())
                .replace("${classpath}", &classpath_str)
                .replace("${launcher_name}", "mango-launcher")
                .replace("${launcher_version}", "1.0.0");
        }
        
        command.args(&jvm_args);
        command.arg(&self.launch_profile.main_class);
        
        let mut minecraft_args = self.launch_profile.minecraft_arguments.clone();
        for arg in &mut minecraft_args {
            *arg = arg
                .replace("${auth_player_name}", &context.profile.username)
                .replace("${version_name}", &self.launch_profile.minecraft_version)
                .replace("${game_directory}", &context.game_directory.to_string_lossy())
                .replace("${assets_root}", &context.assets_directory.to_string_lossy())
                .replace("${assets_index_name}", &self.launch_profile.assets_index)
                .replace("${auth_uuid}", "00000000-0000-0000-0000-000000000000")
                .replace("${auth_access_token}", "0")
                .replace("${clientid}", "00000000-0000-0000-0000-000000000000")
                .replace("${auth_xuid}", "0")
                .replace("${user_type}", "legacy");
        }
        
        command.args(&minecraft_args);
        command.current_dir(&context.game_directory);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        
        log::info!("Запуск Minecraft: {:?}", command);
        
        Ok(())
    }

    fn name(&self) -> &str {
        "Запуск Minecraft"
    }
}

impl LaunchTask {
    pub fn new(context: LaunchContext) -> Self {
        Self {
            context,
            steps: Vec::new(),
            current_step: 0,
            process: None,
        }
    }

    pub fn add_step(&mut self, step: Box<dyn LaunchStep>) {
        self.steps.push(step);
    }

    pub async fn execute(&mut self) -> Result<()> {
        for (i, step) in self.steps.iter_mut().enumerate() {
            self.current_step = i;
            log::info!("Выполнение шага: {}", step.name());
            step.execute(&self.context).await?;
        }
        Ok(())
    }

    pub fn progress(&self) -> f32 {
        if self.steps.is_empty() {
            return 1.0;
        }
        self.current_step as f32 / self.steps.len() as f32
    }

    pub fn current_step_name(&self) -> Option<&str> {
        self.steps.get(self.current_step).map(|step| step.name())
    }
}

pub struct LaunchManager {
    running_instances: HashMap<Uuid, LaunchTask>,
    log_manager: Option<LogManager>,
}

impl LaunchManager {
    pub fn new() -> Self {
        Self {
            running_instances: HashMap::new(),
            log_manager: None,
        }
    }

    pub fn set_log_manager(&mut self, log_manager: LogManager) {
        self.log_manager = Some(log_manager);
    }

    pub async fn launch_instance(
        &mut self,
        instance: Instance,
        profile: Profile,
        java_installation: JavaInstallation,
        offline_mode: bool,
        demo_mode: bool,
    ) -> Result<Uuid> {
        let launch_id = Uuid::new_v4();
        
        let game_directory = instance.path.join(".minecraft");
        let assets_directory = game_directory.join("assets");
        let libraries_directory = game_directory.join("libraries");
        let natives_directory = game_directory.join("natives");
        let version_jar_path = libraries_directory
            .join("versions")
            .join(&instance.minecraft_version)
            .join(format!("{}.jar", instance.minecraft_version));

        let context = LaunchContext {
            instance,
            profile,
            java_installation,
            game_directory,
            assets_directory,
            libraries_directory,
            natives_directory,
            version_jar_path,
            offline_mode,
            demo_mode,
        };

        let mut task = LaunchTask::new(context);
        
        task.add_step(Box::new(CreateDirectoriesStep));
        task.add_step(Box::new(ExtractNativesStep { libraries: Vec::new() }));
        task.add_step(Box::new(BuildClasspathStep { 
            libraries: Vec::new(),
            classpath: Vec::new(),
        }));
        
        self.running_instances.insert(launch_id, task);
        
        Ok(launch_id)
    }

    pub async fn execute_launch(&mut self, launch_id: Uuid) -> Result<()> {
        if let Some(task) = self.running_instances.get_mut(&launch_id) {
            task.execute().await?;
        }
        Ok(())
    }

    pub fn get_launch_progress(&self, launch_id: Uuid) -> Option<f32> {
        self.running_instances.get(&launch_id).map(|task| task.progress())
    }

    pub fn get_current_step(&self, launch_id: Uuid) -> Option<&str> {
        self.running_instances
            .get(&launch_id)
            .and_then(|task| task.current_step_name())
    }

    pub fn kill_instance(&mut self, launch_id: Uuid) -> Result<()> {
        if let Some(mut task) = self.running_instances.remove(&launch_id) {
            if let Some(mut process) = task.process.take() {
                let _ = process.kill();
            }
        }
        Ok(())
    }

    pub fn is_running(&self, launch_id: Uuid) -> bool {
        self.running_instances.contains_key(&launch_id)
    }

    pub fn list_running(&self) -> Vec<Uuid> {
        self.running_instances.keys().copied().collect()
    }

    pub async fn launch_minecraft(
        &mut self,
        instance: &Instance,
        account: &crate::auth::Account,
        java: &JavaInstallation,
        version_manager: &crate::version::VersionManager,
        data_dir: &PathBuf,
    ) -> Result<()> {
        let instance_dir = data_dir.join("instances").join(instance.id.to_string());
        let minecraft_dir = instance_dir.join(".minecraft");
        let natives_dir = minecraft_dir.join("natives");
        
        tokio::fs::create_dir_all(&minecraft_dir).await?;
        tokio::fs::create_dir_all(&natives_dir).await?;
        
        let version_details = version_manager.get_version_details(&instance.minecraft_version)?;
        let version_jar = version_manager.get_version_jar_path(&instance.minecraft_version);
        
        if !version_jar.exists() {
            return Err(crate::Error::Other(format!("Version JAR not found: {}", version_jar.display())));
        }
        
        let libraries_dir = version_manager.get_libraries_dir();
        let mut classpath_entries = Vec::new();
        
        if let Some(libraries) = &version_details.libraries {
            for library in libraries {
                if let Some(downloads) = &library.downloads {
                    if let Some(artifact) = &downloads.artifact {
                        let lib_path = libraries_dir.join(&artifact.path);
                        if lib_path.exists() {
                            classpath_entries.push(lib_path);
                        } else {
                            log::warn!("Library not found: {}", lib_path.display());
                        }
                    }
                }
            }
        }
        
        classpath_entries.push(version_jar);
        
        let classpath = classpath_entries
            .iter()
            .map(|p| p.to_string_lossy())
            .collect::<Vec<_>>()
            .join(if cfg!(windows) { ";" } else { ":" });
        
        let mut cmd = Command::new(&java.path);
        
        #[cfg(target_os = "macos")]
        cmd.arg("-XstartOnFirstThread");
        
        cmd.arg(format!("-Djava.library.path={}", natives_dir.to_string_lossy()));
        cmd.arg(format!("-Xms{}M", instance.memory_min.unwrap_or(1024)));
        cmd.arg(format!("-Xmx{}M", instance.memory_max.unwrap_or(4096)));
        
        if let Some(java_args) = &instance.java_args {
            for arg in java_args.split_whitespace() {
                cmd.arg(arg);
            }
        }
        
        cmd.arg("-cp").arg(&classpath);
        
        if let Some(main_class) = &version_details.main_class {
            cmd.arg(main_class);
        } else {
            cmd.arg("net.minecraft.client.main.Main");
        }
        
        cmd.arg("--username").arg(&account.display_name);
        cmd.arg("--version").arg(&instance.minecraft_version);
        cmd.arg("--gameDir").arg(minecraft_dir.to_string_lossy().as_ref());
        cmd.arg("--userType").arg(if account.account_type == crate::auth::AccountType::Offline { "legacy" } else { "msa" });
        
        if let Some(uuid) = &account.uuid {
            cmd.arg("--uuid").arg(uuid);
        }
        
        if let Some(token) = &account.access_token {
            cmd.arg("--accessToken").arg(token);
        }
        
        if let Some(width) = instance.width {
            cmd.arg("--width").arg(width.to_string());
        }
        if let Some(height) = instance.height {
            cmd.arg("--height").arg(height.to_string());
        }
        if instance.fullscreen {
            cmd.arg("--fullscreen");
        }
        
        cmd.current_dir(&minecraft_dir);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        log::info!("Запуск Minecraft: {:?}", cmd);
        
        let mut child = cmd.spawn()?;
        
        let log_manager_stdout = self.log_manager.clone();
        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Some(ref log_manager) = log_manager_stdout {
                        Self::parse_and_log_with_manager(log_manager, &line, false);
                    } else {
                        Self::parse_and_log_minecraft_line(&line, false);
                    }
                }
            });
        }
        
        let log_manager_stderr = self.log_manager.clone();
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Some(ref log_manager) = log_manager_stderr {
                        Self::parse_and_log_with_manager(log_manager, &line, true);
                    } else {
                        Self::parse_and_log_minecraft_line(&line, true);
                    }
                }
            });
        }
        
        tokio::spawn(async move {
            let _ = child.wait().await;
            log::info!("Minecraft процесс завершен");
        });
        
        Ok(())
    }

    fn parse_and_log_with_manager(log_manager: &LogManager, line: &str, is_stderr: bool) {
        if let Some(parsed) = Self::parse_minecraft_log_line(line) {
            let level = LogLevel::from_minecraft_level(&parsed.level);
            let source = if parsed.source.is_empty() { 
                "Minecraft".to_string() 
            } else { 
                format!("Minecraft/{}", parsed.source) 
            };
            
            let formatted = format!("!![{}]! {}", parsed.level.to_uppercase(), parsed.message);
            log_manager.log(level, formatted, Some(source));
        } else {
            if is_stderr {
                log_manager.log(LogLevel::Error, format!("!![ERROR]! {}", line), Some("Minecraft".to_string()));
            } else {
                log_manager.log(LogLevel::Info, format!("!![INFO]! {}", line), Some("Minecraft".to_string()));
            }
        }
    }

    fn parse_and_log_minecraft_line(line: &str, is_stderr: bool) {
    
        if let Some(parsed) = Self::parse_minecraft_log_line(line) {
            let level_str = parsed.level.to_uppercase();
            let _source = if parsed.source.is_empty() { 
                "Minecraft".to_string() 
            } else { 
                format!("Minecraft/{}", parsed.source) 
            };
            
        
            let formatted = format!("!![{}]! {}", level_str, parsed.message);
            
            match parsed.level.to_lowercase().as_str() {
                "error" | "fatal" => log::error!("{}", formatted),
                "warn" | "warning" => log::warn!("{}", formatted),
                "debug" => log::debug!("{}", formatted),
                _ => log::info!("{}", formatted),
            }
        } else {
        
            if is_stderr {
                log::warn!("!![ERROR]! {}", line);
            } else {
                log::info!("!![INFO]! {}", line);
            }
        }
    }
    
    fn parse_minecraft_log_line(line: &str) -> Option<MinecraftLogEntry> {
    
        if let Some(start) = line.find('[') {
            if let Some(time_end) = line[start..].find(']') {
                let remaining = &line[start+time_end+1..].trim_start();
                
                if let Some(thread_start) = remaining.find('[') {
                    if let Some(thread_end) = remaining[thread_start..].find(']') {
                        let thread_level = &remaining[thread_start+1..thread_start+thread_end];
                        let after_thread = &remaining[thread_start+thread_end+1..].trim_start();
                        
                        let level = if let Some(slash_pos) = thread_level.find('/') {
                            thread_level[slash_pos+1..].to_string()
                        } else {
                            thread_level.to_string()
                        };
                        
                    
                        let (source, message) = if let Some(source_start) = after_thread.find('[') {
                            if let Some(source_end) = after_thread[source_start..].find(']') {
                                let source = &after_thread[source_start+1..source_start+source_end];
                                let message = &after_thread[source_start+source_end+1..].trim_start();
                                let message = if message.starts_with(':') {
                                    message[1..].trim()
                                } else {
                                    message
                                };
                                (source.to_string(), message.to_string())
                            } else {
                                ("".to_string(), after_thread.to_string())
                            }
                        } else {
                            ("".to_string(), after_thread.to_string())
                        };
                        
                        return Some(MinecraftLogEntry {
                            level,
                            source,
                            message,
                        });
                    }
                }
            }
        }
        None
    }
}

#[derive(Debug)]
struct MinecraftLogEntry {
    level: String,
    source: String,
    message: String,
} 