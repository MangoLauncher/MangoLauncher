use std::collections::HashMap;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccountType {
    Offline,
    Microsoft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: Uuid,
    pub account_type: AccountType,
    pub username: String,
    pub display_name: String,
    pub uuid: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub profile_picture_url: Option<String>,
    pub is_default: bool,
    pub microsoft_data: Option<MicrosoftAccountData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftAccountData {
    pub client_id: String,
    pub xbox_user_token: Option<String>,
    pub xbox_api_token: Option<String>,
    pub mojang_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub gamertag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSession {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub user_type: String,
    pub wants_online: bool,
    pub demo: bool,
}

impl Account {
    pub fn new_offline(username: String) -> Self {
        let uuid = Self::generate_offline_uuid(&username);
        
        Self {
            id: Uuid::new_v4(),
            account_type: AccountType::Offline,
            username: username.clone(),
            display_name: username,
            uuid: Some(uuid),
            access_token: Some("0".to_string()),
            refresh_token: None,
            created_at: Utc::now(),
            last_used: None,
            profile_picture_url: None,
            is_default: false,
            microsoft_data: None,
        }
    }

    pub fn new_microsoft(username: String, display_name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            account_type: AccountType::Microsoft,
            username,
            display_name,
            uuid: None,
            access_token: None,
            refresh_token: None,
            created_at: Utc::now(),
            last_used: None,
            profile_picture_url: None,
            is_default: false,
            microsoft_data: Some(MicrosoftAccountData {
                client_id: String::new(),
                xbox_user_token: None,
                xbox_api_token: None,
                mojang_token: None,
                expires_at: None,
                gamertag: None,
            }),
        }
    }

    pub fn is_valid(&self) -> bool {
        match self.account_type {
            AccountType::Offline => true,
            AccountType::Microsoft => {
                self.access_token.is_some() && 
                self.uuid.is_some() &&
                self.is_token_valid()
            }
        }
    }

    pub fn is_token_valid(&self) -> bool {
        if let Some(microsoft_data) = &self.microsoft_data {
            if let Some(expires_at) = microsoft_data.expires_at {
                return Utc::now() < expires_at;
            }
        }
        true
    }

    pub fn needs_refresh(&self) -> bool {
        match self.account_type {
            AccountType::Offline => false,
            AccountType::Microsoft => !self.is_token_valid() && self.refresh_token.is_some(),
        }
    }

    pub fn create_session(&self) -> Result<GameSession> {
        match self.account_type {
            AccountType::Offline => Ok(GameSession {
                username: self.username.clone(),
                uuid: self.uuid.clone().unwrap_or_else(|| Self::generate_offline_uuid(&self.username)),
                access_token: "0".to_string(),
                user_type: "offline".to_string(),
                wants_online: false,
                demo: false,
            }),
            AccountType::Microsoft => {
                if !self.is_valid() {
                    return Err(Error::Auth("Microsoft account not authenticated".to_string()));
                }
                
                Ok(GameSession {
                    username: self.username.clone(),
                    uuid: self.uuid.clone().ok_or_else(|| Error::Auth("UUID missing".to_string()))?,
                    access_token: self.access_token.clone().ok_or_else(|| Error::Auth("Access token missing".to_string()))?,
                    user_type: "msa".to_string(),
                    wants_online: true,
                    demo: false,
                })
            }
        }
    }

    fn generate_offline_uuid(username: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        format!("OfflinePlayer:{}", username).hash(&mut hasher);
        let hash = hasher.finish();
        
        format!("{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            (hash >> 32) as u32,
            ((hash >> 16) & 0xFFFF) as u16,
            (hash & 0xFFFF) as u16,
            ((hash >> 48) & 0xFFFF) as u16,
            hash & 0xFFFFFFFFFFFF
        )
    }
}

pub struct AuthManager {
    accounts: HashMap<Uuid, Account>,
    default_account: Option<Uuid>,
    accounts_file: PathBuf,
}

impl AuthManager {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            default_account: None,
            accounts_file: PathBuf::from("accounts.json"),
        }
    }

    pub fn new_with_file(accounts_file: PathBuf) -> Self {
        let mut manager = Self {
            accounts: HashMap::new(),
            default_account: None,
            accounts_file,
        };
        
        if let Err(e) = manager.load_accounts() {
            log::warn!("Failed to load accounts: {}", e);
        }
        
        manager
    }

    pub fn add_account(&mut self, mut account: Account) -> Result<Uuid> {
        if self.accounts.is_empty() {
            account.is_default = true;
            self.default_account = Some(account.id);
        }
        
        let id = account.id;
        self.accounts.insert(id, account);
        self.save_accounts()?;
        Ok(id)
    }

    pub fn remove_account(&mut self, account_id: Uuid) -> Result<()> {
        if let Some(account) = self.accounts.remove(&account_id) {
            if account.is_default {
                self.default_account = None;
        
                if let Some((&new_default_id, _)) = self.accounts.iter().next() {
                    self.set_default_account(new_default_id)?;
                }
            }
        }
        self.save_accounts()?;
        Ok(())
    }

    pub fn get_account(&self, account_id: Uuid) -> Option<&Account> {
        self.accounts.get(&account_id)
    }

    pub fn get_account_mut(&mut self, account_id: Uuid) -> Option<&mut Account> {
        self.accounts.get_mut(&account_id)
    }

    pub fn get_default_account(&self) -> Option<&Account> {
        self.default_account.and_then(|id| self.accounts.get(&id))
    }

    pub fn set_default_account(&mut self, account_id: Uuid) -> Result<()> {
        if !self.accounts.contains_key(&account_id) {
            return Err(Error::Auth("Account not found".to_string()));
        }


        if let Some(current_default_id) = self.default_account {
            if let Some(current_default) = self.accounts.get_mut(&current_default_id) {
                current_default.is_default = false;
            }
        }


        if let Some(account) = self.accounts.get_mut(&account_id) {
            account.is_default = true;
        }

        self.default_account = Some(account_id);
        self.save_accounts()?;
        Ok(())
    }

    pub fn list_accounts(&self) -> Vec<&Account> {
        self.accounts.values().collect()
    }

    pub fn get_accounts_by_type(&self, account_type: AccountType) -> Vec<&Account> {
        self.accounts.values()
            .filter(|account| account.account_type == account_type)
            .collect()
    }

    pub fn update_account_last_used(&mut self, account_id: Uuid) -> Result<()> {
        if let Some(account) = self.accounts.get_mut(&account_id) {
            account.last_used = Some(Utc::now());
            self.save_accounts()?;
        }
        Ok(())
    }

    pub async fn authenticate_microsoft_account(&mut self, account_id: Uuid) -> Result<()> {
        if let Some(_account) = self.accounts.get_mut(&account_id) {
            return Err(Error::Auth("Microsoft authentication not implemented yet".to_string()));
        }
        Err(Error::Auth("Account not found".to_string()))
    }

    pub async fn refresh_account(&mut self, account_id: Uuid) -> Result<()> {
        if let Some(account) = self.accounts.get_mut(&account_id) {
            if account.needs_refresh() {
                match account.account_type {
                    AccountType::Microsoft => {
                
                        return Err(Error::Auth("Token refresh not implemented yet".to_string()));
                    }
                    AccountType::Offline => {
                
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }

    fn load_accounts(&mut self) -> Result<()> {
        if !self.accounts_file.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.accounts_file)?;
        let accounts_data: Vec<Account> = serde_json::from_str(&content)?;
        
        for account in accounts_data {
            if account.is_default {
                self.default_account = Some(account.id);
            }
            self.accounts.insert(account.id, account);
        }
        
        Ok(())
    }

    fn save_accounts(&self) -> Result<()> {
        let accounts_vec: Vec<&Account> = self.accounts.values().collect();
        let content = serde_json::to_string_pretty(&accounts_vec)?;
        std::fs::write(&self.accounts_file, content)?;
        Ok(())
    }

    pub fn count(&self) -> usize {
        self.accounts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }

    pub fn change_account_name(&mut self, account_id: Uuid, new_name: String) -> Result<()> {
        if let Some(account) = self.accounts.get_mut(&account_id) {
            account.display_name = new_name.clone();
            if account.account_type == AccountType::Offline {
                account.username = new_name;
                account.uuid = Some(Self::generate_offline_uuid(&account.username));
            }
            self.save_accounts()?;
            Ok(())
        } else {
            Err(Error::Auth("Account not found".to_string()))
        }
    }

    fn generate_offline_uuid(username: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        format!("OfflinePlayer:{}", username).hash(&mut hasher);
        let hash = hasher.finish();
        
        format!("{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            (hash >> 32) as u32,
            ((hash >> 16) & 0xFFFF) as u16,
            (hash & 0xFFFF) as u16,
            ((hash >> 48) & 0xFFFF) as u16,
            hash & 0xFFFFFFFFFFFF
        )
    }
} 