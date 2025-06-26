 
use std::time::{Duration, Instant};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Style},
    widgets::{Block, Borders, Gauge, Paragraph, Clear},
    Frame,
};
use crossterm::event::{self, Event, KeyCode};
use crate::utils;

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub speed_bps: f64,
    pub eta_seconds: f64,
    pub filename: String,
    pub status: String,
}

impl DownloadProgress {
    pub fn new(filename: String) -> Self {
        Self {
            downloaded: 0,
            total: 0,
            speed_bps: 0.0,
            eta_seconds: 0.0,
            filename,
            status: "Подготовка...".to_string(),
        }
    }

    pub fn update(&mut self, downloaded: u64, total: u64) {
        self.downloaded = downloaded;
        self.total = total;
    }

    pub fn calculate_speed(&mut self, elapsed: Duration, bytes_since_last: u64) {
        if elapsed.as_millis() > 0 {
            self.speed_bps = (bytes_since_last as f64) / elapsed.as_secs_f64();
            
            if self.total > self.downloaded && self.speed_bps > 0.0 {
                self.eta_seconds = (self.total - self.downloaded) as f64 / self.speed_bps;
            } else {
                self.eta_seconds = 0.0;
            }
        }
    }

    pub fn get_progress_percentage(&self) -> u16 {
        if self.total == 0 {
            0
        } else {
            ((self.downloaded as f64 / self.total as f64) * 100.0) as u16
        }
    }

    pub fn format_progress(&self) -> String {
        format!(
            "{} / {}",
            utils::format_size(self.downloaded),
            utils::format_size(self.total)
        )
    }

    pub fn format_speed(&self) -> String {
        if self.speed_bps > 0.0 {
            let eta_str = if self.eta_seconds > 0.0 {
                format_duration(self.eta_seconds)
            } else {
                "неизвестно".to_string()
            };
            format!(
                "{}/s (осталось: {})",
                utils::format_size(self.speed_bps as u64),
                eta_str
            )
        } else {
            "0 B/s".to_string()
        }
    }
}

pub struct ProgressDialog {
    progress: DownloadProgress,
    last_update: Instant,
    last_bytes: u64,
    cancelled: bool,
}

impl ProgressDialog {
    pub fn new(filename: String) -> Self {
        Self {
            progress: DownloadProgress::new(filename),
            last_update: Instant::now(),
            last_bytes: 0,
            cancelled: false,
        }
    }

    pub fn update_progress(&mut self, downloaded: u64, total: u64) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);
        let bytes_since_last = downloaded.saturating_sub(self.last_bytes);

        self.progress.update(downloaded, total);
        self.progress.calculate_speed(elapsed, bytes_since_last);

        if downloaded == total && total > 0 {
            self.progress.status = "Завершено!".to_string();
        } else if total > 0 {
            self.progress.status = "Загружается...".to_string();
        }

        self.last_update = now;
        self.last_bytes = downloaded;
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    pub fn draw(&self, f: &mut Frame, area: Rect) {
        f.render_widget(Clear, area);
        let popup_area = centered_rect(60, 30, area);

        let main_block = Block::default()
            .title("Загрузка файла")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        f.render_widget(main_block, popup_area);

        let inner_area = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(2), 
                Constraint::Length(1), 
                Constraint::Length(1), 
                Constraint::Length(3), 
                Constraint::Length(1), 
                Constraint::Length(1), 
                Constraint::Length(1), 
                Constraint::Length(1), 
                Constraint::Length(1), 
            ])
            .split(popup_area);

        let filename = Paragraph::new(format!("Файл: {}", self.progress.filename))
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center);
        f.render_widget(filename, inner_area[0]);

        let progress_text = Paragraph::new(self.progress.format_progress())
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
        f.render_widget(progress_text, inner_area[2]);

        let progress_bar = Gauge::default()
            .block(Block::default().borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Green))
            .percent(self.progress.get_progress_percentage())
            .label(format!("{}%", self.progress.get_progress_percentage()));
        f.render_widget(progress_bar, inner_area[3]);

        let speed_text = Paragraph::new(self.progress.format_speed())
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center);
        f.render_widget(speed_text, inner_area[4]);

        let status = Paragraph::new(self.progress.status.clone())
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center);
        f.render_widget(status, inner_area[6]);

        let controls = Paragraph::new("Esc: Отмена")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        f.render_widget(controls, inner_area[8]);
    }

    pub fn handle_input(&mut self) -> bool {
        if event::poll(Duration::from_millis(10)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                match key.code {
                    KeyCode::Esc => {
                        self.cancelled = true;
                        return false;
                    }
                    _ => {}
                }
            }
        }
        true
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn format_duration(seconds: f64) -> String {
    if seconds < 0.0 || seconds.is_infinite() || seconds.is_nan() {
        return "неизвестно".to_string();
    }

    let total_seconds = seconds as u64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;

    if hours > 0 {
        format!("{}ч {}м {}с", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}м {}с", minutes, secs)
    } else {
        format!("{}с", secs)
    }
} 