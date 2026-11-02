//! Gerenciamento de permissões para extensões.
//!
//! Controla quais permissões cada extensão possui, gerencia permissões
//! opcionais e mantém registro das aprovações do usuário.

use crate::errors::Result;
use crate::types::ExtensionId;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// Nível de uma permissão individual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionLevel {
    /// Permissão que não requer aviso ao usuário.
    Normal,
    /// Permissão que requer aviso antes de conceder.
    Warning,
    /// Permissão que requer confirmação explícita.
    Sensitive,
}

/// Categorias de permissões suportadas.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    // APIs essenciais
    Storage,
    Tabs,
    Scripting,
    ActiveTab,
    Alarms,
    Notifications,
    ContextMenus,
    Commands,
    Cookies,
    WebRequest,
    WebNavigation,
    Downloads,
    Bookmarks,
    History,
    Favicon,

    // Permissões de host (URLs)
    HostPermission(String),

    // Permissões especiais
    AllUrls,
    NativeMessaging,
    ClipboardRead,
    ClipboardWrite,
    Privacy,
    TabCapture,
    DesktopCapture,
    Management,
    SystemCpu,
    SystemMemory,
    SystemDisplay,
    // ... outras podem ser adicionadas dinamicamente
    Other(String),
}

impl Permission {
    /// Cria uma permissão a partir de uma string (como aparece no manifest.json).
    pub fn from_str(s: &str) -> Self {
        match s {
            "storage" => Permission::Storage,
            "tabs" => Permission::Tabs,
            "scripting" => Permission::Scripting,
            "activeTab" => Permission::ActiveTab,
            "alarms" => Permission::Alarms,
            "notifications" => Permission::Notifications,
            "contextMenus" => Permission::ContextMenus,
            "commands" => Permission::Commands,
            "cookies" => Permission::Cookies,
            "webRequest" => Permission::WebRequest,
            "webNavigation" => Permission::WebNavigation,
            "downloads" => Permission::Downloads,
            "bookmarks" => Permission::Bookmarks,
            "history" => Permission::History,
            "favicon" => Permission::Favicon,
            "<all_urls>" => Permission::AllUrls,
            "nativeMessaging" => Permission::NativeMessaging,
            "clipboardRead" => Permission::ClipboardRead,
            "clipboardWrite" => Permission::ClipboardWrite,
            "privacy" => Permission::Privacy,
            "tabCapture" => Permission::TabCapture,
            "desktopCapture" => Permission::DesktopCapture,
            "management" => Permission::Management,
            "system.cpu" => Permission::SystemCpu,
            "system.memory" => Permission::SystemMemory,
            "system.display" => Permission::SystemDisplay,
            _ if s.starts_with("https://") || s.starts_with("http://") || s.starts_with("*://") || s.contains("://") => {
                Permission::HostPermission(s.to_string())
            }
            _ => Permission::Other(s.to_string()),
        }
    }

    /// Retorna o nível de sensibilidade da permissão.
    pub fn level(&self) -> PermissionLevel {
        match self {
            Permission::Storage
            | Permission::Alarms
            | Permission::Commands
            | Permission::Favicon => PermissionLevel::Normal,

            Permission::Tabs
            | Permission::ActiveTab
            | Permission::Scripting
            | Permission::Notifications
            | Permission::ContextMenus
            | Permission::Downloads
            | Permission::Bookmarks
            | Permission::History => PermissionLevel::Warning,

            Permission::Cookies
            | Permission::WebRequest
            | Permission::WebNavigation
            | Permission::AllUrls
            | Permission::NativeMessaging
            | Permission::ClipboardRead
            | Permission::ClipboardWrite
            | Permission::Privacy
            | Permission::TabCapture
            | Permission::DesktopCapture
            | Permission::Management
            | Permission::SystemCpu
            | Permission::SystemMemory
            | Permission::SystemDisplay => PermissionLevel::Sensitive,

            Permission::HostPermission(_) => PermissionLevel::Warning,

            Permission::Other(_) => PermissionLevel::Sensitive,
        }
    }

    /// Descrição amigável da permissão.
    pub fn description(&self) -> String {
        match self {
            Permission::Storage => "Acessar dados de armazenamento local".to_string(),
            Permission::Tabs => "Acessar e modificar abas".to_string(),
            Permission::Scripting => "Injetar scripts em páginas".to_string(),
            Permission::ActiveTab => "Acessar a aba ativa temporariamente".to_string(),
            Permission::Alarms => "Agendar tarefas periódicas".to_string(),
            Permission::Notifications => "Exibir notificações".to_string(),
            Permission::ContextMenus => "Adicionar itens ao menu de contexto".to_string(),
            Permission::Commands => "Definir atalhos de teclado".to_string(),
            Permission::Cookies => "Acessar e modificar cookies".to_string(),
            Permission::WebRequest => "Observar e modificar requisições de rede".to_string(),
            Permission::WebNavigation => "Acompanhar navegação em páginas".to_string(),
            Permission::Downloads => "Gerenciar downloads".to_string(),
            Permission::Bookmarks => "Acessar e modificar favoritos".to_string(),
            Permission::History => "Acessar e modificar histórico".to_string(),
            Permission::Favicon => "Acessar favicons de sites".to_string(),
            Permission::AllUrls => "Acessar dados em todos os sites".to_string(),
            Permission::NativeMessaging => "Comunicar-se com aplicativos nativos".to_string(),
            Permission::ClipboardRead => "Ler conteúdo da área de transferência".to_string(),
            Permission::ClipboardWrite => "Modificar a área de transferência".to_string(),
            Permission::Privacy => "Acessar configurações de privacidade".to_string(),
            Permission::TabCapture => "Capturar áudio e vídeo de abas".to_string(),
            Permission::DesktopCapture => "Capturar a tela".to_string(),
            Permission::Management => "Gerenciar outras extensões".to_string(),
            Permission::SystemCpu => "Acessar informações da CPU".to_string(),
            Permission::SystemMemory => "Acessar informações de memória".to_string(),
            Permission::SystemDisplay => "Acessar informações de tela".to_string(),
            Permission::HostPermission(_) => "Acessar dados de sites específicos".to_string(),
            Permission::Other(s) => s.clone(),
        }
    }

    /// Nome da permissão como string.
    pub fn as_str(&self) -> String {
        match self {
            Permission::HostPermission(url) => url.clone(),
            Permission::Other(s) => s.clone(),
            _ => format!("{:?}", self).to_lowercase(),
        }
    }
}

/// Conjunto completo de permissões de uma extensão.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSet {
    pub permissions: Vec<String>,
    pub optional_permissions: Vec<String>,
    pub host_permissions: Vec<String>,
    pub optional_host_permissions: Vec<String>,
}

impl PermissionSet {
    pub fn new() -> Self {
        Self {
            permissions: vec![],
            optional_permissions: vec![],
            host_permissions: vec![],
            optional_host_permissions: vec![],
        }
    }

    /// Verifica se uma permissão específica está concedida.
    pub fn has(&self, perm: &str) -> bool {
        let p = Permission::from_str(perm);
        self.permissions.contains(&perm.to_string())
            || self.host_permissions.contains(&perm.to_string())
            || match p {
                Permission::AllUrls => !self.host_permissions.is_empty() || self.permissions.contains(&"<all_urls>".to_string()),
                _ => false,
            }
    }

    /// Retorna todas as permissões como lista de strings.
    pub fn all_permissions(&self) -> Vec<String> {
        let mut all = self.permissions.clone();
        all.extend(self.host_permissions.clone());
        all
    }

    /// Retorna o nível máximo de sensibilidade neste conjunto.
    pub fn max_level(&self) -> PermissionLevel {
        let mut max = PermissionLevel::Normal;
        for p in self.all_permissions() {
            let level = Permission::from_str(&p).level();
            if level as u8 > max as u8 {
                max = level;
            }
        }
        max
    }
}

impl Default for PermissionSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Gerenciador de permissões para extensões.
pub struct PermissionsManager {
    storage: Arc<brunya_storage::StorageManager>,
    granted: RwLock<HashMap<String, PermissionSet>>,
    pending_requests: RwLock<HashMap<String, Vec<String>>>,
}

impl PermissionsManager {
    pub fn new(storage: Arc<brunya_storage::StorageManager>) -> Self {
        Self {
            storage,
            granted: RwLock::new(HashMap::new()),
            pending_requests: RwLock::new(HashMap::new()),
        }
    }

    /// Concede permissões a uma extensão (registra no storage).
    pub fn grant_permissions(&self, ext_id: &ExtensionId, perms: &PermissionSet) -> Result<()> {
        let key = format!("ext_perms_{}", ext_id.0);
        let json = serde_json::to_string(perms)?;
        self.storage.set_preference(&key, &json)?;

        self.granted.write().insert(ext_id.0.clone(), perms.clone());
        info!("Permissões concedidas para extensão {}: {:?}", ext_id, perms.all_permissions());
        Ok(())
    }

    /// Carrega permissões previamente concedidas para uma extensão.
    pub fn load_permissions(&self, ext_id: &ExtensionId) -> PermissionSet {
        if let Some(perms) = self.granted.read().get(&ext_id.0) {
            return perms.clone();
        }

        let key = format!("ext_perms_{}", ext_id.0);
        if let Ok(Some(json)) = self.storage.get_preference(&key) {
            if let Ok(perms) = serde_json::from_str::<PermissionSet>(&json) {
                self.granted.write().insert(ext_id.0.clone(), perms.clone());
                return perms;
            }
        }

        PermissionSet::new()
    }

    /// Verifica se uma permissão específica está concedida.
    pub fn check_permission(&self, ext_id: &ExtensionId, permission: &str) -> bool {
        let perms = self.load_permissions(ext_id);
        perms.has(permission)
    }

    /// Solicita permissões adicionais (opcionais) ao usuário.
    pub fn request_permissions(&self, ext_id: &ExtensionId, new_perms: &[String]) {
        let mut pending = self.pending_requests.write();
        pending.insert(ext_id.0.clone(), new_perms.to_vec());
    }

    /// Retorna as permissões pendentes para uma extensão.
    pub fn pending_permissions(&self, ext_id: &ExtensionId) -> Vec<String> {
        self.pending_requests
            .read()
            .get(&ext_id.0)
            .cloned()
            .unwrap_or_default()
    }

    /// Revoga permissões específicas de uma extensão.
    pub fn revoke_permissions(&self, ext_id: &ExtensionId, perms_to_remove: &[String]) -> Result<()> {
        let mut perms = self.load_permissions(ext_id);
        perms.permissions.retain(|p| !perms_to_remove.contains(p));
        perms.host_permissions.retain(|p| !perms_to_remove.contains(p));
        perms.optional_permissions.retain(|p| !perms_to_remove.contains(p));

        self.grant_permissions(ext_id, &perms)
    }

    /// Revoga todas as permissões de uma extensão.
    pub fn revoke_all(&self, ext_id: &ExtensionId) -> Result<()> {
        let empty = PermissionSet::new();
        self.grant_permissions(ext_id, &empty)
    }

    /// Lista avisos de segurança para um conjunto de permissões.
    pub fn permission_warnings(permissions: &[String]) -> Vec<String> {
        let mut warnings = Vec::new();
        for p in permissions {
            let perm = Permission::from_str(p);
            match perm.level() {
                PermissionLevel::Sensitive => {
                    warnings.push(format!("⚠️ Permissão sensível: {}", perm.description()));
                }
                PermissionLevel::Warning => {
                    warnings.push(format!("ℹ️ {}", perm.description()));
                }
                PermissionLevel::Normal => {}
            }
        }
        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_levels() {
        assert_eq!(Permission::Storage.level(), PermissionLevel::Normal);
        assert_eq!(Permission::Tabs.level(), PermissionLevel::Warning);
        assert_eq!(Permission::Cookies.level(), PermissionLevel::Sensitive);
        assert_eq!(Permission::AllUrls.level(), PermissionLevel::Sensitive);
        assert_eq!(
            Permission::HostPermission("https://example.com".to_string()).level(),
            PermissionLevel::Warning
        );
    }

    #[test]
    fn test_permission_set() {
        let mut set = PermissionSet::new();
        set.permissions.push("storage".to_string());
        set.permissions.push("tabs".to_string());
        set.host_permissions.push("*://*.example.com/*".to_string());

        assert!(set.has("storage"));
        assert!(set.has("tabs"));
        assert!(!set.has("cookies"));
    }

    #[test]
    fn test_permission_warnings() {
        let perms = vec!["storage".to_string(), "cookies".to_string(), "tabs".to_string()];
        let warnings = PermissionsManager::permission_warnings(&perms);
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("cookies")));
    }
}
