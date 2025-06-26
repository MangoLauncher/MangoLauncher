use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use chrono::{DateTime, Local, Duration};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Debug,
    Launcher,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Debug => "DEBUG",
            LogLevel::Launcher => "LAUNCHER",
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        match self {
            LogLevel::Info => ratatui::style::Color::White,
            LogLevel::Warning => ratatui::style::Color::Yellow,
            LogLevel::Error => ratatui::style::Color::Red,
            LogLevel::Debug => ratatui::style::Color::Gray,
            LogLevel::Launcher => ratatui::style::Color::Cyan,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Local>,
    pub level: LogLevel,
    pub message: String,
    pub source: Option<String>,
}

impl LogEntry {
    pub fn new(level: LogLevel, message: String, source: Option<String>) -> Self {
        Self {
            timestamp: Local::now(),
            level,
            message,
            source,
        }
    }

    pub fn format(&self) -> String {
        let time_str = self.timestamp.format("%H:%M:%S").to_string();
        let source_str = self.source.as_ref()
            .map(|s| format!("[{}]", s))
            .unwrap_or_default();
        
        format!("{} {} {} {}", 
            time_str, 
            self.level.as_str(), 
            source_str, 
            self.message
        )
    }
}

#[derive(Debug, Clone)]
pub struct LogManager {
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
    max_entries: usize,
    log_dir: Arc<Mutex<Option<PathBuf>>>,
    current_log_file: Arc<Mutex<Option<(PathBuf, File)>>>,
    file_logging_enabled: Arc<AtomicBool>,
}

impl LogManager {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(max_entries))),
            max_entries,
            log_dir: Arc::new(Mutex::new(None)),
            current_log_file: Arc::new(Mutex::new(None)),
            file_logging_enabled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_file_logging(max_entries: usize, log_dir: PathBuf, enabled: bool) -> Self {
        let manager = Self::new(max_entries);
        if let Ok(mut dir) = manager.log_dir.lock() {
            *dir = Some(log_dir);
        }
        manager.file_logging_enabled.store(enabled, Ordering::Relaxed);
        if enabled {
            manager.ensure_log_file();
            manager.cleanup_old_logs();
        }
        manager
    }

    pub fn set_file_logging(&self, enabled: bool, log_dir: Option<PathBuf>) {
        self.file_logging_enabled.store(enabled, Ordering::Relaxed);
        if let Some(dir) = log_dir {
            if let Ok(mut current_dir) = self.log_dir.lock() {
                *current_dir = Some(dir);
            }
        }
        
        if enabled {
            self.ensure_log_file();
            self.cleanup_old_logs();
        } else {
            if let Ok(mut file) = self.current_log_file.lock() {
                *file = None;
            }
        }
    }

    fn ensure_log_file(&self) {
        if !self.file_logging_enabled.load(Ordering::Relaxed) {
            return;
        }

        let log_dir = if let Ok(dir) = self.log_dir.lock() {
            if let Some(ref d) = *dir {
                d.clone()
            } else {
                return;
            }
        } else {
            return;
        };
        
        if let Err(_) = fs::create_dir_all(&log_dir) {
            return;
        }

        let now = Local::now();
        let log_filename = format!("mango-launcher-{}.log", now.format("%Y-%m-%d_%H-%M-%S"));
        let log_path = log_dir.join(log_filename);

        if let Ok(file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path) 
        {
            if let Ok(mut current_file) = self.current_log_file.lock() {
                *current_file = Some((log_path, file));
            }
        }
    }

    fn cleanup_old_logs(&self) {
        let log_dir = if let Ok(dir) = self.log_dir.lock() {
            if let Some(ref d) = *dir {
                d.clone()
            } else {
                return;
            }
        } else {
            return;
        };
        
        let cutoff_time = Local::now() - Duration::hours(24);
        
        if let Ok(entries) = fs::read_dir(&log_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && 
                   path.extension().and_then(|s| s.to_str()) == Some("log") &&
                   path.file_stem().and_then(|s| s.to_str()).unwrap_or("").starts_with("mango-launcher-") {
                    
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            let modified_datetime: DateTime<Local> = modified.into();
                            if modified_datetime < cutoff_time {
                                let _ = fs::remove_file(path);
                            }
                        }
                    }
                }
            }
        }
    }

    fn write_to_file(&self, entry: &LogEntry) {
        if !self.file_logging_enabled.load(Ordering::Relaxed) {
            return;
        }

        if let Ok(mut current_file) = self.current_log_file.lock() {
            if current_file.is_none() {
                drop(current_file);
                self.ensure_log_file();
                current_file = self.current_log_file.lock().unwrap();
            }

            if let Some((_, ref mut file)) = *current_file {
                let formatted = format!("{}\n", entry.format());
                let _ = file.write_all(formatted.as_bytes());
                let _ = file.flush();
            }
        }
    }

    pub fn log(&self, level: LogLevel, message: String, source: Option<String>) {
        let entry = LogEntry::new(level, message, source);
        
        self.write_to_file(&entry);
        
        if let Ok(mut entries) = self.entries.lock() {
            entries.push_back(entry);
            
            if entries.len() > self.max_entries {
                entries.pop_front();
            }
        }
    }

    pub fn info(&self, message: String, source: Option<String>) {
        self.log(LogLevel::Info, message, source);
    }

    pub fn warning(&self, message: String, source: Option<String>) {
        self.log(LogLevel::Warning, message, source);
    }

    pub fn error(&self, message: String, source: Option<String>) {
        self.log(LogLevel::Error, message, source);
    }

    pub fn debug(&self, message: String, source: Option<String>) {
        self.log(LogLevel::Debug, message, source);
    }

    pub fn launcher(&self, message: String, source: Option<String>) {
        self.log(LogLevel::Launcher, message, source);
    }

    pub fn get_entries(&self) -> Vec<LogEntry> {
        if let Ok(entries) = self.entries.lock() {
            entries.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_recent_entries(&self, count: usize) -> Vec<LogEntry> {
        if let Ok(entries) = self.entries.lock() {
            entries.iter()
                .rev()
                .take(count)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
    }

    pub fn count(&self) -> usize {
        if let Ok(entries) = self.entries.lock() {
            entries.len()
        } else {
            0
        }
    }

    pub fn get_entries_by_level(&self, level: LogLevel) -> Vec<LogEntry> {
        if let Ok(entries) = self.entries.lock() {
            entries.iter()
                .filter(|entry| entry.level == level)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn search(&self, query: &str) -> Vec<LogEntry> {
        let query_lower = query.to_lowercase();
        
        if let Ok(entries) = self.entries.lock() {
            entries.iter()
                .filter(|entry| {
                    entry.message.to_lowercase().contains(&query_lower) ||
                    entry.source.as_ref().map_or(false, |s| s.to_lowercase().contains(&query_lower))
                })
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl Default for LogManager {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl LogLevel {
    pub fn from_minecraft_level(level: &str) -> Self {
        match level.to_uppercase().as_str() {
            "INFO" => LogLevel::Info,
            "WARN" | "WARNING" => LogLevel::Warning, 
            "ERROR" | "FATAL" => LogLevel::Error,
            "DEBUG" | "TRACE" => LogLevel::Debug,
            _ => LogLevel::Info,
        }
    }
} 