 
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use sha1::{Sha1, Digest};
use tokio::io::AsyncWriteExt;
use crate::{Error, Result};
use crate::progress::ProgressDialog;
use reqwest::Client;
use serde::de::DeserializeOwned;

use ratatui::Terminal;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;


pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

#[derive(Debug, Clone)]
pub struct NetworkManager {
    client: Client,
    cache: Cache,
    max_concurrent_downloads: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub url: String,
    pub file_path: PathBuf,
    pub hash: String,
    pub size: u64,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub etag: Option<String>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HttpCache {
    cache_dir: PathBuf,
    entries: HashMap<String, CacheEntry>,
    max_cache_size: u64,
    max_age: Duration,
}

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub total_bytes: Option<u64>,
    pub downloaded_bytes: u64,
    pub speed_bps: u64,
    pub eta: Option<Duration>,
}

impl NetworkManager {
    pub fn new(_cache_dir: PathBuf, max_concurrent_downloads: usize) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            cache: Cache::new(),
            max_concurrent_downloads,
        }
    }

    pub fn set_max_concurrent_downloads(&mut self, max_concurrent: usize) {
        self.max_concurrent_downloads = max_concurrent;
    }

    pub fn get_max_concurrent_downloads(&self) -> usize {
        self.max_concurrent_downloads
    }

    pub async fn get(&self, url: &str) -> Result<String> {
        let response = self.client.get(url).send().await?;
        let text = response.text().await?;
        Ok(text)
    }

    pub async fn get_json<T>(&self, url: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let text = self.get(url).await?;
        let data = serde_json::from_str(&text)?;
        Ok(data)
    }

    pub async fn download_file(
        &self,
        url: &str,
        path: &Path,
        expected_hash: Option<&str>,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<()> {
        if path.exists() {
            if let Some(hash) = expected_hash {
                let existing_hash = self.calculate_file_hash(path).await?;
                if existing_hash == hash {
                    return Ok(());
                }
            }
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let response = self.client.get(url).send().await?;
        let total_size = response.content_length().unwrap_or(0);

        let mut file = tokio::fs::File::create(path).await?;
        let mut downloaded = 0u64;

        let bytes = response.bytes().await?;
        let mut pos = 0;
        let chunk_size = 8192;
        
        while pos < bytes.len() {
            let end = std::cmp::min(pos + chunk_size, bytes.len());
            let chunk = &bytes[pos..end];
            
            file.write_all(chunk).await?;
            
            downloaded += chunk.len() as u64;
            
            if let Some(ref callback) = progress_callback {
                callback(downloaded, total_size);
            }
            
                        pos = end;
        }
        

        file.flush().await?;

        if let Some(expected) = expected_hash {
            let actual_hash = self.calculate_file_hash(path).await?;
            if actual_hash != expected {
                std::fs::remove_file(path).ok();
                return Err(Error::Other(format!(
                    "Hash mismatch: expected {}, got {}", expected, actual_hash
                )));
            }
        }

        Ok(())
    }

    async fn calculate_file_hash(&self, path: &Path) -> Result<String> {
        let contents = tokio::fs::read(path).await?;
        let mut hasher = Sha1::new();
        hasher.update(&contents);
        Ok(hex::encode(hasher.finalize()))
    }

    pub async fn download_with_retries(
        &self,
        url: &str,
        path: &Path,
        expected_hash: Option<&str>,
        max_retries: u32,
        progress_callback: Option<&ProgressCallback>,
    ) -> Result<()> {
        let mut last_error = None;
        
        for attempt in 0..=max_retries {
            let cloned_callback = progress_callback.map(|cb| {
                let cb_ptr = cb.as_ref() as *const (dyn Fn(u64, u64) + Send + Sync);
                unsafe { Box::from_raw(cb_ptr as *mut (dyn Fn(u64, u64) + Send + Sync)) }
            });
            
            match self.download_file(url, path, expected_hash, cloned_callback).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries {
                        tokio::time::sleep(Duration::from_millis(1000 * (attempt + 1) as u64)).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap())
    }

    pub fn get_cache(&self) -> &Cache {
        &self.cache
    }

    pub fn get_cache_mut(&mut self) -> &mut Cache {
        &mut self.cache
    }

    pub async fn download_with_progress_dialog(
        &self,
        url: &str,
        path: &Path,
        expected_hash: Option<&str>,
        filename: String,
    ) -> Result<bool> {
        if path.exists() {
            if let Some(hash) = expected_hash {
                let existing_hash = self.calculate_file_hash(path).await?;
                if existing_hash == hash {
                    return Ok(true);
                }
            }
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut progress_dialog = ProgressDialog::new(filename);
        
        terminal.draw(|f| {
            let area = f.size();
            progress_dialog.draw(f, area);
        })?;
        
        let response = self.client.get(url).send().await?;
        let total_size = response.content_length().unwrap_or(0);
        let mut file = tokio::fs::File::create(path).await?;
        let mut downloaded = 0u64;

        let bytes = response.bytes().await?;
        let mut pos = 0;
        let chunk_size = 8192;
        
        while pos < bytes.len() {
            let end = std::cmp::min(pos + chunk_size, bytes.len());
            let chunk = &bytes[pos..end];
            
            file.write_all(chunk).await?;
            
            downloaded += chunk.len() as u64;
            progress_dialog.update_progress(downloaded, total_size);
            
            if !progress_dialog.handle_input() {
                Self::cleanup_terminal(&mut terminal)?;
                if path.exists() {
                    std::fs::remove_file(path).ok();
                }
                return Ok(false);
            }
            
            if let Err(_) = terminal.draw(|f| {
                let area = f.size();
                progress_dialog.draw(f, area);
            }) {
                Self::cleanup_terminal(&mut terminal)?;
                if path.exists() {
                    std::fs::remove_file(path).ok();
                }
                return Ok(false);
            }
            
            pos = end;
        }
        
        file.flush().await?;

        if let Some(expected) = expected_hash {
            let actual_hash = self.calculate_file_hash(path).await?;
            if actual_hash != expected {
                Self::cleanup_terminal(&mut terminal)?;
                std::fs::remove_file(path).ok();
                return Err(Error::Other(format!(
                    "Hash mismatch: expected {}, got {}", expected, actual_hash
                )));
            }
        }

    
        progress_dialog.update_progress(total_size, total_size);
        terminal.draw(|f| {
            let area = f.size();
            progress_dialog.draw(f, area);
        })?;
        tokio::time::sleep(Duration::from_millis(1500)).await;

        Self::cleanup_terminal(&mut terminal)?;
        Ok(true)
    }

    pub async fn download_files_concurrent(
        &self,
        files: Vec<(String, PathBuf, Option<String>)>, // (url, path, expected_hash)
    ) -> Result<Vec<bool>> {
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(self.max_concurrent_downloads));
        let mut handles = Vec::new();

        for (url, path, expected_hash) in files {
            let permit = semaphore.clone();
            let network = self.clone();
            
            let handle = tokio::spawn(async move {
                let _permit = permit.acquire().await.unwrap();
                
                let filename = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file")
                    .to_string();

                network.download_with_progress_dialog(
                    &url,
                    &path,
                    expected_hash.as_deref(),
                    filename,
                ).await
            });
            
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result?),
                Err(e) => return Err(Error::Other(format!("Task join error: {}", e)).into()),
            }
        }

        Ok(results)
    }

    fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SimpleCacheEntry {
    pub data: Vec<u8>,
    pub timestamp: SystemTime,
    pub last_accessed: SystemTime,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct Cache {
    entries: HashMap<String, SimpleCacheEntry>,
    max_size: u64,
    current_size: u64,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            max_size: 100 * 1024 * 1024,
            current_size: 0,
        }
    }

    pub fn get(&mut self, key: &str) -> Option<&[u8]> {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.last_accessed = SystemTime::now();
            Some(&entry.data)
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: String, data: Vec<u8>) {
        let size = data.len() as u64;
        let now = SystemTime::now();
        
        if size > self.max_size {
            return;
        }

        self.evict_if_needed(size);

        let entry = SimpleCacheEntry {
            data,
            timestamp: now,
            last_accessed: now,
            size,
        };

        if let Some(old_entry) = self.entries.insert(key, entry) {
            self.current_size -= old_entry.size;
        }
        self.current_size += size;
    }

    fn evict_if_needed(&mut self, new_size: u64) {
        while self.current_size + new_size > self.max_size && !self.entries.is_empty() {
            let oldest_key = {
                let mut oldest_time = SystemTime::now();
                let mut oldest_key = String::new();
                
                for (key, entry) in &self.entries {
                    if entry.last_accessed < oldest_time {
                        oldest_time = entry.last_accessed;
                        oldest_key = key.clone();
                    }
                }
                oldest_key
            };
            
            if let Some(removed_entry) = self.entries.remove(&oldest_key) {
                self.current_size -= removed_entry.size;
            } else {
                break;
            }
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_size = 0;
    }

    pub fn size(&self) -> u64 {
        self.current_size
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl HttpCache {
    pub fn new(cache_dir: PathBuf, max_size: u64, max_age: Duration) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;
        
        let mut cache = Self {
            cache_dir,
            entries: HashMap::new(),
            max_cache_size: max_size,
            max_age,
        };
        
        cache.load_metadata()?;
        Ok(cache)
    }

    pub async fn get_cached_file(&self, url: &str, expected_hash: Option<&str>) -> Result<Option<PathBuf>> {
        let url_hash = Self::hash_url(url);
        
        if let Some(entry) = self.entries.get(&url_hash) {
            if let Some(expires) = entry.expires_at {
                let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
                if now > expires {
                    return Ok(None);
                }
            }
            
            if expected_hash.map_or(true, |h| h == entry.hash) {
                if entry.file_path.exists() {
                    return Ok(Some(entry.file_path.clone()));
                }
            }
        }
        
        Ok(None)
    }

    pub async fn store_file(&mut self, url: &str, file_path: &Path, hash: &str) -> Result<()> {
        let url_hash = Self::hash_url(url);
        let cached_path = self.cache_dir.join(&url_hash);
        
        tokio::fs::copy(file_path, &cached_path).await?;
        
        let metadata = tokio::fs::metadata(&cached_path).await?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        
        let entry = CacheEntry {
            url: url.to_string(),
            file_path: cached_path,
            hash: hash.to_string(),
            size: metadata.len(),
            created_at: now,
            expires_at: Some(now + self.max_age.as_secs()),
            etag: None,
            content_type: None,
        };
        
        self.entries.insert(url_hash, entry);
        self.save_metadata()?;
        self.cleanup_if_needed().await?;
        
        Ok(())
    }

    pub async fn clear(&mut self) -> Result<()> {
        for entry in self.entries.values() {
            if entry.file_path.exists() {
                tokio::fs::remove_file(&entry.file_path).await?;
            }
        }
        
        self.entries.clear();
        self.save_metadata()?;
        Ok(())
    }

    pub async fn cleanup_expired(&mut self) -> Result<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let mut to_remove = Vec::new();
        
        for (key, entry) in &self.entries {
            if let Some(expires) = entry.expires_at {
                if now > expires {
                    to_remove.push(key.clone());
                    if entry.file_path.exists() {
                        tokio::fs::remove_file(&entry.file_path).await?;
                    }
                }
            }
        }
        
        for key in to_remove {
            self.entries.remove(&key);
        }
        
        if !self.entries.is_empty() {
            self.save_metadata()?;
        }
        
        Ok(())
    }

    pub fn get_cache_size(&self) -> u64 {
        self.entries.values().map(|e| e.size).sum()
    }

    pub fn get_cache_info(&self) -> (usize, u64, u64) {
        let count = self.entries.len();
        let size = self.get_cache_size();
        let max_size = self.max_cache_size;
        (count, size, max_size)
    }

    async fn cleanup_if_needed(&mut self) -> Result<()> {
        let current_size = self.get_cache_size();
        
        if current_size > self.max_cache_size {
            let mut entries_to_remove: Vec<String> = Vec::new();
            let mut removed_size = 0u64;
            let target_remove = current_size - (self.max_cache_size * 8 / 10);
            
            let mut entries: Vec<_> = self.entries.iter().map(|(k, v)| (k.clone(), v.created_at, v.size)).collect();
            entries.sort_by_key(|(_, created_at, _)| *created_at);
            
            for (url_hash, _, size) in entries {
                if removed_size >= target_remove {
                    break;
                }
                entries_to_remove.push(url_hash);
                removed_size += size;
            }
            
            for url_hash in entries_to_remove {
                if let Some(removed_entry) = self.entries.remove(&url_hash) {
                    if removed_entry.file_path.exists() {
                        tokio::fs::remove_file(&removed_entry.file_path).await?;
                    }
                }
            }
            
            self.save_metadata()?;
        }
        
        Ok(())
    }

    fn hash_url(url: &str) -> String {
        let mut hasher = Sha1::new();
        hasher.update(url.as_bytes());
        hex::encode(hasher.finalize())
    }

    fn load_metadata(&mut self) -> Result<()> {
        let metadata_path = self.cache_dir.join("cache_metadata.json");
        
        if metadata_path.exists() {
            let content = std::fs::read_to_string(metadata_path)?;
            self.entries = serde_json::from_str(&content)?;
        }
        
        Ok(())
    }

    fn save_metadata(&self) -> Result<()> {
        let metadata_path = self.cache_dir.join("cache_metadata.json");
        let content = serde_json::to_string_pretty(&self.entries)?;
        std::fs::write(metadata_path, content)?;
        Ok(())
    }
} 