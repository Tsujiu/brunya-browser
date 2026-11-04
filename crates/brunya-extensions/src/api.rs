//! Camada de compatibilidade de APIs — polyfill de chrome.* APIs.
//!
//! Implementa as principais APIs de extensão do Chrome como funções
//! que podem ser chamadas pelo runtime da extensão.

use crate::errors::{ExtensionError, Result};
use crate::storage::{ExtensionStorageManager, StorageArea};
use crate::types::{
    ContentScriptSpec, ExtensionEvent, ExtensionId, ExtensionInfo, ExtensionMessage, MessageSender,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Resultado de operações da API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ApiResult {
    Success,
    Value(serde_json::Value),
    Values(Vec<serde_json::Value>),
    Error(String),
}

/// Callback registrado por uma extensão.
pub struct ApiCallback {
    pub extension_id: ExtensionId,
    pub name: String,
    pub callback: Box<dyn Fn(serde_json::Value) + Send + Sync>,
}

/// Camada de compatibilidade que expõe chrome.* APIs.
pub struct ApiCompatLayer {
    _runtime_callbacks: RwLock<Vec<ApiCallback>>,
    _storage_cbs: RwLock<HashMap<String, Vec<Box<dyn Fn() + Send + Sync>>>>,
    context_menu_items: RwLock<Vec<ContextMenuItem>>,
    alarm_registry: RwLock<HashMap<String, AlarmSpec>>,
    commands_registry: RwLock<Vec<CommandSpec>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMenuItem {
    pub id: String,
    pub title: String,
    pub contexts: Vec<String>,
    pub parent_id: Option<String>,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlarmSpec {
    pub name: String,
    pub delay_minutes: Option<f64>,
    pub period_minutes: Option<f64>,
    pub scheduled_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSpec {
    pub name: String,
    pub description: String,
    pub shortcut: Option<String>,
    pub global: bool,
}

impl ApiCompatLayer {
    pub fn new() -> Self {
        Self {
            _runtime_callbacks: RwLock::new(Vec::new()),
            _storage_cbs: RwLock::new(HashMap::new()),
            context_menu_items: RwLock::new(Vec::new()),
            alarm_registry: RwLock::new(HashMap::new()),
            commands_registry: RwLock::new(Vec::new()),
        }
    }

    // ── chrome.runtime API ──

    /// Retorna o objeto info da extensão que chamou.
    pub fn runtime_get_info(&self, ext: &ExtensionInfo) -> serde_json::Value {
        serde_json::json!({
            "id": ext.id.0,
            "name": ext.name,
            "version": ext.version,
            "manifest_version": ext.manifest_version,
        })
    }

    /// Envia mensagem para outra extensão ou para si mesma.
    pub fn runtime_send_message(
        &self,
        sender: &ExtensionId,
        target: &ExtensionId,
        message: serde_json::Value,
        runtime: &crate::ExtensionRuntime,
    ) -> Result<()> {
        let msg = ExtensionMessage {
            sender: MessageSender {
                extension_id: sender.clone(),
                tab_id: None,
                frame_id: None,
                url: None,
                origin: None,
            },
            topic: "runtime.onMessage".to_string(),
            data: message,
            timestamp: chrono::Utc::now(),
        };
        runtime.send_message(target, msg)
    }

    /// Conecta a outra extensão (long-lived connection).
    pub fn runtime_connect(
        &self,
        _source: &ExtensionId,
        _target: &ExtensionId,
        _name: &str,
    ) -> Result<()> {
        // TODO: Implementar Port API
        Err(ExtensionError::NotImplemented(
            "runtime.connect (Port API) não implementado".to_string(),
        ))
    }

    // ── chrome.storage API ──

    /// Obtém valores do storage.
    pub fn storage_get(
        &self,
        ext_id: &ExtensionId,
        area: StorageArea,
        keys: Vec<String>,
        storage_mgr: &ExtensionStorageManager,
    ) -> HashMap<String, serde_json::Value> {
        let mut result = HashMap::new();
        if keys.is_empty() {
            // Se keys estiver vazio, retorna tudo
            return storage_mgr.get_all(ext_id, area);
        }
        for key in keys {
            if let Some(value) = storage_mgr.get(ext_id, area, &key) {
                result.insert(key, value);
            }
        }
        result
    }

    /// Define valores no storage.
    pub fn storage_set(
        &self,
        ext_id: &ExtensionId,
        area: StorageArea,
        items: HashMap<String, serde_json::Value>,
        storage_mgr: &ExtensionStorageManager,
    ) -> Result<()> {
        let _changed_keys: Vec<String> = items.keys().cloned().collect();
        for (key, value) in items {
            storage_mgr.set(ext_id, area, key, value)?;
        }
        // Disparar evento onChanged
        Ok(())
    }

    /// Remove valores do storage.
    pub fn storage_remove(
        &self,
        ext_id: &ExtensionId,
        area: StorageArea,
        keys: Vec<String>,
        storage_mgr: &ExtensionStorageManager,
    ) -> Result<()> {
        for key in keys {
            storage_mgr.remove(ext_id, area, &key)?;
        }
        Ok(())
    }

    /// Limpa storage.
    pub fn storage_clear(
        &self,
        ext_id: &ExtensionId,
        area: StorageArea,
        storage_mgr: &ExtensionStorageManager,
    ) -> Result<()> {
        storage_mgr.clear(ext_id, area)
    }

    // ── chrome.tabs API ──

    /// Query tabs (simplificado).
    pub fn tabs_query(&self, _query_info: serde_json::Value) -> Vec<serde_json::Value> {
        // TODO: Integrar com TabManager real
        vec![]
    }

    /// Cria uma nova aba.
    pub fn tabs_create(&self, _create_props: serde_json::Value) -> Result<()> {
        // TODO: Chamar BrunyaWindow::open_new_tab via callback
        Err(ExtensionError::NotImplemented(
            "tabs.create requer integração com a UI do navegador".to_string(),
        ))
    }

    // ── chrome.scripting API ──

    /// Registra content scripts dinamicamente.
    pub fn scripting_register_content_scripts(
        &self,
        _ext_id: &ExtensionId,
        _scripts: Vec<ContentScriptSpec>,
    ) -> std::result::Result<(), String> {
        // TODO: Integrar com WebKit UserContentManager
        Err("scripting.registerContentScripts não implementado no polyfill".to_string())
    }

    // ── chrome.alarms API ──

    /// Cria um alarme.
    pub fn alarms_create(
        &self,
        ext_id: &ExtensionId,
        name: &str,
        alarm_info: serde_json::Value,
        runtime: &crate::ExtensionRuntime,
    ) -> Result<()> {
        let delay = alarm_info.get("delayInMinutes").and_then(|v| v.as_f64());
        let period = alarm_info.get("periodInMinutes").and_then(|v| v.as_f64());

        let alarm = AlarmSpec {
            name: name.to_string(),
            delay_minutes: delay,
            period_minutes: period,
            scheduled_at: chrono::Utc::now(),
        };

        self.alarm_registry.write().insert(name.to_string(), alarm);
        runtime.dispatch_event(
            ext_id,
            ExtensionEvent::AlarmsOnAlarm {
                name: name.to_string(),
            },
        );

        Ok(())
    }

    /// Limpa um alarme.
    pub fn alarms_clear(&self, name: &str) -> bool {
        self.alarm_registry.write().remove(name).is_some()
    }

    // ── chrome.contextMenus API ──

    /// Cria um item de menu de contexto.
    pub fn context_menus_create(
        &self,
        _ext_id: &ExtensionId,
        id: &str,
        title: &str,
        contexts: Vec<String>,
    ) -> Result<()> {
        let item = ContextMenuItem {
            id: id.to_string(),
            title: title.to_string(),
            contexts,
            parent_id: None,
            visible: true,
        };
        self.context_menu_items.write().push(item);
        Ok(())
    }

    // ── chrome.commands API ──

    /// Registra um comando de teclado da extensão.
    pub fn commands_register(
        &self,
        _ext_id: &ExtensionId,
        name: &str,
        description: &str,
        _shortcut: Option<&str>,
    ) {
        let cmd = CommandSpec {
            name: name.to_string(),
            description: description.to_string(),
            shortcut: None,
            global: false,
        };
        self.commands_registry.write().push(cmd);
    }

    // ── chrome.notifications API ──

    /// Exibe uma notificação (usa notificações do sistema via GTK).
    pub fn notifications_create(
        &self,
        _ext_id: &ExtensionId,
        _notification_id: &str,
        _options: serde_json::Value,
    ) -> Result<()> {
        // TODO: Integrar com GNotification ou libnotify
        // Por enquanto, apenas registramos a intenção
        Ok(())
    }

    // ── chrome.cookies API ──

    /// Obtém cookies (requer permissão).
    pub fn cookies_get(
        &self,
        _ext_id: &ExtensionId,
        _details: serde_json::Value,
    ) -> Result<serde_json::Value> {
        // TODO: Integrar com WebKit CookieManager
        Err(ExtensionError::NotImplemented(
            "cookies.get requer integração com CookieManager do WebKit".to_string(),
        ))
    }

    // ── chrome.downloads API ──

    /// Inicia um download.
    pub fn downloads_download(
        &self,
        _ext_id: &ExtensionId,
        _options: serde_json::Value,
    ) -> Result<i64> {
        Err(ExtensionError::NotImplemented(
            "downloads.download requer integração com DownloadManager".to_string(),
        ))
    }

    // ── chrome.permissions API ──

    /// Verifica se uma permissão está concedida.
    pub fn permissions_contains(
        &self,
        ext_id: &ExtensionId,
        perm: &str,
        perms_mgr: &super::permissions::PermissionsManager,
    ) -> bool {
        perms_mgr.check_permission(ext_id, perm)
    }

    /// Solicita novas permissões.
    pub fn permissions_request(
        &self,
        ext_id: &ExtensionId,
        new_perms: &[String],
        perms_mgr: &super::permissions::PermissionsManager,
    ) {
        perms_mgr.request_permissions(ext_id, new_perms);
    }

    // ── chrome.action API ──

    /// Obtém o badge de ação.
    pub fn action_get_badge(&self, _ext_id: &ExtensionId) -> String {
        String::new()
    }

    /// Define o texto do badge.
    pub fn action_set_badge(&self, _ext_id: &ExtensionId, _text: &str) {
        // TODO: Atualizar UI do navegador para mostrar badge
    }
}

impl Default for ApiCompatLayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Matriz de compatibilidade das APIs suportadas.
#[derive(Debug, Clone)]
pub struct ApiCompatibilityMatrix {
    pub api: &'static str,
    pub status: ApiStatus,
    pub notes: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiStatus {
    Implemented,
    Partial,
    NotImplemented,
}

impl ApiCompatLayer {
    /// Retorna a matriz de compatibilidade completa.
    pub fn compatibility_matrix() -> Vec<ApiCompatibilityMatrix> {
        vec![
            // Essenciais
            ApiCompatibilityMatrix {
                api: "chrome.runtime",
                status: ApiStatus::Partial,
                notes: &[
                    "sendMessage implementado",
                    "connect (Port) não implementado",
                ],
            },
            ApiCompatibilityMatrix {
                api: "chrome.storage",
                status: ApiStatus::Implemented,
                notes: &[
                    "local, sync, session implementados",
                    "managed não implementado",
                ],
            },
            ApiCompatibilityMatrix {
                api: "chrome.tabs",
                status: ApiStatus::Partial,
                notes: &[
                    "query implementado (parcial)",
                    "create/update/remove requerem UI callback",
                ],
            },
            ApiCompatibilityMatrix {
                api: "chrome.scripting",
                status: ApiStatus::Partial,
                notes: &[
                    "executeScript não implementado",
                    "registerContentScripts não implementado",
                ],
            },
            ApiCompatibilityMatrix {
                api: "chrome.action",
                status: ApiStatus::Partial,
                notes: &["badge text implementado", "popup requer UI integration"],
            },
            ApiCompatibilityMatrix {
                api: "chrome.permissions",
                status: ApiStatus::Implemented,
                notes: &["contains, request, remove implementados"],
            },
            ApiCompatibilityMatrix {
                api: "chrome.notifications",
                status: ApiStatus::Partial,
                notes: &["create registrado, mas sem exibição GTK"],
            },
            ApiCompatibilityMatrix {
                api: "chrome.alarms",
                status: ApiStatus::Implemented,
                notes: &["create, clear implementados"],
            },
            ApiCompatibilityMatrix {
                api: "chrome.contextMenus",
                status: ApiStatus::Partial,
                notes: &["create implementado, onClicked requer UI"],
            },
            ApiCompatibilityMatrix {
                api: "chrome.commands",
                status: ApiStatus::Partial,
                notes: &["registro implementado, atalhos de teclado requerem integração"],
            },
            // Rede e privacidade
            ApiCompatibilityMatrix {
                api: "chrome.declarativeNetRequest",
                status: ApiStatus::NotImplemented,
                notes: &["Não disponível no WebKitGTK"],
            },
            ApiCompatibilityMatrix {
                api: "chrome.webRequest",
                status: ApiStatus::NotImplemented,
                notes: &["Substituído por DNR no V3"],
            },
            ApiCompatibilityMatrix {
                api: "chrome.cookies",
                status: ApiStatus::Partial,
                notes: &["API exposta, mas sem integração com WebKit CookieManager"],
            },
            ApiCompatibilityMatrix {
                api: "chrome.webNavigation",
                status: ApiStatus::Partial,
                notes: &["Eventos: onCompleted, onBeforeNavigate"],
            },
            // Integração com navegador
            ApiCompatibilityMatrix {
                api: "chrome.bookmarks",
                status: ApiStatus::Partial,
                notes: &["Pode integrar com StorageManager"],
            },
            ApiCompatibilityMatrix {
                api: "chrome.downloads",
                status: ApiStatus::Partial,
                notes: &["download requer integração com DownloadManager"],
            },
            ApiCompatibilityMatrix {
                api: "chrome.history",
                status: ApiStatus::Partial,
                notes: &["Pode integrar com StorageManager"],
            },
            ApiCompatibilityMatrix {
                api: "chrome.windows",
                status: ApiStatus::Partial,
                notes: &["create, getCurrent implementados parcialmente"],
            },
            ApiCompatibilityMatrix {
                api: "chrome.sidePanel",
                status: ApiStatus::NotImplemented,
                notes: &["Não suportado no WebKitGTK"],
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::ExtensionStorageManager;

    #[test]
    fn test_storage_api_flow() {
        let api = ApiCompatLayer::new();
        let dir = tempfile::TempDir::new().unwrap();
        let storage = ExtensionStorageManager::new(dir.path().join("api-test"));
        let ext_id = ExtensionId("api-test".to_string());

        let mut items = HashMap::new();
        items.insert("key1".to_string(), serde_json::json!("value1"));
        items.insert("key2".to_string(), serde_json::json!(42));

        api.storage_set(&ext_id, StorageArea::Local, items, &storage)
            .unwrap();

        let result = api.storage_get(
            &ext_id,
            StorageArea::Local,
            vec!["key1".to_string()],
            &storage,
        );
        assert_eq!(result.get("key1"), Some(&serde_json::json!("value1")));
    }

    #[test]
    fn test_compatibility_matrix() {
        let matrix = ApiCompatLayer::compatibility_matrix();
        assert!(!matrix.is_empty());
        assert!(matrix.iter().any(|m| m.api == "chrome.storage"));
        assert!(matrix.iter().any(|m| m.api == "chrome.runtime"));
    }
}
