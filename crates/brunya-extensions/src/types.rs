//! Tipos fundamentais do sistema de extensões.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// Identificador único para cada extensão instalada.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExtensionId(pub String);

impl ExtensionId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    /// Gera um ID a partir de uma chave pública ou hash do diretório.
    pub fn from_path(path: &str) -> Self {
        let hash = ring::digest::digest(&ring::digest::SHA256, path.as_bytes());
        let hex = hex_encode(hash.as_ref());
        Self(hex[..32].to_string())
    }
}

impl fmt::Display for ExtensionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Estado atual de uma extensão instalada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtensionState {
    /// Extensão carregada e ativa.
    Active,
    /// Extensão desativada pelo usuário.
    Disabled,
    /// Extensão carregada de diretório local (modo desenvolvedor).
    DeveloperMode,
    /// Extensão com erro de carregamento.
    Error,
    /// Extensão em processo de instalação ou atualização.
    Installing,
    /// Extensão incompatível com a versão atual do motor.
    Incompatible,
}

impl fmt::Display for ExtensionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtensionState::Active => write!(f, "Ativa"),
            ExtensionState::Disabled => write!(f, "Desativada"),
            ExtensionState::DeveloperMode => write!(f, "Modo Desenvolvedor"),
            ExtensionState::Error => write!(f, "Erro"),
            ExtensionState::Installing => write!(f, "Instalando"),
            ExtensionState::Incompatible => write!(f, "Incompatível"),
        }
    }
}

/// Informações resumidas de uma extensão para exibição na interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionInfo {
    pub id: ExtensionId,
    pub name: String,
    pub short_name: Option<String>,
    pub description: Option<String>,
    pub version: String,
    pub author: Option<String>,
    pub homepage_url: Option<String>,
    pub icons: Vec<ExtensionIcon>,
    pub state: ExtensionState,
    pub install_type: InstallType,
    pub manifest_version: u8,
    pub permissions: Vec<String>,
    pub optional_permissions: Vec<String>,
    pub host_permissions: Vec<String>,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub directory: std::path::PathBuf,
    pub has_options_page: bool,
    pub has_popup: bool,
    pub popup_path: Option<String>,
    pub options_page: Option<String>,
    pub background_type: Option<BackgroundType>,
    pub content_scripts: Vec<ContentScriptSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallType {
    /// Instalado via pacote CRX ou arquivo local.
    Normal,
    /// Carregado como diretório de desenvolvimento.
    Unpacked,
    /// Instalado via loja/catálogo.
    FromStore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackgroundType {
    /// Manifest V3: Service Worker
    ServiceWorker,
    /// Manifest V2: Persistent Background Page (legado)
    BackgroundPage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionIcon {
    pub size: u32,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentScriptSpec {
    pub matches: Vec<String>,
    pub exclude_matches: Vec<String>,
    pub css: Vec<String>,
    pub js: Vec<String>,
    pub run_at: String, // "document_start", "document_end", "document_idle"
    pub all_frames: bool,
    pub match_about_blank: bool,
    pub world: String, // "ISOLATED" or "MAIN"
}

/// Contador global para IDs de sessão de runtime.
static NEXT_RUNTIME_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimeSessionId(pub u64);

impl RuntimeSessionId {
    pub fn new() -> Self {
        Self(NEXT_RUNTIME_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for RuntimeSessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RuntimeSessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RS-{}", self.0)
    }
}

/// Mensagem de runtime entre contextos de extensão.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMessage {
    pub sender: MessageSender,
    pub topic: String,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSender {
    pub extension_id: ExtensionId,
    pub tab_id: Option<u64>,
    pub frame_id: Option<u64>,
    pub url: Option<String>,
    pub origin: Option<String>,
}

/// Evento que pode ativar um Service Worker de extensão.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtensionEvent {
    RuntimeOnInstalled { reason: String, previous_version: Option<String> },
    RuntimeOnStartup,
    RuntimeOnSuspend,
    RuntimeOnMessage { message: ExtensionMessage },
    TabsOnCreated { tab_id: u64, url: Option<String> },
    TabsOnUpdated { tab_id: u64, url: Option<String>, status: String },
    TabsOnActivated { tab_id: u64 },
    TabsOnRemoved { tab_id: u64 },
    WebNavigationOnCompleted { tab_id: u64, url: String },
    WebNavigationOnBeforeNavigate { tab_id: u64, url: String },
    AlarmsOnAlarm { name: String },
    StorageOnChanged { keys: Vec<String>, area: String },
    ActionOnClicked { tab_id: u64 },
    CommandsOnCommand { command: String, tab_id: Option<u64> },
    ContextMenusOnClicked { menu_item_id: String, tab_id: u64 },
    PermissionsOnAdded { permissions: Vec<String> },
    PermissionsOnRemoved { permissions: Vec<String> },
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}