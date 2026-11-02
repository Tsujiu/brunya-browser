//! Caminhos relativos de pacotes de extensão, sem sair do diretório base.

use crate::errors::{ExtensionError, Result};
use std::path::{Component, Path, PathBuf};

/// Junta um caminho relativo ao diretório da extensão.
/// Recusa caminhos absolutos e `..`.
pub fn resolve_extension_file(base: &Path, relative: &str) -> Result<PathBuf> {
    let relative = relative.trim().trim_start_matches(['/', '\\']);
    if relative.is_empty() {
        return Err(ExtensionError::InvalidManifest(
            "Caminho vazio dentro da extensão".to_string(),
        ));
    }
    let rel = Path::new(relative);
    if rel.is_absolute()
        || rel.components().any(|c| matches!(c, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(ExtensionError::InvalidManifest(format!(
            "Caminho de extensão recusado: {relative}"
        )));
    }
    Ok(base.join(rel))
}

pub fn read_extension_file(base: &Path, relative: &str) -> Result<Vec<u8>> {
    let path = resolve_extension_file(base, relative)?;
    if !path.is_file() {
        return Err(ExtensionError::NotFound(path.display().to_string()));
    }
    // Confere de novo depois de normalizar links, quando o diretório existe.
    if let (Ok(base_c), Ok(path_c)) = (base.canonicalize(), path.canonicalize()) {
        if !path_c.starts_with(&base_c) {
            return Err(ExtensionError::InvalidManifest(format!(
                "Caminho de extensão recusado: {relative}"
            )));
        }
    }
    Ok(std::fs::read(path)?)
}

pub fn read_extension_text(base: &Path, relative: &str) -> Result<String> {
    let bytes = read_extension_file(base, relative)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
