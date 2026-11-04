//! Gerenciamento de segurança do sistema de extensões.
//!
//! Validação, isolamento e políticas de segurança para extensões.

use crate::types::ExtensionId;
use parking_lot::RwLock;
use std::collections::HashSet;
use tracing::warn;

/// Políticas de segurança do sistema de extensões.
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Bloquear execução de código remoto (CSP).
    pub block_remote_code: bool,
    /// Bloquear eval() e setTimeout de string.
    pub block_eval: bool,
    /// Exigir HTTPS para páginas de extensão.
    pub enforce_https: bool,
    /// Bloquear extensões com permissões excessivas automaticamente.
    pub block_excessive_permissions: bool,
    /// Permitir apenas Manifest V3.
    pub manifest_v3_only: bool,
    /// Máximo de permissões host permitidas.
    pub max_host_permissions: usize,
    /// Registrar todas as ações de segurança.
    pub audit_logging: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            block_remote_code: true,
            block_eval: true,
            enforce_https: true,
            block_excessive_permissions: false,
            manifest_v3_only: false,
            max_host_permissions: 50,
            audit_logging: true,
        }
    }
}

/// Gerenciador de segurança.
pub struct SecurityManager {
    policy: RwLock<SecurityPolicy>,
    blocked_extensions: RwLock<HashSet<String>>,
    trusted_origins: RwLock<HashSet<String>>,
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {
            policy: RwLock::new(SecurityPolicy::default()),
            blocked_extensions: RwLock::new(HashSet::new()),
            trusted_origins: RwLock::new(HashSet::new()),
        }
    }

    /// Obtém a política atual.
    pub fn policy(&self) -> SecurityPolicy {
        self.policy.read().clone()
    }

    /// Atualiza a política de segurança.
    pub fn set_policy(&self, policy: SecurityPolicy) {
        *self.policy.write() = policy;
    }

    /// Verifica se uma extensão pode acessar uma URL.
    pub fn check_url_access(&self, ext_id: &ExtensionId, url: &str) -> bool {
        // URLs internas sempre protegidas
        if url.starts_with("brunya://") || url.starts_with("about:") {
            return false;
        }

        // Verificar se extensão não está bloqueada
        if self.blocked_extensions.read().contains(&ext_id.0) {
            return false;
        }

        true
    }

    /// Verifica se um script é de código remoto (bloqueado por política).
    pub fn is_remote_code(&self, url: &str) -> bool {
        let policy = self.policy.read();
        if !policy.block_remote_code {
            return false;
        }

        // Código remoto = qualquer URL que não seja local à extensão
        !(url.starts_with("chrome-extension://")
            || url.starts_with("moz-extension://")
            || url.starts_with("file://")
            || url.contains("brunya-extension"))
    }

    /// Verifica se eval é permitido.
    pub fn eval_allowed(&self) -> bool {
        !self.policy.read().block_eval
    }

    /// Bloqueia uma extensão por comportamento suspeito.
    pub fn block_extension(&self, ext_id: &ExtensionId, reason: &str) {
        self.blocked_extensions.write().insert(ext_id.0.clone());
        if self.policy.read().audit_logging {
            warn!(
                "🔒 Segurança: Extensão bloqueada {} — Motivo: {}",
                ext_id, reason
            );
        }
    }

    /// Desbloqueia uma extensão.
    pub fn unblock_extension(&self, ext_id: &ExtensionId) {
        self.blocked_extensions.write().remove(&ext_id.0);
    }

    /// Verifica se uma extensão está bloqueada.
    pub fn is_blocked(&self, ext_id: &ExtensionId) -> bool {
        self.blocked_extensions.read().contains(&ext_id.0)
    }

    /// Valida a integridade de um arquivo de extensão.
    pub fn verify_file_integrity(&self, data: &[u8], expected_hash: Option<&str>) -> bool {
        if let Some(hash) = expected_hash {
            let actual_hash = ring::digest::digest(&ring::digest::SHA256, data);
            let actual_hex = hex::encode(actual_hash.as_ref());
            return actual_hex == hash;
        }
        true // Sem hash esperado, assume válido
    }

    /// Adiciona origem confiável.
    pub fn add_trusted_origin(&self, origin: String) {
        self.trusted_origins.write().insert(origin);
    }

    /// Verifica se uma origem é confiável.
    pub fn is_trusted_origin(&self, origin: &str) -> bool {
        self.trusted_origins.read().contains(origin)
    }

    /// Gera um hash SHA-256 de dados para verificação.
    pub fn hash_data(data: &[u8]) -> String {
        let digest = ring::digest::digest(&ring::digest::SHA256, data);
        hex::encode(digest.as_ref())
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Utilitário hex encoding (evita dependência se não disponível).
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_url_access() {
        let sec = SecurityManager::new();
        let ext_id = ExtensionId("test".to_string());

        assert!(sec.check_url_access(&ext_id, "https://example.com"));
        assert!(!sec.check_url_access(&ext_id, "brunya://settings"));
        assert!(!sec.check_url_access(&ext_id, "about:blank"));
    }

    #[test]
    fn test_block_extension() {
        let sec = SecurityManager::new();
        let ext_id = ExtensionId("bad-ext".to_string());

        assert!(!sec.is_blocked(&ext_id));
        sec.block_extension(&ext_id, "Comportamento malicioso detectado");
        assert!(sec.is_blocked(&ext_id));
    }
}
