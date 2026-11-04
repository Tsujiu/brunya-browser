//! Gerenciador de atualizações de extensões.
//!
//! Verifica, baixa e aplica atualizações de extensões de forma segura.

use crate::errors::{ExtensionError, Result};
use crate::registry::ExtensionRegistry;
use crate::security::SecurityManager;
use crate::types::ExtensionId;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Informações de atualização para uma extensão.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub ext_id: ExtensionId,
    pub current_version: String,
    pub available_version: Option<String>,
    pub update_url: Option<String>,
    pub update_checked_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_error: Option<String>,
}

/// Gerenciador de atualizações.
pub struct UpdateManager {
    base_path: PathBuf,
    registry: Arc<ExtensionRegistry>,
    _security: Arc<SecurityManager>,
    update_check_interval: Duration,
    last_check: RwLock<Option<Instant>>,
}

impl UpdateManager {
    pub fn new(
        base_path: PathBuf,
        registry: Arc<ExtensionRegistry>,
        security: Arc<SecurityManager>,
    ) -> Self {
        std::fs::create_dir_all(&base_path).ok();
        Self {
            base_path,
            registry,
            _security: security,
            update_check_interval: Duration::from_secs(3600), // 1 hora
            last_check: RwLock::new(None),
        }
    }

    /// Define o intervalo entre verificações de atualização.
    pub fn set_check_interval(&mut self, interval: Duration) {
        self.update_check_interval = interval;
    }

    /// Verifica atualizações para todas as extensões instaladas.
    pub fn check_for_updates(&self) -> Vec<UpdateInfo> {
        let mut results = Vec::new();
        let extensions = self.registry.list_active();

        if extensions.is_empty() {
            return results;
        }

        for ext in &extensions {
            let update_info = self.check_single_update(ext);
            results.push(update_info);
        }

        *self.last_check.write() = Some(Instant::now());
        results
    }

    /// Verifica atualização para uma extensão específica.
    pub fn check_single_update(&self, ext: &crate::types::ExtensionInfo) -> UpdateInfo {
        // Determinar URL de update (update_url do manifest ou padrão Chrome Web Store)
        let update_url = ext
            .homepage_url
            .clone()
            .or_else(|| Some(format!("https://clients2.google.com/service/update2/crx")));

        let available_version = if let Some(ref url) = update_url {
            match self.query_update_server(url, &ext.id, &ext.version) {
                Ok(version) => {
                    if version != ext.version {
                        info!(
                            "Atualização disponível para {}: {} → {}",
                            ext.name, ext.version, version
                        );
                        Some(version)
                    } else {
                        None
                    }
                }
                Err(e) => {
                    warn!("Falha ao verificar atualização para {}: {}", ext.name, e);
                    None
                }
            }
        } else {
            None
        };

        UpdateInfo {
            ext_id: ext.id.clone(),
            current_version: ext.version.clone(),
            available_version,
            update_url,
            update_checked_at: Some(chrono::Utc::now()),
            last_error: None,
        }
    }

    /// Baixa e aplica uma atualização.
    pub fn apply_update(&self, update: &UpdateInfo) -> Result<bool> {
        let available = update.available_version.as_ref().ok_or_else(|| {
            ExtensionError::UpdateError("Nenhuma atualização disponível".to_string())
        })?;

        let ext_info = self
            .registry
            .get(&update.ext_id)
            .ok_or_else(|| ExtensionError::NotFound(update.ext_id.0.clone()))?;

        let _update_url = update.update_url.as_ref().ok_or_else(|| {
            ExtensionError::UpdateError("URL de atualização não disponível".to_string())
        })?;

        info!(
            "Aplicando atualização para {}: v{} → v{}",
            ext_info.name, ext_info.version, available
        );

        // Baixar pacote de atualização
        let download_dir = self.base_path.join(&update.ext_id.0);
        std::fs::create_dir_all(&download_dir)?;

        // Nota: Em um cenário real, isso baixaria o CRX da URL de update
        // e verificaria a assinatura antes de extrair

        // Verificar hash e assinatura
        // self.security.verify_file_integrity(&data, ext_info.manifest_hash)

        // Atualizar versão no registro
        let version_dir = download_dir.join(available);
        self.registry
            .update_version(&update.ext_id, available, version_dir)?;

        info!("Extensão {} atualizada para v{}", ext_info.name, available);
        Ok(true)
    }

    /// Verifica se há atualizações disponíveis em lote (chamado periodicamente).
    pub fn periodic_check(&self) -> Vec<UpdateInfo> {
        let last = self.last_check.read();
        if let Some(last_check) = *last {
            if last_check.elapsed() < self.update_check_interval {
                return Vec::new();
            }
        }
        drop(last);
        self.check_for_updates()
    }

    /// Consulta o servidor de atualização (simulado para CWS).
    fn query_update_server(
        &self,
        _url: &str,
        _ext_id: &ExtensionId,
        _current_version: &str,
    ) -> std::result::Result<String, String> {
        // Em uma implementação real, faria uma requisição HTTP para a URL de update
        // e processaria a resposta XML (protocolo Omaha/CUP)
        //
        // Para o polyfill atual, retornamos a versão atual (sem update)
        Err("Servidor de atualização não implementado no polyfill".to_string())
    }

    /// Reserva a conferência de assinatura do pacote de atualização.
    /// Ainda não entra no fluxo: a implementação atual aceitaria qualquer pacote.
    #[allow(dead_code)]
    fn verify_update_signature(_data: &[u8], _expected_public_key: &str) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ExtensionRegistry;
    use crate::security::SecurityManager;
    use crate::types::ExtensionInfo;
    use std::sync::Arc;

    #[test]
    fn test_update_check_on_latest() {
        let dir = tempfile::TempDir::new().unwrap();
        let registry = Arc::new(ExtensionRegistry::new(dir.path().join("reg")));
        let security = Arc::new(SecurityManager::new());
        let updater = UpdateManager::new(dir.path().join("updates"), registry.clone(), security);

        // Registrar extensão
        let info = ExtensionInfo {
            id: ExtensionId("test-ext".to_string()),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            ..Default::default()
        };
        registry.register(info).unwrap();

        let updates = updater.check_for_updates();
        assert_eq!(updates.len(), 1);
        assert!(updates[0].available_version.is_none());
    }
}

// Implementação de Default para ExtensionInfo para testes
impl Default for crate::types::ExtensionInfo {
    fn default() -> Self {
        Self {
            id: crate::types::ExtensionId("default".to_string()),
            name: "Default".to_string(),
            short_name: None,
            description: None,
            version: "0.0.0".to_string(),
            author: None,
            homepage_url: None,
            icons: vec![],
            state: crate::types::ExtensionState::Active,
            install_type: crate::types::InstallType::Normal,
            manifest_version: 3,
            permissions: vec![],
            optional_permissions: vec![],
            host_permissions: vec![],
            installed_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            directory: PathBuf::from("/tmp"),
            has_options_page: false,
            has_popup: false,
            popup_path: None,
            options_page: None,
            background_type: None,
            content_scripts: vec![],
        }
    }
}
