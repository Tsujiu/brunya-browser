//! Sistema de extensões de terceiros do Brunya Browser.
//!
//! Implementa um polyfill sobre WebKitGTK para oferecer suporte a extensões
//! compatíveis com o ecossistema Chrome Extensions (Manifest V3).
//!
//! Arquitetura:
//!   ExtensionManager (ponto de entrada central)
//!     ├── ExtensionRegistry    — catálogo de extensões instaladas
//!     ├── ManifestParser      — interpretador de manifest.json
//!     ├── ExtensionLoader     — carregamento e validação
//!     ├── ExtensionRuntime    — execução de scripts de extensão
//!     ├── PermissionsManager  — controle de permissões
//!     ├── ApiCompatLayer      — polyfill de chrome.* APIs
//!     ├── ExtensionStorage    — armazenamento isolado por extensão
//!     ├── UpdateManager       — atualizações e verificação
//!     └── SecurityManager     — validação e isolamento

pub mod api;
pub mod errors;
pub mod inject;
pub mod loader;
pub mod manager;
pub mod manifest;
pub mod paths;
pub mod permissions;
pub mod registry;
pub mod runtime;
pub mod security;
pub mod storage;
pub mod store;
pub mod update;
pub mod types;

pub use errors::{ExtensionError, Result};
pub use manager::ExtensionManager;
pub use manifest::{Manifest, ManifestVersion, ManifestValidation};
pub use permissions::{Permission, PermissionLevel, PermissionSet};
pub use registry::ExtensionRegistry;
pub use runtime::{ExtensionRuntime, ServiceWorkerState};
pub use security::SecurityManager;
pub use storage::ExtensionStorageManager;
pub use types::{ExtensionId, ExtensionState, ExtensionInfo};
pub use update::UpdateManager;

use std::sync::Arc;

/// Sistema central de extensões da BruNya.
pub struct ExtensionSystem {
    pub manager: Arc<ExtensionManager>,
    pub registry: Arc<ExtensionRegistry>,
    pub permissions: Arc<permissions::PermissionsManager>,
    pub storage: Arc<ExtensionStorageManager>,
    pub runtime: Arc<ExtensionRuntime>,
    pub security: Arc<SecurityManager>,
    pub update_mgr: Arc<UpdateManager>,
}

impl ExtensionSystem {
    pub fn new(
        data_dir: std::path::PathBuf,
        storage_mgr: Arc<brunya_storage::StorageManager>,
    ) -> Self {
        let registry = Arc::new(ExtensionRegistry::new(data_dir.join("extensions")));
        let permissions = Arc::new(permissions::PermissionsManager::new(storage_mgr));
        let storage = Arc::new(ExtensionStorageManager::new(data_dir.join("extension-data")));
        let security = Arc::new(SecurityManager::new());
        let runtime = Arc::new(ExtensionRuntime::new(security.clone()));
        let update_mgr = Arc::new(UpdateManager::new(
            data_dir.join("extension-updates"),
            registry.clone(),
            security.clone(),
        ));

        let manager = Arc::new(ExtensionManager::new(
            registry.clone(),
            permissions.clone(),
            storage.clone(),
            runtime.clone(),
            update_mgr.clone(),
            security.clone(),
        ));

        Self {
            manager,
            registry,
            permissions,
            storage,
            runtime,
            security,
            update_mgr,
        }
    }
}
