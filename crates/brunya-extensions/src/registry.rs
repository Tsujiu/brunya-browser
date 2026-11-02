//! Registro central de extensões instaladas no Brunya Browser.
//!
//! Mantém um catálogo persistente de todas as extensões, seus estados,
//! versões e metadados.

use crate::errors::Result;
use crate::types::{ExtensionId, ExtensionInfo, ExtensionState};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryData {
    extensions: HashMap<String, ExtensionInfo>,
    version: u32,
}

impl Default for RegistryData {
    fn default() -> Self {
        Self {
            extensions: HashMap::new(),
            version: 1,
        }
    }
}

pub struct ExtensionRegistry {
    data_dir: PathBuf,
    registry_path: PathBuf,
    data: RwLock<RegistryData>,
}

impl ExtensionRegistry {
    pub fn new(data_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&data_dir).ok();
        let registry_path = data_dir.join("registry.json");

        let data = if registry_path.exists() {
            match std::fs::read_to_string(&registry_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(e) => {
                    warn!("Falha ao ler registro de extensões: {}. Usando vazio.", e);
                    RegistryData::default()
                }
            }
        } else {
            RegistryData::default()
        };

        Self {
            data_dir,
            registry_path,
            data: RwLock::new(data),
        }
    }

    /// Persiste o registro atual em disco.
    pub fn save(&self) -> Result<()> {
        let data = self.data.read();
        let json = serde_json::to_string_pretty(&*data)?;
        std::fs::write(&self.registry_path, json)?;
        Ok(())
    }

    /// Registra uma nova extensão ou atualiza uma existente.
    pub fn register(&self, info: ExtensionInfo) -> Result<()> {
        {
            let mut data = self.data.write();
            data.extensions.insert(info.id.0.clone(), info);
        }
        self.save()
    }

    /// Remove uma extensão do registro.
    pub fn unregister(&self, id: &ExtensionId) -> Result<Option<ExtensionInfo>> {
        let removed = {
            let mut data = self.data.write();
            data.extensions.remove(&id.0)
        };
        if removed.is_some() {
            self.save()?;
            info!("Extensão removida do registro: {}", id);
        }
        Ok(removed)
    }

    /// Atualiza o estado de uma extensão.
    pub fn set_state(&self, id: &ExtensionId, state: ExtensionState) -> Result<bool> {
        let updated = {
            let mut data = self.data.write();
            if let Some(ref mut info) = data.extensions.get_mut(&id.0) {
                info.state = state;
                info.updated_at = chrono::Utc::now();
                true
            } else {
                false
            }
        };
        if updated {
            self.save()?;
        }
        Ok(updated)
    }

    /// Atualiza a versão de uma extensão (após update).
    pub fn update_version(&self, id: &ExtensionId, new_version: &str, new_dir: PathBuf) -> Result<bool> {
        let updated = {
            let mut data = self.data.write();
            if let Some(ref mut info) = data.extensions.get_mut(&id.0) {
                info.version = new_version.to_string();
                info.directory = new_dir;
                info.updated_at = chrono::Utc::now();
                true
            } else {
                false
            }
        };
        if updated {
            self.save()?;
        }
        Ok(updated)
    }

    /// Obtém informações de uma extensão pelo ID.
    pub fn get(&self, id: &ExtensionId) -> Option<ExtensionInfo> {
        self.data.read().extensions.get(&id.0).cloned()
    }

    /// Lista todas as extensões registradas.
    pub fn list_all(&self) -> Vec<ExtensionInfo> {
        self.data
            .read()
            .extensions
            .values()
            .cloned()
            .collect()
    }

    /// Lista extensões por estado.
    pub fn list_by_state(&self, state: ExtensionState) -> Vec<ExtensionInfo> {
        self.data
            .read()
            .extensions
            .values()
            .filter(|e| e.state == state)
            .cloned()
            .collect()
    }

    /// Extensões habilitadas: instaladas da loja e carregadas em modo desenvolvedor.
    pub fn list_active(&self) -> Vec<ExtensionInfo> {
        self.data
            .read()
            .extensions
            .values()
            .filter(|ext| {
                matches!(
                    ext.state,
                    ExtensionState::Active | ExtensionState::DeveloperMode
                )
            })
            .cloned()
            .collect()
    }

    /// Verifica se uma extensão está registrada.
    pub fn contains(&self, id: &ExtensionId) -> bool {
        self.data.read().extensions.contains_key(&id.0)
    }

    /// Retorna o número de extensões registradas.
    pub fn count(&self) -> usize {
        self.data.read().extensions.len()
    }

    /// Obtém o diretório de dados de extensões.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Cria o diretório para uma extensão específica.
    pub fn extension_dir(&self, id: &ExtensionId) -> PathBuf {
        self.data_dir.join(&id.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ExtensionId, InstallType};

    fn create_test_info(id: &str) -> ExtensionInfo {
        ExtensionInfo {
            id: ExtensionId(id.to_string()),
            name: format!("Test {}", id),
            short_name: None,
            description: Some("A test extension".to_string()),
            version: "1.0.0".to_string(),
            author: None,
            homepage_url: None,
            icons: vec![],
            state: ExtensionState::Active,
            install_type: InstallType::Normal,
            manifest_version: 3,
            permissions: vec![],
            optional_permissions: vec![],
            host_permissions: vec![],
            installed_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            directory: PathBuf::from("/tmp/test-ext"),
            has_options_page: false,
            has_popup: false,
            popup_path: None,
            options_page: None,
            background_type: None,
            content_scripts: vec![],
        }
    }

    #[test]
    fn test_registry_lifecycle() {
        let dir = tempfile::TempDir::new().unwrap();
        let registry = ExtensionRegistry::new(dir.path().join("ext-reg"));

        let id = ExtensionId("test-ext-1".to_string());
        let info = create_test_info("test-ext-1");

        registry.register(info.clone()).unwrap();
        assert_eq!(registry.count(), 1);
        assert!(registry.contains(&id));

        let retrieved = registry.get(&id).unwrap();
        assert_eq!(retrieved.name, "Test test-ext-1");

        registry.set_state(&id, ExtensionState::Disabled).unwrap();
        assert_eq!(registry.get(&id).unwrap().state, ExtensionState::Disabled);

        registry.unregister(&id).unwrap();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_persistence() {
        let dir = tempfile::TempDir::new().unwrap();
        let reg_path = dir.path().join("persist");

        {
            let registry = ExtensionRegistry::new(reg_path.clone());
            let info = create_test_info("persist-ext");
            registry.register(info).unwrap();
        }

        // Recarregar do disco
        let registry = ExtensionRegistry::new(reg_path);
        assert_eq!(registry.count(), 1);
        assert!(registry.contains(&ExtensionId("persist-ext".to_string())));
    }
}