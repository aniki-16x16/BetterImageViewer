use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub window_pos: Option<[f32; 2]>,
    pub window_size: Option<[f32; 2]>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window_pos: None,
            window_size: Some([800.0, 600.0]),
        }
    }
}

impl AppConfig {
    fn get_config_path() -> Option<PathBuf> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "aniki", "better_image_viewer") {
            let config_dir = proj_dirs.config_dir();
            if !config_dir.exists() {
                let _ = fs::create_dir_all(config_dir);
            }
            Some(config_dir.join("config.json"))
        } else {
            None
        }
    }

    pub fn load() -> Self {
        if let Some(path) = Self::get_config_path() {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(config) = serde_json::from_str(&content) {
                        return config;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::get_config_path() {
            if let Ok(content) = serde_json::to_string_pretty(self) {
                let _ = fs::write(path, content);
            }
        }
    }
}
