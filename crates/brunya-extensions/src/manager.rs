//! Gerenciador central de extensões.
//!
//! Ponto de entrada para todo o sistema de extensões. Coordena o registro,
//! carregamento, permissões, runtime e atualizações.

use crate::api::ApiCompatLayer;
use crate::errors::Result;
use crate::loader::{ExtensionLoader, LoadedExtension};
use crate::manifest::Manifest;
use crate::permissions::{PermissionSet, PermissionsManager};
use crate::registry::ExtensionRegistry;
use crate::runtime::ExtensionRuntime;
use crate::security::SecurityManager;
use crate::storage::ExtensionStorageManager;
use crate::types::{ExtensionEvent, ExtensionId, ExtensionInfo, ExtensionState, InstallType};
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

/// Gerenciador central de extensões.
pub struct ExtensionManager {
    registry: Arc<ExtensionRegistry>,
    permissions: Arc<PermissionsManager>,
    storage: Arc<ExtensionStorageManager>,
    runtime: Arc<ExtensionRuntime>,
    update_mgr: Arc<crate::update::UpdateManager>,
    security: Arc<SecurityManager>,
    api: ApiCompatLayer,
    loader: parking_lot::RwLock<ExtensionLoader>,
}

impl ExtensionManager {
    pub fn new(
        registry: Arc<ExtensionRegistry>,
        permissions: Arc<PermissionsManager>,
        storage: Arc<ExtensionStorageManager>,
        runtime: Arc<ExtensionRuntime>,
        update_mgr: Arc<crate::update::UpdateManager>,
        security: Arc<SecurityManager>,
    ) -> Self {
        let loader = ExtensionLoader::new(registry.clone());

        Self {
            registry,
            permissions,
            storage,
            runtime,
            update_mgr,
            security,
            api: ApiCompatLayer::new(),
            loader: parking_lot::RwLock::new(loader),
        }
    }

    // ── Instalação ──

    /// Instala uma extensão a partir de um diretório local (modo desenvolvedor).
    pub fn install_from_directory(&self, dir: &Path) -> Result<ExtensionInfo> {
        let loaded = self.loader.read().load_from_directory(dir)?;
        let info = self.install_loaded(loaded)?;
        Ok(info)
    }

    /// Instala uma extensão a partir de um arquivo CRX, ZIP ou XPI.
    pub fn install_from_crx(&self, crx_path: &Path) -> Result<ExtensionInfo> {
        let loaded = self.loader.read().load_from_crx(crx_path)?;
        let info = self.install_loaded(loaded)?;
        Ok(info)
    }

    /// Instala os bytes já baixados de uma loja ou de um arquivo local.
    pub fn install_from_bytes(
        &self,
        bytes: &[u8],
        explicit_id: Option<&str>,
        install_type: InstallType,
    ) -> Result<ExtensionInfo> {
        let loaded = self
            .loader
            .read()
            .load_from_bytes(bytes, explicit_id, install_type)?;
        self.install_loaded(loaded)
    }

    /// Instala uma extensão já carregada (registro + permissões + runtime).
    fn install_loaded(&self, loaded: LoadedExtension) -> Result<ExtensionInfo> {
        if self.registry.contains(&loaded.info.id) {
            // Extensão já existe — atualizar
            self.registry.register(loaded.info.clone())?;
        } else {
            // Registrar no registry
            self.registry.register(loaded.info.clone())?;
        }

        // Conceder permissões obrigatórias
        let perm_set = PermissionSet {
            permissions: loaded.manifest.permissions.clone(),
            optional_permissions: loaded.manifest.optional_permissions.clone(),
            host_permissions: loaded.manifest.host_permissions.clone(),
            optional_host_permissions: vec![],
        };
        self.permissions
            .grant_permissions(&loaded.info.id, &perm_set)?;

        // Inicializar runtime
        self.runtime.initialize_context(&loaded.info);

        // Se tem service worker, iniciar
        if loaded.info.background_type.is_some() {
            self.runtime.start_service_worker(&loaded.info.id).ok();
        }

        // Disparar evento onInstalled
        if loaded.info.state != ExtensionState::DeveloperMode {
            self.runtime.dispatch_event(
                &loaded.info.id,
                ExtensionEvent::RuntimeOnInstalled {
                    reason: "install".to_string(),
                    previous_version: None,
                },
            );
        }

        info!(
            "Extensão instalada com sucesso: {} v{} (ID: {})",
            loaded.info.name, loaded.info.version, loaded.info.id
        );

        Ok(loaded.info)
    }

    // ── Gerenciamento ──

    /// Ativa uma extensão desativada.
    pub fn enable_extension(&self, id: &ExtensionId) -> Result<()> {
        self.registry.set_state(id, ExtensionState::Active)?;

        if let Some(info) = self.registry.get(id) {
            self.runtime.initialize_context(&info);
            if info.background_type.is_some() {
                self.runtime.start_service_worker(id).ok();
            }
            self.runtime
                .dispatch_event(id, ExtensionEvent::RuntimeOnStartup);
        }

        info!("Extensão ativada: {}", id);
        Ok(())
    }

    /// Desativa uma extensão (preserva dados).
    pub fn disable_extension(&self, id: &ExtensionId) -> Result<()> {
        self.registry.set_state(id, ExtensionState::Disabled)?;
        self.runtime.suspend_service_worker(id).ok();
        self.runtime.remove_context(id);

        info!("Extensão desativada: {}", id);
        Ok(())
    }

    /// Remove completamente uma extensão.
    pub fn remove_extension(&self, id: &ExtensionId) -> Result<()> {
        // Remover contexto de runtime
        self.runtime.remove_context(id);

        // Remover armazenamento
        self.storage.remove_extension_store(id)?;

        // Revogar permissões
        self.permissions.revoke_all(id)?;

        // Remover do registro e arquivos
        self.loader.read().remove_extension(id)?;

        info!("Extensão removida completamente: {}", id);
        Ok(())
    }

    // ── Consulta ──

    /// Obtém informações de uma extensão.
    pub fn get_extension(&self, id: &ExtensionId) -> Option<ExtensionInfo> {
        self.registry.get(id)
    }

    /// Lista todas as extensões instaladas.
    pub fn list_extensions(&self) -> Vec<ExtensionInfo> {
        self.registry.list_all()
    }

    /// Lista extensões ativas.
    pub fn list_active_extensions(&self) -> Vec<ExtensionInfo> {
        self.registry.list_active()
    }

    /// Lista extensões por estado.
    pub fn list_extensions_by_state(&self, state: ExtensionState) -> Vec<ExtensionInfo> {
        self.registry.list_by_state(state)
    }

    /// Número total de extensões.
    pub fn extension_count(&self) -> usize {
        self.registry.count()
    }

    // ── Runtime ──

    /// Referência ao runtime.
    pub fn runtime(&self) -> &ExtensionRuntime {
        &self.runtime
    }

    /// Referência ao gerenciador de permissões.
    pub fn permissions_manager(&self) -> &PermissionsManager {
        &self.permissions
    }

    /// Referência ao armazenamento de extensões.
    pub fn storage_manager(&self) -> &ExtensionStorageManager {
        &self.storage
    }

    /// Referência ao security manager.
    pub fn security_manager(&self) -> &SecurityManager {
        &self.security
    }

    /// Referência ao registry.
    pub fn registry(&self) -> &ExtensionRegistry {
        &self.registry
    }

    /// Referência à camada de API.
    pub fn api_layer(&self) -> &ApiCompatLayer {
        &self.api
    }

    /// Referência ao update manager.
    pub fn update_manager(&self) -> &crate::update::UpdateManager {
        &self.update_mgr
    }

    // ── Carregamento na inicialização ──

    /// Carrega todas as extensões ativas do registro na inicialização.
    pub fn load_active_extensions_on_startup(&self) -> Vec<ExtensionId> {
        let mut loaded = Vec::new();
        let active = self.registry.list_active();

        for ext in active {
            // Verificar se diretório ainda existe
            if !ext.directory.exists() && ext.install_type != InstallType::Unpacked {
                warn!(
                    "Diretório da extensão não encontrado: {} ({})",
                    ext.name,
                    ext.directory.display()
                );
                self.registry.set_state(&ext.id, ExtensionState::Error).ok();
                continue;
            }

            // Recarregar manifest para verificar se ainda é válido
            if let Ok((manifest, _)) = Manifest::load_from_dir(&ext.directory) {
                let validation = manifest.validate();
                if !validation.is_valid {
                    warn!(
                        "Manifest inválido na inicialização: {} ({})",
                        ext.name, ext.id
                    );
                    self.registry.set_state(&ext.id, ExtensionState::Error).ok();
                    continue;
                }

                // Recarregar permissões do storage persistido
                let _granted_perms = self.permissions.load_permissions(&ext.id);

                // Inicializar runtime
                self.runtime.initialize_context(&ext);

                // Disparar evento onStartup
                self.runtime
                    .dispatch_event(&ext.id, ExtensionEvent::RuntimeOnStartup);

                loaded.push(ext.id);
            } else {
                warn!(
                    "Falha ao carregar manifest na inicialização: {} ({})",
                    ext.name, ext.id
                );
                self.registry.set_state(&ext.id, ExtensionState::Error).ok();
            }
        }

        info!("{} extensões carregadas na inicialização", loaded.len());
        loaded
    }

    // ── Suspensão periódica de SWs ──

    /// Suspende service workers ociosos.
    pub fn suspend_idle_workers(&self) {
        let timeout = std::time::Duration::from_secs(30);
        let suspended = self.runtime.suspend_idle_workers(timeout);
        if !suspended.is_empty() {
            info!(
                "{} Service Workers suspensos por inatividade",
                suspended.len()
            );
        }
    }

    // ── Verificação de atualizações ──

    /// Verifica atualizações periodicamente.
    pub fn periodic_update_check(&self) {
        let updates = self.update_mgr.periodic_check();
        for update in updates {
            if update.available_version.is_some() {
                info!(
                    "Atualização disponível para {}: v{} → v{}",
                    update.current_version,
                    update.available_version.unwrap_or_default(),
                    update.ext_id
                );
            }
        }
    }

    // ── Matriz de compatibilidade ──

    /// Retorna a matriz de compatibilidade de APIs.
    pub fn api_compatibility_matrix() -> Vec<crate::api::ApiCompatibilityMatrix> {
        ApiCompatLayer::compatibility_matrix()
    }

    /// Obtém avisos de compatibilidade para uma extensão.
    pub fn get_compatibility_warnings(&self, id: &ExtensionId) -> Vec<String> {
        let mut warnings = Vec::new();

        if let Some(info) = self.registry.get(id) {
            // Verificar manifest version
            if info.manifest_version == 2 {
                warnings.push(
                    "Manifest V2 tem suporte limitado. Considere migrar para V3.".to_string(),
                );
            }

            // Verificar permissões não implementadas
            for perm in &info.permissions {
                match perm.as_str() {
                    "declarativeNetRequest" | "nativeMessaging" | "privacy" | "tabCapture" => {
                        warnings.push(format!(
                            "Permissão '{}' não está disponível no polyfill atual.",
                            perm
                        ));
                    }
                    _ => {}
                }
            }
        }

        warnings
    }
}

#[cfg(test)]
mod tests {
    use crate::ExtensionSystem;
    use std::sync::Arc;

    fn create_test_system() -> ExtensionSystem {
        let dir = tempfile::TempDir::new().unwrap();
        let storage_mgr = Arc::new(brunya_storage::StorageManager::open_in_memory().unwrap());
        ExtensionSystem::new(dir.path().to_path_buf(), storage_mgr)
    }

    #[test]
    fn test_empty_registry() {
        let system = create_test_system();
        let extensions = system.manager.list_extensions();
        assert!(extensions.is_empty());
        assert_eq!(system.manager.extension_count(), 0);
    }

    #[test]
    fn test_load_active_extensions_empty() {
        let system = create_test_system();
        let loaded = system.manager.load_active_extensions_on_startup();
        assert!(loaded.is_empty());
    }
}
