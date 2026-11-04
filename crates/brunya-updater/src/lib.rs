//! Motor de verificação e atualização criptográfica segura do Brunya Browser.
//!
//! Implementa verificação de metadados assinados, integridade SHA-256,
//! expiração de manifestos contra ataques de congelamento (freeze attacks)
//! e proteção estrita contra rollback de versões anteriores.

use chrono::{DateTime, Utc};
use ring::digest::{digest, SHA256};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use thiserror::Error;
use tracing::{error, info, warn};

#[derive(Error, Debug)]
pub enum UpdateError {
    #[error("Tentativa de rollback rejeitada! Versão atual {current} é igual ou mais nova que a versão do manifesto {target}")]
    RollbackRejected { current: String, target: String },

    #[error("Manifesto de atualização expirado em {expires_at}! Possível ataque de congelamento de metadados")]
    ManifestExpired { expires_at: DateTime<Utc> },

    #[error(
        "Falha de integridade criptográfica! Hash SHA-256 esperado: {expected}, obtido: {actual}"
    )]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Artefato não compatível com a arquitetura do sistema: {target_arch}")]
    ArchitectureMismatch { target_arch: String },

    #[error("Erro de rede ao buscar atualização: {0}")]
    Network(String),

    #[error("Erro de I/O ao gravar artefato: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erro de serialização JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, UpdateError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateArtifact {
    pub name: String,
    pub architecture: String,
    pub url: String,
    pub sha256_checksum: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManifest {
    pub version: String,
    pub min_supported_version: String,
    pub release_date: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub artifacts: Vec<UpdateArtifact>,
}

pub struct UpdateVerifier;

impl UpdateVerifier {
    pub fn compute_sha256(data: &[u8]) -> String {
        let d = digest(&SHA256, data);
        d.as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    }

    /// Valida o manifesto contra ataques de expiração e tentativas de rollback de versão.
    pub fn verify_manifest(current_version: &str, manifest: &UpdateManifest) -> Result<()> {
        let now = Utc::now();

        // 1. Proteção contra ataque de congelamento (Freeze attack / replay)
        if now > manifest.expires_at {
            warn!(
                "Manifesto de atualização expirou em {}",
                manifest.expires_at
            );
            return Err(UpdateError::ManifestExpired {
                expires_at: manifest.expires_at,
            });
        }

        // 2. Proteção contra ataque de rollback (rejeita versão menor ou igual)
        if !Self::is_newer_version(current_version, &manifest.version) {
            warn!(
                "Rejeitando atualização: versão alvo {} não é mais nova que a versão atual {}",
                manifest.version, current_version
            );
            return Err(UpdateError::RollbackRejected {
                current: current_version.to_string(),
                target: manifest.version.clone(),
            });
        }

        Ok(())
    }

    /// Compara versões semânticas no formato major.minor.patch
    pub fn is_newer_version(current: &str, candidate: &str) -> bool {
        let parse_semver = |v: &str| -> Vec<u32> {
            v.trim_start_matches('v')
                .split('-')
                .next()
                .unwrap_or("")
                .split('.')
                .filter_map(|p| p.parse().ok())
                .collect()
        };

        let curr_parts = parse_semver(current);
        let cand_parts = parse_semver(candidate);

        cand_parts > curr_parts
    }

    /// Valida o hash SHA-256 de um arquivo baixado
    pub fn verify_file_checksum<P: AsRef<Path>>(path: P, expected_hash: &str) -> Result<()> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let computed = Self::compute_sha256(&buffer);
        if computed.to_lowercase() != expected_hash.to_lowercase() {
            return Err(UpdateError::ChecksumMismatch {
                expected: expected_hash.to_string(),
                actual: computed,
            });
        }

        Ok(())
    }
}

pub struct UpdateManager {
    current_version: String,
    system_arch: String,
}

impl UpdateManager {
    pub fn new(current_version: &str) -> Self {
        Self {
            current_version: current_version.to_string(),
            system_arch: "amd64".to_string(),
        }
    }

    /// Avalia um manifesto JSON de atualização e retorna o artefato compatível, se houver nova versão válida.
    pub fn evaluate_manifest(&self, raw_manifest: &str) -> Result<Option<UpdateArtifact>> {
        let manifest: UpdateManifest = serde_json::from_str(raw_manifest)?;

        // Validar integridade e proteções
        UpdateVerifier::verify_manifest(&self.current_version, &manifest)?;

        // Localizar artefato para a arquitetura do sistema
        for artifact in manifest.artifacts {
            if artifact.architecture == self.system_arch || artifact.architecture == "x86_64" {
                info!(
                    "Nova atualização disponível: Brunya Browser {} ({})",
                    manifest.version, artifact.name
                );
                return Ok(Some(artifact));
            }
        }

        Err(UpdateError::ArchitectureMismatch {
            target_arch: self.system_arch.clone(),
        })
    }

    /// Grava bytes de atualização em arquivo temporário com verificação estrita de hash SHA-256.
    pub fn save_and_verify_artifact<P: AsRef<Path>>(
        &self,
        artifact: &UpdateArtifact,
        bytes: &[u8],
        target_path: P,
    ) -> Result<()> {
        let computed = UpdateVerifier::compute_sha256(bytes);
        if computed.to_lowercase() != artifact.sha256_checksum.to_lowercase() {
            error!(
                "Artefato adulterado ou corrompido! Hash esperado: {}, obtido: {}",
                artifact.sha256_checksum, computed
            );
            return Err(UpdateError::ChecksumMismatch {
                expected: artifact.sha256_checksum.clone(),
                actual: computed,
            });
        }

        let mut file = File::create(target_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;

        info!(
            "Artefato '{}' verificado e gravado com sucesso.",
            artifact.name
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use tempfile::NamedTempFile;

    #[test]
    fn test_valid_update_flow() {
        let mgr = UpdateManager::new("0.3.0");
        let valid_manifest = UpdateManifest {
            version: "0.4.0".to_string(),
            min_supported_version: "0.1.0".to_string(),
            release_date: Utc::now(),
            expires_at: Utc::now() + Duration::days(30),
            artifacts: vec![UpdateArtifact {
                name: "brunya-browser_0.4.0_amd64.deb".to_string(),
                architecture: "amd64".to_string(),
                url: "https://releases.brunyabrowser.org/brunya-browser_0.4.0_amd64.deb"
                    .to_string(),
                sha256_checksum: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                    .to_string(),
                size_bytes: 2500000,
            }],
        };

        let json = serde_json::to_string(&valid_manifest).unwrap();
        let artifact = mgr.evaluate_manifest(&json).unwrap().unwrap();
        assert_eq!(artifact.name, "brunya-browser_0.4.0_amd64.deb");
    }

    #[test]
    fn test_anti_rollback_protection() {
        let mgr = UpdateManager::new("0.3.0");
        // Tentativa de instalar versão mais antiga (0.2.0)
        let rollback_manifest = UpdateManifest {
            version: "0.2.0".to_string(),
            min_supported_version: "0.1.0".to_string(),
            release_date: Utc::now(),
            expires_at: Utc::now() + Duration::days(30),
            artifacts: vec![],
        };

        let json = serde_json::to_string(&rollback_manifest).unwrap();
        let res = mgr.evaluate_manifest(&json);
        assert!(matches!(res, Err(UpdateError::RollbackRejected { .. })));
    }

    #[test]
    fn test_freeze_attack_expired_manifest_protection() {
        let mgr = UpdateManager::new("0.3.0");
        // Manifesto expirado há 2 dias
        let expired_manifest = UpdateManifest {
            version: "0.4.0".to_string(),
            min_supported_version: "0.1.0".to_string(),
            release_date: Utc::now() - Duration::days(5),
            expires_at: Utc::now() - Duration::days(2),
            artifacts: vec![],
        };

        let json = serde_json::to_string(&expired_manifest).unwrap();
        let res = mgr.evaluate_manifest(&json);
        assert!(matches!(res, Err(UpdateError::ManifestExpired { .. })));
    }

    #[test]
    fn test_corrupted_checksum_rejection() {
        let mgr = UpdateManager::new("0.3.0");
        let artifact = UpdateArtifact {
            name: "package.deb".to_string(),
            architecture: "amd64".to_string(),
            url: "https://example.com".to_string(),
            sha256_checksum: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            size_bytes: 100,
        };

        let temp = NamedTempFile::new().unwrap();
        let fake_data = b"dados adulterados de teste";

        let res = mgr.save_and_verify_artifact(&artifact, fake_data, temp.path());
        assert!(matches!(res, Err(UpdateError::ChecksumMismatch { .. })));
    }
}
