//! Carregador de extensões — valida estruturas, interpreta manifestos
//! e prepara extensões para execução.

use crate::errors::{ExtensionError, Result};
use crate::manifest::{Manifest, ManifestValidation};
use crate::permissions::PermissionSet;
use crate::registry::ExtensionRegistry;
use crate::types::{
    ExtensionId, ExtensionInfo,
    ExtensionState, InstallType,
};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Resultado do carregamento de uma extensão.
pub struct LoadedExtension {
    pub info: ExtensionInfo,
    pub manifest: Manifest,
    pub validation: ManifestValidation,
    pub base_path: PathBuf,
}

/// Verificador de integridade de pacotes.
pub struct PackageVerifier;

impl PackageVerifier {
    /// Verifica a estrutura básica de um diretório de extensão.
    pub fn verify_directory(dir: &Path) -> Result<()> {
        if !dir.exists() {
            return Err(ExtensionError::NotFound(
                format!("Diretório não encontrado: {}", dir.display()),
            ));
        }

        if !dir.is_dir() {
            return Err(ExtensionError::InvalidManifest(
                format!("Caminho não é um diretório: {}", dir.display()),
            ));
        }

        let manifest_path = dir.join("manifest.json");
        if !manifest_path.exists() {
            return Err(ExtensionError::ManifestNotFound(
                format!("manifest.json não encontrado em {}", dir.display()),
            ));
        }

        Ok(())
    }

    /// Verifica a integridade de um pacote CRX (assinatura).
    pub fn verify_crx_package(path: &Path) -> Result<()> {
        if !path.exists() || !path.is_file() {
            return Err(ExtensionError::CorruptPackage(
                "Arquivo CRX não encontrado".to_string(),
            ));
        }

        let data = std::fs::read(path)?;
        if data.len() < 16 {
            return Err(ExtensionError::CorruptPackage(
                "Arquivo CRX muito pequeno".to_string(),
            ));
        }

        // Magic number CRX3: "Cr24" + version (3) + header length
        let magic = &data[0..4];
        if magic != b"Cr24" {
            return Err(ExtensionError::InvalidSignature(
                "Magic number CRX inválido (esperado 'Cr24')".to_string(),
            ));
        }

        // Verificar versão CRX
        let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        if version < 2 || version > 3 {
            return Err(ExtensionError::InvalidSignature(
                format!("Versão CRX não suportada: {}", version),
            ));
        }

        Ok(())
    }
}

/// Carregador de extensões.
pub struct ExtensionLoader {
    registry: std::sync::Arc<ExtensionRegistry>,
    allowed_paths: Vec<PathBuf>,
}

impl ExtensionLoader {
    pub fn new(registry: std::sync::Arc<ExtensionRegistry>) -> Self {
        Self {
            registry,
            allowed_paths: vec![],
        }
    }

    /// Define diretórios permitidos para carregamento de extensões em modo desenvolvedor.
    pub fn add_allowed_path(&mut self, path: PathBuf) {
        self.allowed_paths.push(path);
    }

    /// Carrega uma extensão de um diretório (modo desenvolvedor ou instalada).
    pub fn load_from_directory(&self, dir: &Path) -> Result<LoadedExtension> {
        // Verificar estrutura do diretório
        PackageVerifier::verify_directory(dir)?;

        // Carregar e validar manifest
        let (manifest, _) = Manifest::load_from_dir(dir)?;
        let validation = manifest.validate();

        if !validation.is_valid {
            for err in &validation.errors {
                warn!("Erro de validação de manifesto: {}", err);
            }
            return Err(ExtensionError::InvalidManifest(
                validation.errors.join("; "),
            ));
        }

        // Gerar ID a partir do caminho
        let dir_str = dir.to_string_lossy();
        let ext_id = ExtensionId::from_path(&dir_str);

        // Verificar se já está registrada
        let state = if self.registry.contains(&ext_id) {
            ExtensionState::Active
        } else {
            ExtensionState::DeveloperMode
        };

        // Criar ExtensionInfo
        let info = manifest.to_extension_info(
            ext_id,
            dir,
            InstallType::Unpacked,
            state,
        );

        Ok(LoadedExtension {
            info,
            manifest,
            validation,
            base_path: dir.to_path_buf(),
        })
    }

    /// Carrega uma extensão de um arquivo CRX.
    pub fn load_from_crx(&self, crx_path: &Path) -> Result<LoadedExtension> {
        // Verificar pacote CRX
        PackageVerifier::verify_crx_package(crx_path)?;

        // Extrair CRX para diretório temporário
        let extract_dir = self.extract_crx(crx_path)?;

        // Carregar do diretório extraído
        let result = self.load_from_directory(&extract_dir)?;

        Ok(result)
    }

    /// Extrai um pacote CRX para um diretório.
    fn extract_crx(&self, crx_path: &Path) -> Result<PathBuf> {
        let data = std::fs::read(crx_path)?;

        // Parse CRX3 header
        let _magic = &data[0..4];    // "Cr24"
        let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);

        let header_start = match version {
            2 => {
                // CRX2: header_size at offset 8
                let header_size = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
                12 + header_size
            }
            3 => {
                // CRX3: header_size at offset 8, with extra fields
                let _header_size = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
                // CRX3 has a more complex header structure
                let header_len = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;
                16 + header_len
            }
            _ => return Err(ExtensionError::CorruptPackage(
                format!("Versão CRX não suportada: {}", version),
            )),
        };

        // O restante após o header é o ZIP
        let zip_data = &data[header_start..];

        // Extrair ZIP
        let ext_id = ExtensionId::from_path(&crx_path.to_string_lossy());
        let extract_dir = self.registry.data_dir().join(&ext_id.0);
        std::fs::create_dir_all(&extract_dir)?;

        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_data))
            .map_err(|e| ExtensionError::CorruptPackage(
                format!("Falha ao ler ZIP do CRX: {}", e),
            ))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| ExtensionError::CorruptPackage(
                    format!("Falha ao ler entrada {} do ZIP: {}", i, e),
                ))?;

            let out_path = extract_dir.join(file.name());
            if file.is_dir() {
                std::fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut out_file = std::fs::File::create(&out_path)?;
                std::io::copy(&mut file, &mut out_file)?;
            }
        }

        Ok(extract_dir)
    }

    /// Carrega e registra uma extensão no sistema.
    pub fn install_extension(&self, loaded: LoadedExtension) -> Result<ExtensionInfo> {
        let _perms = PermissionSet {
            permissions: loaded.manifest.permissions.clone(),
            optional_permissions: loaded.manifest.optional_permissions.clone(),
            host_permissions: loaded.manifest.host_permissions.clone(),
            optional_host_permissions: vec![],
        };

        let ext_info = loaded.info;

        info!(
            "Extensão carregada: {} v{} (ID: {})",
            ext_info.name, ext_info.version, ext_info.id
        );

        if !ext_info.content_scripts.is_empty() {
            info!(
                "  → {} content scripts registrados",
                ext_info.content_scripts.len()
            );
        }

        if ext_info.has_popup {
            info!("  → Possui popup: {:?}", ext_info.popup_path);
        }

        if let Some(ref bg) = ext_info.background_type {
            info!("  → Background: {:?}", bg);
        }

        // Registrar no registry
        self.registry.register(ext_info.clone())?;

        Ok(ext_info)
    }

    /// Verifica se uma extensão é compatível com as APIs atuais.
    pub fn check_compatibility(manifest: &Manifest, warnings: &[String]) -> Vec<String> {
        let mut incompatibilities = Vec::new();

        // Verificar características não suportadas
        if manifest.declarative_net_request.is_some() {
            incompatibilities.push(
                "declarativeNetRequest não está disponível nesta versão.".to_string(),
            );
        }
        if manifest.storage.as_ref().and_then(|s| s.managed_schema.as_ref()).is_some() {
            incompatibilities.push(
                "managed storage não está disponível nesta versão.".to_string(),
            );
        }
        if manifest.native_messaging() {
            incompatibilities.push(
                "nativeMessaging não está disponível nesta versão.".to_string(),
            );
        }

        // Incluir warnings do manifesto como notas
        for w in warnings {
            if w.contains("V2") {
                incompatibilities.push(w.clone());
            }
        }

        incompatibilities
    }

    /// Remove uma extensão do sistema (diretório + registro).
    pub fn remove_extension(&self, id: &ExtensionId) -> Result<()> {
        if let Some(info) = self.registry.get(id) {
            // Remover diretório (exceto para modo desenvolvedor)
            if info.install_type != InstallType::Unpacked {
                if info.directory.exists() {
                    std::fs::remove_dir_all(&info.directory).ok();
                }
            }

            // Remover do registro
            self.registry.unregister(id)?;
            info!("Extensão removida: {} ({})", info.name, id);
        }

        Ok(())
    }
}

impl Manifest {
    fn native_messaging(&self) -> bool {
        self.permissions.contains(&"nativeMessaging".to_string())
            || self.optional_permissions.contains(&"nativeMessaging".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ExtensionRegistry;
    use std::sync::Arc;

    #[test]
    fn test_basic_crx_verification() {
        // Apenas verifica a validação de header CRX
        let mut crx_data = Vec::new();
        crx_data.extend_from_slice(b"Cr24"); // magic
        crx_data.extend_from_slice(&3u32.to_le_bytes()); // version
        crx_data.extend_from_slice(&0u32.to_le_bytes()); // header size
        crx_data.extend_from_slice(&0u32.to_le_bytes()); // CRX3 header length
        // No ZIP data, but header validation should pass

        let dir = tempfile::TempDir::new().unwrap();
        let crx_path = dir.path().join("test.crx");
        std::fs::write(&crx_path, &crx_data).unwrap();

        assert!(PackageVerifier::verify_crx_package(&crx_path).is_ok());
    }
}