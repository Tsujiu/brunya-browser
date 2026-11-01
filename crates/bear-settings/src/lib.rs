//! Configurações de usuário e preferências de sistema do BEAR Browser.

use bear_adblock::ProtectionMode;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum SettingsError {
    #[error("Erro de I/O em configurações: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erro de serialização JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, SettingsError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEngine {
    pub name: String,
    pub query_template: String,
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self {
            name: "DuckDuckGo".to_string(),
            query_template: "https://duckduckgo.com/?q=%s".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    pub language: String,
    pub theme: ThemePreference,
    pub home_url: String,
    pub new_tab_url: String,
    pub search_engine: SearchEngine,
    pub tab_suspension_enabled: bool,
    pub tab_suspension_timeout_minutes: u32,
    pub adblock_mode: ProtectionMode,
    pub block_third_party_cookies: bool,
    pub hardware_acceleration: bool,
    pub do_not_track: bool,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            language: "system".to_string(),
            theme: ThemePreference::System,
            home_url: "bear://newtab".to_string(),
            new_tab_url: "bear://newtab".to_string(),
            search_engine: SearchEngine::default(),
            tab_suspension_enabled: true,
            tab_suspension_timeout_minutes: 15,
            adblock_mode: ProtectionMode::Balanced,
            block_third_party_cookies: true,
            hardware_acceleration: true,
            do_not_track: true,
        }
    }
}

pub struct SettingsManager {
    file_path: Option<PathBuf>,
    settings: RwLock<UserSettings>,
}

impl SettingsManager {
    pub fn new_in_memory() -> Self {
        Self {
            file_path: None,
            settings: RwLock::new(UserSettings::default()),
        }
    }

    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        let p = path.as_ref().to_path_buf();
        let loaded = if p.exists() {
            match fs::read_to_string(&p) {
                Ok(content) => match serde_json::from_str::<UserSettings>(&content) {
                    Ok(s) => {
                        info!("Configurações do BEAR carregadas de {}", p.display());
                        s
                    }
                    Err(e) => {
                        warn!("Falha ao decodificar configurações (usando padrão): {}", e);
                        UserSettings::default()
                    }
                },
                Err(e) => {
                    warn!("Falha ao ler arquivo de configurações: {}", e);
                    UserSettings::default()
                }
            }
        } else {
            UserSettings::default()
        };

        Self {
            file_path: Some(p),
            settings: RwLock::new(loaded),
        }
    }

    pub fn get(&self) -> UserSettings {
        self.settings.read().clone()
    }

    pub fn update<F: FnOnce(&mut UserSettings)>(&self, updater: F) -> Result<()> {
        {
            let mut s = self.settings.write();
            updater(&mut s);
        }
        self.save()
    }

    pub fn save(&self) -> Result<()> {
        if let Some(ref path) = self.file_path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let data = serde_json::to_string_pretty(&*self.settings.read())?;
            fs::write(path, data)?;
        }
        Ok(())
    }
}

impl Default for SettingsManager {
    fn default() -> Self {
        Self::new_in_memory()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_settings_persistence() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_path_buf();

        let mgr = SettingsManager::load_or_default(&path);
        assert_eq!(mgr.get().theme, ThemePreference::System);

        mgr.update(|s| {
            s.theme = ThemePreference::Dark;
            s.tab_suspension_timeout_minutes = 20;
        })
        .unwrap();

        // Recarregar de arquivo
        let mgr2 = SettingsManager::load_or_default(&path);
        assert_eq!(mgr2.get().theme, ThemePreference::Dark);
        assert_eq!(mgr2.get().tab_suspension_timeout_minutes, 20);
    }
}
