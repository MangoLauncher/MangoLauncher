pub mod error;
pub mod utils;
pub mod platform;
pub mod settings;
pub mod java;
pub mod network;
pub mod assets;
pub mod auth;
pub mod instance;
pub mod profile;
pub mod launch;
pub mod mods;
pub mod version;
pub mod progress;
pub mod logs;
pub mod app;
pub mod ui;

pub use error::{Error, Result};
use crate::app::App;

pub const VERSION: &str = "2.0.0";

pub async fn run() -> Result<()> {
    let mut app = App::new().await?;
    app.init().await?;
    ui::run_ui(app).await
} 