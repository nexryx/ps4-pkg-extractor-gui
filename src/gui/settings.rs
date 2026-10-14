use crate::gui::i18n::Locale;
use directories::ProjectDirs;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_THEME: &str = "Catppuccin.json";

// Settings written before every theme was a palette file held "dark", "light"
// or {"file": "<name>"}: the first two mean the default, the last that file.
fn read_theme<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Ok(match Value::deserialize(deserializer)? {
        Value::String(name) if name.ends_with(".json") => name,
        Value::Object(map) => map
            .get("file")
            .and_then(Value::as_str)
            .map_or_else(|| DEFAULT_THEME.to_owned(), str::to_owned),
        _ => DEFAULT_THEME.to_owned(),
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    #[serde(deserialize_with = "read_theme")]
    pub theme: String,
    pub language: Option<Locale>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: DEFAULT_THEME.to_owned(),
            language: None,
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
pub struct AppDirs {
    pub config: PathBuf,
    pub state: PathBuf,
}

impl AppDirs {
    pub fn discover() -> Self {
        match ProjectDirs::from("dev", "afkarxyz", "pkgextract") {
            Some(dirs) => Self {
                config: dirs.config_dir().to_path_buf(),
                state: dirs.data_local_dir().to_path_buf(),
            },
            None => {
                let fallback = std::env::temp_dir().join("pkgextract");
                Self {
                    config: fallback.clone(),
                    state: fallback,
                }
            }
        }
    }

    pub fn settings_file(&self) -> PathBuf {
        self.config.join("settings.json")
    }

    pub fn themes_dir(&self) -> PathBuf {
        self.config.join("themes")
    }

    pub fn log_file(&self) -> PathBuf {
        self.state.join("pkgextract.log")
    }

    pub fn panic_log(&self) -> PathBuf {
        self.state.join("panic.log")
    }
}
