//! Interpretador e validador de manifest.json de extensões.
//!
//! Suporta Manifest V3 (prioritário) e V2 (compatibilidade limitada).

use crate::types::{
    BackgroundType, ContentScriptSpec, ExtensionIcon, InstallType,
};
use crate::errors::{ExtensionError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManifestVersion {
    V2,
    V3,
}

impl ManifestVersion {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            2 => Some(ManifestVersion::V2),
            3 => Some(ManifestVersion::V3),
            _ => None,
        }
    }

    pub fn as_u8(&self) -> u8 {
        match self {
            ManifestVersion::V2 => 2,
            ManifestVersion::V3 => 3,
        }
    }
}

/// Estrutura completa de um manifest.json de extensão.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    // Campos obrigatórios
    pub manifest_version: u8,
    pub name: String,
    pub version: String,

    // Campos opcionais comuns
    #[serde(default)]
    pub short_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub homepage_url: Option<String>,
    #[serde(default)]
    pub icons: HashMap<String, String>,

    // Background
    #[serde(default)]
    pub background: Option<BackgroundManifest>,

    // Content Scripts
    #[serde(default)]
    pub content_scripts: Vec<ContentScriptManifest>,

    // Ações
    #[serde(default)]
    pub action: Option<ActionManifest>,
    #[serde(default)]
    pub browser_action: Option<ActionManifest>,
    #[serde(default)]
    pub page_action: Option<ActionManifest>,

    // Permissões
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub optional_permissions: Vec<String>,
    #[serde(default)]
    pub host_permissions: Vec<String>,

    // Páginas
    #[serde(default)]
    pub options_page: Option<String>,
    #[serde(default)]
    pub options_ui: Option<OptionsUiManifest>,

    // Comandos
    #[serde(default)]
    pub commands: HashMap<String, CommandManifest>,

    // Menus de Contexto
    #[serde(default)]
    pub context_menus: Option<ContextMenusManifest>,

    // Declarative Net Request
    #[serde(default)]
    pub declarative_net_request: Option<DeclarativeNetRequestManifest>,

    // Web Accessible Resources
    #[serde(default)]
    pub web_accessible_resources: Vec<WebAccessibleResourceManifest>,

    // Minimum browser version
    #[serde(default)]
    pub minimum_chrome_version: Option<String>,

    // Key para verificação de atualizações
    #[serde(default)]
    pub key: Option<String>,

    // Storage schema
    #[serde(default)]
    pub storage: Option<StorageManifest>,

    // Side panel
    #[serde(default)]
    pub side_panel: Option<SidePanelManifest>,

    // Devtools
    #[serde(default)]
    pub devtools_page: Option<String>,

    // Campos não mapeados (para compatibilidade futura)
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundManifest {
    #[serde(default)]
    pub service_worker: Option<String>,
    #[serde(default)]
    pub page: Option<String>,
    #[serde(default)]
    pub scripts: Vec<String>,
    #[serde(default)]
    pub persistent: Option<bool>,
    #[serde(default)]
    pub r#type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentScriptManifest {
    pub matches: Vec<String>,
    #[serde(default)]
    pub exclude_matches: Vec<String>,
    #[serde(default)]
    pub css: Vec<String>,
    #[serde(default)]
    pub js: Vec<String>,
    #[serde(default)]
    pub run_at: Option<String>,
    #[serde(default)]
    pub all_frames: bool,
    #[serde(default)]
    pub match_about_blank: bool,
    #[serde(default)]
    pub world: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionManifest {
    #[serde(default)]
    pub default_title: Option<String>,
    #[serde(default)]
    pub default_popup: Option<String>,
    #[serde(default)]
    pub default_icon: Option<HashMap<String, String>>,
    #[serde(default)]
    pub default_area: Option<String>, // "navbar" or "menubar"
    #[serde(default)]
    pub pinned: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionsUiManifest {
    pub page: String,
    #[serde(default)]
    pub open_in_tab: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandManifest {
    #[serde(default)]
    pub suggested_key: Option<SuggestedKeyManifest>,
    pub description: Option<String>,
    #[serde(default)]
    pub global: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedKeyManifest {
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub chromeos: Option<String>,
    #[serde(default)]
    pub linux: Option<String>,
    #[serde(default)]
    pub mac: Option<String>,
    #[serde(default)]
    pub windows: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMenusManifest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclarativeNetRequestManifest {
    pub rule_resources: Vec<RuleResourceManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleResourceManifest {
    pub id: String,
    pub enabled: bool,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAccessibleResourceManifest {
    pub resources: Vec<String>,
    pub matches: Vec<String>,
    #[serde(default)]
    pub extension_ids: Vec<String>,
    #[serde(default)]
    pub use_dynamic_url: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageManifest {
    #[serde(default)]
    pub managed_schema: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidePanelManifest {
    #[serde(default)]
    pub default_path: Option<String>,
}

/// Resultado da validação de um manifesto.
#[derive(Debug, Clone)]
pub struct ManifestValidation {
    pub is_valid: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub manifest_version: ManifestVersion,
}

impl Manifest {
    /// Carrega e valida um manifest.json de um diretório de extensão.
    pub fn load_from_dir(dir: &Path) -> Result<(Self, PathBuf)> {
        let manifest_path = dir.join("manifest.json");
        if !manifest_path.exists() {
            return Err(ExtensionError::ManifestNotFound(
                manifest_path.to_string_lossy().to_string(),
            ));
        }

        let content = std::fs::read_to_string(&manifest_path)?;
        let manifest: Manifest = serde_json::from_str(&content)?;

        Ok((manifest, manifest_path))
    }

    /// Valida o manifesto contra as regras de suporte da BruNya.
    pub fn validate(&self) -> ManifestValidation {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        // Verificar versão do manifesto
        let mv = match ManifestVersion::from_u8(self.manifest_version) {
            Some(v) => {
                if v == ManifestVersion::V2 {
                    warnings.push("Manifest V2 é legado e terá suporte limitado. Considere migrar para V3.".to_string());
                }
                v
            }
            None => {
                errors.push(format!(
                    "Versão de manifest não suportada: {}. Suportamos V2 e V3.",
                    self.manifest_version
                ));
                return ManifestValidation {
                    is_valid: false,
                    warnings,
                    errors,
                    manifest_version: ManifestVersion::V3,
                };
            }
        };

        // Verificar campos obrigatórios
        if self.name.is_empty() {
            errors.push("Campo 'name' é obrigatório no manifest.json.".to_string());
        }
        if self.version.is_empty() {
            errors.push("Campo 'version' é obrigatório no manifest.json.".to_string());
        } else if !is_valid_version_string(&self.version) {
            warnings.push(format!("Formato de versão não padrão: '{}'. Esperado formato semver (x.y.z).", self.version));
        }

        // Verificar background
        if let Some(ref bg) = self.background {
            if mv == ManifestVersion::V3 {
                if bg.service_worker.is_none() && bg.page.is_some() {
                    warnings.push("Manifest V3 recomenda o uso de 'service_worker' em vez de 'page' para background.".to_string());
                }
                if bg.persistent == Some(true) {
                    errors.push("Manifest V3 não permite background pages persistentes. Use service_worker.".to_string());
                }
            }
        }

        // Verificar action vs browser_action
        if mv == ManifestVersion::V3 && self.browser_action.is_some() {
            warnings.push("Manifest V3: 'browser_action' foi renomeado para 'action'.".to_string());
        }
        if mv == ManifestVersion::V3 && self.page_action.is_some() {
            warnings.push("Manifest V3: 'page_action' foi mesclado em 'action'.".to_string());
        }

        // Verificar host_permissions no V3
        if mv == ManifestVersion::V3 && self.host_permissions.is_empty() && self.permissions.contains(&"<all_urls>".to_string()) {
            warnings.push("Manifest V3: '<all_urls>' em permissions deve ser movido para 'host_permissions'.".to_string());
        }

        // Verificar content scripts
        for (i, cs) in self.content_scripts.iter().enumerate() {
            if cs.matches.is_empty() {
                warnings.push(format!("Content script #{} não possui 'matches'. Ele nunca será executado.", i + 1));
            }
        }

        let is_valid = errors.is_empty();

        ManifestValidation {
            is_valid,
            warnings,
            errors,
            manifest_version: mv,
        }
    }

    /// Converte o Manifest para o formato interno ExtensionInfo.
    pub fn to_extension_info(
        &self,
        ext_id: crate::ExtensionId,
        dir: &Path,
        install_type: InstallType,
        state: crate::ExtensionState,
    ) -> crate::ExtensionInfo {
        let icons: Vec<ExtensionIcon> = self
            .icons
            .iter()
            .map(|(size_str, path)| {
                let size = size_str.parse::<u32>().unwrap_or(48);
                ExtensionIcon { size, path: path.clone() }
            })
            .collect();

        let action_popup = self
            .action
            .as_ref()
            .or(self.browser_action.as_ref())
            .and_then(|a| a.default_popup.clone());

        let options_page = self
            .options_ui
            .as_ref()
            .map(|o| o.page.clone())
            .or_else(|| self.options_page.clone());

        let background_type = self.background.as_ref().map(|bg| {
            if bg.service_worker.is_some() {
                BackgroundType::ServiceWorker
            } else {
                BackgroundType::BackgroundPage
            }
        });

        let content_scripts: Vec<ContentScriptSpec> = self
            .content_scripts
            .iter()
            .map(|cs| ContentScriptSpec {
                matches: cs.matches.clone(),
                exclude_matches: cs.exclude_matches.clone(),
                css: cs.css.clone(),
                js: cs.js.clone(),
                run_at: cs.run_at.clone().unwrap_or_else(|| "document_idle".to_string()),
                all_frames: cs.all_frames,
                match_about_blank: cs.match_about_blank,
                world: cs.world.clone().unwrap_or_else(|| "ISOLATED".to_string()),
            })
            .collect();

        let now = chrono::Utc::now();

        crate::ExtensionInfo {
            id: ext_id,
            name: self.name.clone(),
            short_name: self.short_name.clone(),
            description: self.description.clone(),
            version: self.version.clone(),
            author: self.author.clone(),
            homepage_url: self.homepage_url.clone(),
            icons,
            state,
            install_type,
            manifest_version: self.manifest_version,
            permissions: self.permissions.clone(),
            optional_permissions: self.optional_permissions.clone(),
            host_permissions: self.host_permissions.clone(),
            installed_at: now,
            updated_at: now,
            directory: dir.to_path_buf(),
            has_options_page: options_page.is_some(),
            has_popup: action_popup.is_some(),
            popup_path: action_popup,
            options_page,
            background_type,
            content_scripts,
        }
    }

    /// Busca a origem do service worker da extensão.
    pub fn service_worker_path(&self) -> Option<String> {
        self.background
            .as_ref()
            .and_then(|bg| bg.service_worker.as_ref().cloned())
    }

    /// Retorna a versão do manifesto.
    pub fn manifest_version(&self) -> ManifestVersion {
        ManifestVersion::from_u8(self.manifest_version).unwrap_or(ManifestVersion::V3)
    }
}

fn is_valid_version_string(v: &str) -> bool {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() < 2 || parts.len() > 4 {
        return false;
    }
    parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_manifest_v3() {
        let json = r#"{
            "manifest_version": 3,
            "name": "Test Extension",
            "version": "1.0.0",
            "description": "A test extension",
            "permissions": ["storage", "tabs"],
            "host_permissions": ["*://*.example.com/*"],
            "background": {
                "service_worker": "background.js"
            },
            "action": {
                "default_popup": "popup.html",
                "default_title": "Test Ext"
            },
            "content_scripts": [{
                "matches": ["*://*.example.com/*"],
                "js": ["content.js"],
                "run_at": "document_idle"
            }]
        }"#;

        let manifest: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.manifest_version, 3);
        assert_eq!(manifest.name, "Test Extension");
        assert_eq!(manifest.version, "1.0.0");
        assert!(manifest.background.is_some());
        assert!(manifest.action.is_some());
        assert_eq!(manifest.content_scripts.len(), 1);
        assert_eq!(manifest.permissions.len(), 2);
        assert_eq!(manifest.host_permissions.len(), 1);

        let validation = manifest.validate();
        assert!(validation.is_valid);
        assert!(validation.warnings.is_empty());
    }

    #[test]
    fn test_invalid_manifest_missing_fields() {
        let json = r#"{
            "manifest_version": 3,
            "name": "",
            "version": ""
        }"#;

        let manifest: Manifest = serde_json::from_str(json).unwrap();
        let validation = manifest.validate();
        assert!(!validation.is_valid);
    }

    #[test]
    fn test_manifest_v2_warning() {
        let json = r#"{
            "manifest_version": 2,
            "name": "Legacy Extension",
            "version": "1.0",
            "background": {
                "scripts": ["background.js"],
                "persistent": true
            }
        }"#;

        let manifest: Manifest = serde_json::from_str(json).unwrap();
        let validation = manifest.validate();
        assert!(validation.is_valid);
        assert!(!validation.warnings.is_empty());
        assert!(validation.warnings.iter().any(|w| w.contains("V2")));
    }

    #[test]
    fn test_version_validation() {
        assert!(is_valid_version_string("1.0.0"));
        assert!(is_valid_version_string("1.0"));
        assert!(is_valid_version_string("1.0.0.0"));
        assert!(!is_valid_version_string("1.0.0-beta"));
        assert!(!is_valid_version_string("abc"));
    }
}
