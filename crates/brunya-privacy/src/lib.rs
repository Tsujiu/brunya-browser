//! Políticas de privacidade, permissões de sites e modo privado do Brunya Browser.

use brunya_storage::{PermissionState, StorageManager};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionType {
    Camera,
    Microphone,
    Geolocation,
    Notifications,
}

impl PermissionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionType::Camera => "camera",
            PermissionType::Microphone => "microphone",
            PermissionType::Geolocation => "geolocation",
            PermissionType::Notifications => "notifications",
        }
    }
}

pub struct PrivacyManager {
    storage: Option<Arc<StorageManager>>,
    is_private_mode: bool,
    temporary_permissions: RwLock<std::collections::HashMap<(String, String), PermissionState>>,
}

impl PrivacyManager {
    pub fn new(storage: Option<Arc<StorageManager>>, is_private_mode: bool) -> Self {
        Self {
            storage,
            is_private_mode,
            temporary_permissions: RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn is_private(&self) -> bool {
        self.is_private_mode
    }

    pub fn get_permission(&self, origin: &str, perm: PermissionType) -> PermissionState {
        let perm_str = perm.as_str();

        // No modo privado, lê de cache temporário
        if self.is_private_mode {
            let temp = self.temporary_permissions.read();
            if let Some(state) = temp.get(&(origin.to_string(), perm_str.to_string())) {
                return *state;
            }
            return PermissionState::Ask;
        }

        if let Some(ref storage) = self.storage {
            storage
                .get_site_permission(origin, perm_str)
                .unwrap_or(PermissionState::Ask)
        } else {
            PermissionState::Ask
        }
    }

    pub fn set_permission(&self, origin: &str, perm: PermissionType, state: PermissionState) {
        let perm_str = perm.as_str();

        if self.is_private_mode {
            self.temporary_permissions
                .write()
                .insert((origin.to_string(), perm_str.to_string()), state);
            return;
        }

        if let Some(ref storage) = self.storage {
            storage.set_site_permission(origin, perm_str, state).ok();
        }
    }

    pub fn clear_private_session_data(&self) {
        if self.is_private_mode {
            info!("Limpando todos os dados da sessão privada...");
            self.temporary_permissions.write().clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_private_mode_isolation() {
        let priv_mgr = PrivacyManager::new(None, true);
        assert!(priv_mgr.is_private());

        priv_mgr.set_permission(
            "https://example.com",
            PermissionType::Camera,
            PermissionState::Allow,
        );
        assert_eq!(
            priv_mgr.get_permission("https://example.com", PermissionType::Camera),
            PermissionState::Allow
        );

        priv_mgr.clear_private_session_data();
        assert_eq!(
            priv_mgr.get_permission("https://example.com", PermissionType::Camera),
            PermissionState::Ask
        );
    }
}
