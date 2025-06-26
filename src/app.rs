use std::collections::HashMap;
use std::path::PathBuf;

use uuid::Uuid;

use crate::instance::{Instance, InstanceManager};
use crate::assets::AssetsManager;
use crate::auth::{AuthManager, Account};
use crate::java::JavaManager;
use crate::profile::{Profile, ProfileManager};
use crate::network::NetworkManager;
use crate::settings::{Settings, SettingsManager, Language};
use crate::launch::LaunchManager;
use crate::mods::ModManager;
use crate::version::{MinecraftVersion, VersionManager};
use crate::logs::LogManager;
use crate::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    MainMenu,
    InstanceList,
    Settings,
    Launcher,
    AccountManager,
    EditInstance,
}

#[derive(Debug, Clone)]
pub enum Focus {
    InstanceList,
    Settings,
}



pub struct App {
    pub should_quit: bool,
    pub state: AppState,
    pub current_state: String,
    pub focus: Focus,
    pub instance_manager: InstanceManager,
    pub profile_manager: ProfileManager,
    pub settings_manager: SettingsManager,
    pub network_manager: NetworkManager,
    pub java_manager: JavaManager,
    pub version_manager: VersionManager,
    pub assets_manager: AssetsManager,
    pub auth_manager: AuthManager,
    pub launch_manager: LaunchManager,
    pub mod_manager: ModManager,
    pub log_manager: LogManager,
    pub current_motd: String,
    pub current_profile: Option<String>,
    pub profiles: HashMap<String, Profile>,
    pub language: Language,
    pub data_dir: PathBuf,
    pub show_logs: bool,
    pub editing_instance_id: Option<Uuid>,
    pub show_installed_only: bool,
}

impl App {
    pub async fn new() -> Result<Self> {
        let data_dir = crate::utils::get_data_dir()?;
        std::fs::create_dir_all(&data_dir)?;
        
        let settings_manager = SettingsManager::new(data_dir.join("settings.toml"))?;
        let settings = settings_manager.get().clone();
        
        let network_manager = NetworkManager::new(
            data_dir.join("cache"),
            settings.network.max_concurrent_downloads as usize
        );
        let java_manager = JavaManager::new(Some(settings.general.java_directory.clone()))?;
        let instance_manager = InstanceManager::new(data_dir.join("instances"))?;
        let profile_manager = ProfileManager::new(data_dir.join("profiles"))?;
        let version_manager = VersionManager::new(
            data_dir.join("versions"), 
            network_manager.clone(),
            settings.network.max_concurrent_downloads as usize
        )?;
        let log_manager = if settings.advanced.save_logs_to_file {
            LogManager::with_file_logging(
                settings.advanced.console_max_lines as usize,
                settings.advanced.logs_directory.clone(),
                true
            )
        } else {
            LogManager::new(settings.advanced.console_max_lines as usize)
        };
        
        let assets_manager = AssetsManager::new(data_dir.join("assets"), network_manager.clone());
        let auth_manager = AuthManager::new_with_file(data_dir.join("accounts.json"));
        let mut launch_manager = LaunchManager::new();
        launch_manager.set_log_manager(log_manager.clone());
        let mod_manager = ModManager::new(data_dir.join("mods"))?;

        Ok(Self {
            should_quit: false,
            state: AppState::MainMenu,
            current_state: "Загрузка...".to_string(),
            focus: Focus::InstanceList,
            instance_manager,
            profile_manager,
            settings_manager,
            network_manager,
            java_manager,
            version_manager,
            assets_manager,
            auth_manager,
            launch_manager,
            mod_manager,
            log_manager,
            current_motd: "Добро пожаловать в MangoLauncher!".to_string(),
            current_profile: None,
            profiles: HashMap::new(),
            language: settings.general.language.clone(),
            data_dir,
            show_logs: false,
            editing_instance_id: None,
            show_installed_only: true,
        })
    }

    pub async fn init(&mut self) -> Result<()> {
        self.log_launcher("Инициализация MangoLauncher...".to_string(), None);
        
        self.log_info("Сканирование Java...".to_string(), Some("JavaManager".to_string()));
        if let Err(e) = self.scan_java_installations().await {
            self.log_warning(format!("Java не найдена: {} (можно добавить вручную)", e), Some("JavaManager".to_string()));
        }
        
        self.log_info("Загрузка списка версий Minecraft...".to_string(), Some("VersionManager".to_string()));
        self.version_manager.load_versions().await?;
        self.log_info(format!("Загружено {} версий", self.version_manager.get_versions().len()), Some("VersionManager".to_string()));
        
        self.current_state = "Готов".to_string();
        self.log_launcher("Инициализация завершена".to_string(), None);
        Ok(())
    }

    pub async fn force_refresh_versions(&mut self) -> Result<()> {
        self.log_info("Принудительное обновление списка версий...".to_string(), Some("VersionManager".to_string()));
        self.version_manager.force_refresh_manifest().await?;
        self.log_info(format!("Список версий обновлен! Загружено {} версий", self.version_manager.get_versions().len()), Some("VersionManager".to_string()));
        Ok(())
    }

    pub fn get_instances(&self) -> Vec<&Instance> {
        self.instance_manager.list_instances()
    }

    pub fn create_instance(&mut self, name: String, version: String) -> Result<Uuid> {
        self.log_info(format!("Создание экземпляра '{}' версии {}", name, version), Some("InstanceManager".to_string()));
        match self.instance_manager.create_instance(name.clone(), version.clone()) {
            Ok(id) => {
                self.log_info(format!("Экземпляр '{}' успешно создан", name), Some("InstanceManager".to_string()));
                Ok(id)
            }
            Err(e) => {
                self.log_error(format!("Ошибка создания экземпляра '{}': {}", name, e), Some("InstanceManager".to_string()));
                Err(e)
            }
        }
    }

    pub fn delete_instance(&mut self, id: Uuid) -> Result<()> {
        if let Some(instance) = self.instance_manager.get_instance(id) {
            let name = instance.name.clone();
            self.log_warning(format!("Удаление экземпляра '{}'", name), Some("InstanceManager".to_string()));
            match self.instance_manager.delete_instance(id) {
                Ok(_) => {
                    self.log_info(format!("Экземпляр '{}' успешно удален", name), Some("InstanceManager".to_string()));
                    Ok(())
                }
                Err(e) => {
                    self.log_error(format!("Ошибка удаления экземпляра '{}': {}", name, e), Some("InstanceManager".to_string()));
                    Err(e)
                }
            }
        } else {
            self.log_error("Попытка удалить несуществующий экземпляр".to_string(), Some("InstanceManager".to_string()));
            Err(crate::Error::Other("Instance not found".to_string()))
        }
    }

    pub async fn launch_instance(&mut self, id: Uuid) -> Result<()> {
        if let Some(instance) = self.instance_manager.get_instance(id).cloned() {
            let instance_name = instance.name.clone();
            self.current_state = format!("Запуск {}...", instance_name);
            self.log_info(format!("Запуск экземпляра '{}'", instance_name), Some("LaunchManager".to_string()));
            
            if !self.version_manager.is_version_installed(&instance.minecraft_version) {
                self.current_state = format!("Версия {} не скачана!", instance.minecraft_version);
                self.log_error(format!("Версия {} не установлена для экземпляра '{}'", instance.minecraft_version, instance_name), Some("LaunchManager".to_string()));
                return Err(crate::Error::Other(format!("Version {} not installed", instance.minecraft_version)));
            }
            
            let account = self.auth_manager.get_default_account()
                .ok_or_else(|| crate::Error::Auth("No default account set".to_string()))?;
            
            let java = self.java_manager.get_default_installation()
                .ok_or_else(|| crate::Error::Java("No Java installation found".to_string()))?;
            
            match self.launch_manager.launch_minecraft(&instance, account, java, &self.version_manager, &self.data_dir).await {
                Ok(_) => {
                    self.current_state = format!("{} запущен!", instance_name);
                    self.log_info(format!("Экземпляр '{}' успешно запущен", instance_name), Some("LaunchManager".to_string()));
                }
                Err(e) => {
                    self.current_state = format!("Ошибка запуска {}: {}", instance_name, e);
                    self.log_error(format!("Ошибка запуска экземпляра '{}': {}", instance_name, e), Some("LaunchManager".to_string()));
                    return Err(e);
                    }
                }
        } else {
            return Err(crate::Error::Instance("Instance not found".to_string()));
        }
        Ok(())
    }

    pub async fn download_version(&mut self, version_id: &str) -> Result<()> {
        self.log_info(format!("Начинаю загрузку версии {}", version_id), Some("VersionManager".to_string()));
        
        let version = self.version_manager.get_versions()
            .iter()
            .find(|v| v.id == version_id)
            .ok_or_else(|| crate::Error::Version(format!("Version {} not found", version_id)))?
            .clone();
        
        match self.version_manager.download_version(&version).await {
            Ok(_) => {
                self.log_info(format!("Версия {} успешно загружена", version_id), Some("VersionManager".to_string()));
                
                if let Ok(version_details) = self.version_manager.get_version_details(version_id) {
                    if let Some(assets_id) = &version_details.assets {
                        self.log_info(format!("Загрузка ассетов для версии {}", version_id), Some("AssetsManager".to_string()));
                        let assets_url = format!("https://launchermeta.mojang.com/v1/packages/{}/legacy.json", assets_id);
                        
                        match self.assets_manager.download_assets(assets_id, &assets_url).await {
                            Ok(_) => {
                                self.log_info(format!("Ассеты для версии {} успешно загружены", version_id), Some("AssetsManager".to_string()));
                            }
                            Err(e) => {
                                self.log_warning(format!("Ошибка загрузки ассетов для версии {}: {}", version_id, e), Some("AssetsManager".to_string()));
                            }
                        }
                    }
                }
                
                Ok(())
            }
            Err(e) => {
                self.log_error(format!("Ошибка загрузки версии {}: {}", version_id, e), Some("VersionManager".to_string()));
                Err(e.into())
            }
        }
    }

    pub fn get_available_versions(&self) -> &[MinecraftVersion] {
        self.version_manager.get_versions()
    }

    pub fn get_profiles(&self) -> Vec<&Profile> {
        self.profile_manager.list_profiles()
    }

    pub fn get_active_profile(&self) -> Option<&Profile> {
        self.profile_manager.get_active_profile()
    }

    pub fn create_profile(&mut self, name: String) -> Result<Uuid> {
        self.profile_manager.create_profile(name, "Player".to_string())
    }

    pub fn delete_profile(&mut self, id: Uuid) -> Result<()> {
        self.profile_manager.delete_profile(id)
    }

    pub fn get_settings(&self) -> &Settings {
        self.settings_manager.get()
    }

    pub fn get_settings_mut(&mut self) -> &mut Settings {
        self.settings_manager.get_mut()
    }

    pub fn save_settings(&mut self) -> Result<()> {
        self.settings_manager.save()
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn toggle_logs(&mut self) {
        self.show_logs = !self.show_logs;
    }

    pub fn log_info(&self, message: String, source: Option<String>) {
        self.log_manager.info(message, source);
    }

    pub fn log_warning(&self, message: String, source: Option<String>) {
        self.log_manager.warning(message, source);
    }

    pub fn log_error(&self, message: String, source: Option<String>) {
        self.log_manager.error(message, source);
    }

    pub fn log_debug(&self, message: String, source: Option<String>) {
        self.log_manager.debug(message, source);
    }

    pub fn log_launcher(&self, message: String, source: Option<String>) {
        self.log_manager.launcher(message, source);
    }

    pub fn add_offline_account(&mut self, username: String) -> Result<Uuid> {
        let account = Account::new_offline(username.clone());
        self.log_info(format!("Добавление offline аккаунта '{}'", username), Some("AuthManager".to_string()));
        match self.auth_manager.add_account(account) {
            Ok(id) => {
                self.log_info(format!("Offline аккаунт '{}' успешно добавлен", username), Some("AuthManager".to_string()));
                Ok(id)
            }
            Err(e) => {
                self.log_error(format!("Ошибка добавления offline аккаунта '{}': {}", username, e), Some("AuthManager".to_string()));
                Err(e)
            }
        }
    }

    pub fn add_microsoft_account(&mut self, username: String, display_name: String) -> Result<Uuid> {
        let account = Account::new_microsoft(username.clone(), display_name.clone());
        self.log_info(format!("Добавление Microsoft аккаунта '{}'", display_name), Some("AuthManager".to_string()));
        match self.auth_manager.add_account(account) {
            Ok(id) => {
                self.log_info(format!("Microsoft аккаунт '{}' успешно добавлен", display_name), Some("AuthManager".to_string()));
                Ok(id)
            }
            Err(e) => {
                self.log_error(format!("Ошибка добавления Microsoft аккаунта '{}': {}", display_name, e), Some("AuthManager".to_string()));
                Err(e)
            }
        }
    }

    pub fn remove_account(&mut self, account_id: Uuid) -> Result<()> {
        if let Some(account) = self.auth_manager.get_account(account_id) {
            let display_name = account.display_name.clone();
            self.log_warning(format!("Удаление аккаунта '{}'", display_name), Some("AuthManager".to_string()));
            match self.auth_manager.remove_account(account_id) {
                Ok(_) => {
                    self.log_info(format!("Аккаунт '{}' успешно удален", display_name), Some("AuthManager".to_string()));
                    Ok(())
                }
                Err(e) => {
                    self.log_error(format!("Ошибка удаления аккаунта '{}': {}", display_name, e), Some("AuthManager".to_string()));
                    Err(e)
                }
            }
        } else {
            self.log_error("Попытка удалить несуществующий аккаунт".to_string(), Some("AuthManager".to_string()));
            Err(crate::Error::Auth("Account not found".to_string()))
        }
    }

    pub fn set_default_account(&mut self, account_id: Uuid) -> Result<()> {
        if let Some(account) = self.auth_manager.get_account(account_id) {
            let display_name = account.display_name.clone();
            self.log_info(format!("Установка аккаунта '{}' как основного", display_name), Some("AuthManager".to_string()));
            match self.auth_manager.set_default_account(account_id) {
                Ok(_) => {
                    self.log_info(format!("Аккаунт '{}' установлен как основной", display_name), Some("AuthManager".to_string()));
                    Ok(())
                }
                Err(e) => {
                    self.log_error(format!("Ошибка установки аккаунта '{}' как основного: {}", display_name, e), Some("AuthManager".to_string()));
                    Err(e)
                }
            }
        } else {
            Err(crate::Error::Auth("Account not found".to_string()))
        }
    }

    pub fn get_accounts(&self) -> Vec<&Account> {
        self.auth_manager.list_accounts()
    }

    pub fn get_default_account(&self) -> Option<&Account> {
        self.auth_manager.get_default_account()
    }

    pub async fn authenticate_microsoft_account(&mut self, account_id: Uuid) -> Result<()> {
        self.auth_manager.authenticate_microsoft_account(account_id).await
    }


    pub fn start_editing_instance(&mut self, instance_id: Uuid) -> Result<()> {
        if self.instance_manager.get_instance(instance_id).is_some() {
            self.editing_instance_id = Some(instance_id);
            self.state = AppState::EditInstance;
            Ok(())
        } else {
            Err(crate::Error::Instance("Instance not found".to_string()))
        }
    }

    pub fn get_editing_instance(&self) -> Option<&Instance> {
        self.editing_instance_id
            .and_then(|id| self.instance_manager.get_instance(id))
    }

    pub fn get_editing_instance_mut(&mut self) -> Option<&mut Instance> {
        self.editing_instance_id
            .and_then(|id| self.instance_manager.get_instance_mut(id))
    }

    pub fn save_instance_changes(&mut self) -> Result<()> {
        if let Some(instance_id) = self.editing_instance_id {
            if let Some(instance) = self.instance_manager.get_instance(instance_id).cloned() {
                self.instance_manager.update_instance(instance)?;
                self.log_info("Изменения экземпляра сохранены".to_string(), Some("InstanceManager".to_string()));
                Ok(())
            } else {
                Err(crate::Error::Instance("Instance not found".to_string()))
            }
        } else {
            Err(crate::Error::Instance("No instance being edited".to_string()))
        }
    }

    pub fn cancel_instance_editing(&mut self) {
        self.editing_instance_id = None;
        self.state = AppState::InstanceList;
    }

    pub async fn scan_java_installations(&mut self) -> Result<()> {
        self.log_info("Сканирование установок Java...".to_string(), Some("JavaManager".to_string()));
        self.java_manager.update_java_directory(Some(self.settings_manager.get().general.java_directory.clone()));
        match self.java_manager.scan_java_installations().await {
            Ok(_) => {
                let count = self.java_manager.get_installations().len();
                self.log_info(format!("Найдено {} установок Java", count), Some("JavaManager".to_string()));
                Ok(())
            }
            Err(e) => {
                self.log_error(format!("Ошибка сканирования Java: {}", e), Some("JavaManager".to_string()));
                Err(e.into())
            }
        }
    }

    pub fn get_java_installations(&self) -> &HashMap<String, crate::java::JavaInstallation> {
        self.java_manager.get_installations()
    }

    pub fn get_default_java(&self) -> Option<&crate::java::JavaInstallation> {
        self.java_manager.get_default_installation()
    }

    pub fn toggle_version_mode(&mut self) {
        self.show_installed_only = !self.show_installed_only;
        if self.show_installed_only {
            self.current_state = "Показываются скачанные версии".to_string();
        } else {
            self.current_state = "Показываются все доступные версии".to_string();
        }
    }

    pub fn get_displayed_versions(&self) -> Vec<MinecraftVersion> {
        if self.show_installed_only {
            self.version_manager.get_installed_versions()
        } else {
            self.version_manager.get_versions().to_vec()
        }
    }

    pub fn change_account_name(&mut self, account_id: Uuid, new_name: String) -> Result<()> {
        match self.auth_manager.change_account_name(account_id, new_name.clone()) {
            Ok(_) => {
                self.log_info(format!("Ник аккаунта изменен на '{}'", new_name), Some("AuthManager".to_string()));
                Ok(())
            }
            Err(e) => {
                self.log_error(format!("Ошибка изменения ника: {}", e), Some("AuthManager".to_string()));
                Err(e)
            }
        }
    }

    pub fn update_file_logging(&self) {
        let settings = self.settings_manager.get();
        self.log_manager.set_file_logging(
            settings.advanced.save_logs_to_file,
            Some(settings.advanced.logs_directory.clone())
        );
    }

    pub fn update_network_settings(&mut self) {
        let settings = self.settings_manager.get();
        let max_concurrent = settings.network.max_concurrent_downloads as usize;
        
        self.network_manager.set_max_concurrent_downloads(max_concurrent);
        self.version_manager.set_max_concurrent_downloads(max_concurrent);
    }
} 