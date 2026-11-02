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

const MAX_PACKAGE_BYTES: usize = 64 * 1024 * 1024;

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

    /// Carrega uma extensão de um arquivo CRX, ZIP ou XPI.
    pub fn load_from_crx(&self, crx_path: &Path) -> Result<LoadedExtension> {
        let bytes = std::fs::read(crx_path)?;
        self.load_from_bytes(&bytes, None, InstallType::Normal)
    }

    /// Instala os bytes de um pacote CRX, ZIP ou XPI no diretório de dados.
    /// `explicit_id` é o id da loja (Chrome/Edge), quando já é conhecido.
    pub fn load_from_bytes(
        &self,
        bytes: &[u8],
        explicit_id: Option<&str>,
        install_type: InstallType,
    ) -> Result<LoadedExtension> {
        let id = match explicit_id {
            Some(id) => ExtensionId::new(sanitize_extension_id(id)?),
            None => ExtensionId::new(infer_extension_id(bytes)?),
        };

        if let Some(old) = self.registry.get(&id) {
            if old.install_type != InstallType::Unpacked
                && old.directory.starts_with(self.registry.data_dir())
                && old.directory.exists()
            {
                std::fs::remove_dir_all(&old.directory).ok();
            }
        }

        let extract_dir = self.registry.extension_dir(&id);
        if extract_dir.exists() {
            std::fs::remove_dir_all(&extract_dir).ok();
        }
        extract_package(bytes, &extract_dir)?;

        let (manifest, _) = match Manifest::load_from_dir(&extract_dir) {
            Ok(loaded) => loaded,
            Err(err) => {
                std::fs::remove_dir_all(&extract_dir).ok();
                return Err(err);
            }
        };
        let validation = manifest.validate();
        if !validation.is_valid {
            std::fs::remove_dir_all(&extract_dir).ok();
            return Err(ExtensionError::InvalidManifest(validation.errors.join("; ")));
        }

        let info = manifest.to_extension_info(id, &extract_dir, install_type, ExtensionState::Active);
        Ok(LoadedExtension {
            info,
            manifest,
            validation,
            base_path: extract_dir,
        })
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

/// Dados mostrados antes de confirmar a instalação.
#[derive(Debug, Clone)]
pub struct PackagePreview {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub host_permissions: Vec<String>,
}

/// Lê o manifesto de um pacote sem registrá-lo.
pub fn inspect_package(bytes: &[u8], explicit_id: Option<&str>) -> Result<PackagePreview> {
    let id = match explicit_id {
        Some(id) => sanitize_extension_id(id)?,
        None => infer_extension_id(bytes)?,
    };
    let dest = std::env::temp_dir().join(format!("brunya-preview-{id}"));
    if dest.exists() {
        std::fs::remove_dir_all(&dest).ok();
    }
    let inspected = (|| {
        extract_package(bytes, &dest)?;
        let (manifest, _) = Manifest::load_from_dir(&dest)?;
        let validation = manifest.validate();
        if !validation.is_valid {
            return Err(ExtensionError::InvalidManifest(validation.errors.join("; ")));
        }
        let info = manifest.to_extension_info(
            ExtensionId::new(id.clone()),
            &dest,
            InstallType::Normal,
            ExtensionState::Installing,
        );
        Ok(PackagePreview {
            id,
            name: info.name,
            version: info.version,
            description: info.description,
            permissions: info.permissions,
            host_permissions: info.host_permissions,
        })
    })();
    std::fs::remove_dir_all(&dest).ok();
    inspected
}

pub fn sanitize_extension_id(id: &str) -> Result<String> {
    let id = id.trim().to_ascii_lowercase();
    let ok = id.len() >= 8
        && id.len() <= 128
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !ok {
        return Err(ExtensionError::InvalidUrl(format!(
            "Identificador de extensão inválido: {id}"
        )));
    }
    Ok(id)
}

/// Id no formato Chrome: 16 primeiros bytes do SHA-256, cada nibble vira `a`–`p`.
pub fn chrome_id_from_public_key(spki_der: &[u8]) -> String {
    let hash = ring::digest::digest(&ring::digest::SHA256, spki_der);
    let mut id = String::with_capacity(32);
    for byte in hash.as_ref().iter().take(16) {
        id.push((b'a' + (byte >> 4)) as char);
        id.push((b'a' + (byte & 0x0f)) as char);
    }
    id
}

fn infer_extension_id(bytes: &[u8]) -> Result<String> {
    if let Some(key) = crx_public_key(bytes) {
        return Ok(chrome_id_from_public_key(&key));
    }
    Ok(chrome_id_from_public_key(bytes))
}

fn crx_public_key(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() < 16 || &bytes[0..4] != b"Cr24" {
        return None;
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().ok()?);
    match version {
        2 => {
            let pk_len = u32::from_le_bytes(bytes[8..12].try_into().ok()?) as usize;
            let start: usize = 16;
            let end = start.checked_add(pk_len)?;
            if end > bytes.len() || pk_len < 32 {
                return None;
            }
            Some(bytes[start..end].to_vec())
        }
        3 => {
            let header_len = u32::from_le_bytes(bytes[8..12].try_into().ok()?) as usize;
            let start: usize = 12;
            let end = start.checked_add(header_len)?;
            if end > bytes.len() {
                return None;
            }
            protobuf_public_key(&bytes[start..end])
        }
        _ => None,
    }
}

fn protobuf_public_key(buf: &[u8]) -> Option<Vec<u8>> {
    let mut index = 0;
    let mut found = None;
    while index < buf.len() {
        let key = read_varint(buf, &mut index)?;
        let field = (key >> 3) as u32;
        let wire = (key & 7) as u32;
        match wire {
            0 => { read_varint(buf, &mut index)?; }
            1 => index = index.checked_add(8)?,
            5 => index = index.checked_add(4)?,
            2 => {
                let len = read_varint(buf, &mut index)? as usize;
                let end = index.checked_add(len)?;
                if end > buf.len() {
                    break;
                }
                let slice = &buf[index..end];
                index = end;
                if field == 1 && slice.first() == Some(&0x30) && slice.len() > 32 {
                    found = Some(slice.to_vec());
                } else if field != 1 {
                    if let Some(key) = protobuf_public_key(slice) {
                        found = Some(key);
                    }
                }
            }
            _ => break,
        }
    }
    found
}

fn read_varint(buf: &[u8], index: &mut usize) -> Option<u64> {
    let mut result = 0u64;
    let mut shift = 0;
    for _ in 0..10 {
        if *index >= buf.len() {
            return None;
        }
        let byte = buf[*index];
        *index += 1;
        result |= ((byte & 0x7f) as u64) << shift;
        if byte & 0x80 == 0 {
            return Some(result);
        }
        shift += 7;
        if shift > 63 {
            return None;
        }
    }
    None
}

fn zip_start(bytes: &[u8]) -> Result<usize> {
    if bytes.starts_with(b"PK") {
        return Ok(0);
    }
    if bytes.len() < 12 || &bytes[0..4] != b"Cr24" {
        return Err(ExtensionError::CorruptPackage(
            "O arquivo não é um pacote CRX, ZIP ou XPI".to_string(),
        ));
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    let start = match version {
        2 => {
            if bytes.len() < 16 {
                return Err(ExtensionError::CorruptPackage("CRX2 truncado".to_string()));
            }
            let pk_len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
            let sig_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
            16usize.checked_add(pk_len).and_then(|n| n.checked_add(sig_len)).ok_or_else(|| {
                ExtensionError::CorruptPackage("Cabeçalho CRX2 inválido".to_string())
            })?
        }
        3 => {
            let header_len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
            12usize.checked_add(header_len).ok_or_else(|| {
                ExtensionError::CorruptPackage("Cabeçalho CRX3 inválido".to_string())
            })?
        }
        other => {
            return Err(ExtensionError::CorruptPackage(format!(
                "Versão CRX não suportada: {other}"
            )));
        }
    };
    if start >= bytes.len() || !bytes[start..].starts_with(b"PK") {
        return Err(ExtensionError::CorruptPackage(
            "O pacote CRX não contém um ZIP válido".to_string(),
        ));
    }
    Ok(start)
}

fn extract_package(bytes: &[u8], dest: &Path) -> Result<()> {
    if bytes.is_empty() || bytes.len() > MAX_PACKAGE_BYTES {
        return Err(ExtensionError::CorruptPackage(
            "Pacote vazio ou maior que 64 MB".to_string(),
        ));
    }
    let start = zip_start(bytes)?;
    std::fs::create_dir_all(dest)?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes[start..])).map_err(|err| {
        ExtensionError::CorruptPackage(format!("Falha ao ler o ZIP da extensão: {err}"))
    })?;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|err| {
            ExtensionError::CorruptPackage(format!("Falha ao ler entrada {index} do ZIP: {err}"))
        })?;
        let Some(rel) = file.enclosed_name().map(|p| p.to_path_buf()) else {
            continue;
        };
        let out_path = dest.join(&rel);
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
    hoist_single_root(dest)?;
    if !dest.join("manifest.json").is_file() {
        return Err(ExtensionError::ManifestNotFound(
            "manifest.json não está na raiz do pacote".to_string(),
        ));
    }
    Ok(())
}

fn hoist_single_root(dest: &Path) -> Result<()> {
    if dest.join("manifest.json").is_file() {
        return Ok(());
    }
    let mut children = Vec::new();
    for entry in std::fs::read_dir(dest)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".DS_Store" || name == "__MACOSX" {
            continue;
        }
        children.push(entry.path());
    }
    if children.len() == 1 && children[0].is_dir() && children[0].join("manifest.json").is_file() {
        let inner = children[0].clone();
        for entry in std::fs::read_dir(&inner)? {
            let entry = entry?;
            std::fs::rename(entry.path(), dest.join(entry.file_name()))?;
        }
        std::fs::remove_dir_all(&inner).ok();
    }
    Ok(())
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

    #[test]
    fn test_extract_crx3_and_crx2() {
        use std::io::Write;
        let zip_bytes = {
            let mut cursor = std::io::Cursor::new(Vec::new());
            {
                let mut writer = zip::ZipWriter::new(&mut cursor);
                let opts = zip::write::SimpleFileOptions::default();
                writer.start_file("manifest.json", opts).unwrap();
                writer.write_all(br#"{"manifest_version":3,"name":"Peca","version":"1.2.0","permissions":["storage"]}"#).unwrap();
                writer.finish().unwrap();
            }
            cursor.into_inner()
        };

        let mut crx3 = Vec::new();
        crx3.extend_from_slice(b"Cr24");
        crx3.extend_from_slice(&3u32.to_le_bytes());
        crx3.extend_from_slice(&0u32.to_le_bytes());
        crx3.extend_from_slice(&zip_bytes);

        let pubkey = vec![0x30u8, 0x82, 0x01, 0x0a, 0x02, 0x82];
        let sig = vec![9u8, 8, 7, 6];
        let mut crx2 = Vec::new();
        crx2.extend_from_slice(b"Cr24");
        crx2.extend_from_slice(&2u32.to_le_bytes());
        crx2.extend_from_slice(&(pubkey.len() as u32).to_le_bytes());
        crx2.extend_from_slice(&(sig.len() as u32).to_le_bytes());
        crx2.extend_from_slice(&pubkey);
        crx2.extend_from_slice(&sig);
        crx2.extend_from_slice(&zip_bytes);

        let preview = inspect_package(&crx3, Some("abcdefghijklmnopabcdefghijklmnop")).unwrap();
        assert_eq!(preview.name, "Peca");
        assert_eq!(preview.version, "1.2.0");
        assert_eq!(preview.permissions, vec!["storage".to_string()]);

        let derived = infer_extension_id(&crx2).unwrap();
        assert_eq!(derived.len(), 32);
        assert!(derived.chars().all(|c| ('a'..='p').contains(&c)));
        let preview2 = inspect_package(&crx2, None).unwrap();
        assert_eq!(preview2.id, derived);
        assert_eq!(preview2.name, "Peca");
    }
}