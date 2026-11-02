use chrono::{DateTime, Utc};
use ring::digest::{digest, SHA256};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterListMetadata {
    pub id: String,
    pub name: String,
    pub source_url: String,
    pub maintainer: String,
    pub license: String,
    pub purpose: String,
    pub sha256_checksum: String,
    pub last_updated: DateTime<Utc>,
    pub rule_count: usize,
    pub errors_count: usize,
    pub is_enabled: bool,
}

pub struct FilterListManager {
    storage_dir: PathBuf,
}

impl FilterListManager {
    pub fn new<P: AsRef<Path>>(storage_dir: P) -> Self {
        let path = storage_dir.as_ref().to_path_buf();
        fs::create_dir_all(&path).ok();
        Self { storage_dir: path }
    }

    pub fn compute_sha256(content: &[u8]) -> String {
        let d = digest(&SHA256, content);
        d.as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    }

    /// Retorna as listas de filtros oficiais pré-configuradas no BruNya Shield
    pub fn default_catalogue() -> Vec<FilterListMetadata> {
        vec![
            FilterListMetadata {
                id: "easylist".to_string(),
                name: "EasyList".to_string(),
                source_url: "https://easylist.to/easylist/easylist.txt".to_string(),
                maintainer: "EasyList Community".to_string(),
                license: "GPLv3 / CC BY-SA 3.0".to_string(),
                purpose: "Bloqueio primário de publicidade e banners".to_string(),
                sha256_checksum: String::new(),
                last_updated: Utc::now(),
                rule_count: 42000,
                errors_count: 0,
                is_enabled: true,
            },
            FilterListMetadata {
                id: "easyprivacy".to_string(),
                name: "EasyPrivacy".to_string(),
                source_url: "https://easylist.to/easylist/easyprivacy.txt".to_string(),
                maintainer: "EasyList Community".to_string(),
                license: "GPLv3 / CC BY-SA 3.0".to_string(),
                purpose: "Proteção contra rastreadores, beacons e analytics".to_string(),
                sha256_checksum: String::new(),
                last_updated: Utc::now(),
                rule_count: 28000,
                errors_count: 0,
                is_enabled: true,
            },
            FilterListMetadata {
                id: "ublock-badware".to_string(),
                name: "uBlock Filters — Badware Risks".to_string(),
                source_url: "https://raw.githubusercontent.com/gorhill/uBlock/master/filters/badware.txt".to_string(),
                maintainer: "uBlock Origin Team".to_string(),
                license: "GPLv3".to_string(),
                purpose: "Proteção contra domínios fraudulentos e malware".to_string(),
                sha256_checksum: String::new(),
                last_updated: Utc::now(),
                rule_count: 8500,
                errors_count: 0,
                is_enabled: true,
            },
            FilterListMetadata {
                id: "easylist-annoyances".to_string(),
                name: "EasyList Annoyances".to_string(),
                source_url: "https://easylist.to/easylist/fanboy-annoyance.txt".to_string(),
                maintainer: "Fanboy & EasyList".to_string(),
                license: "GPLv3".to_string(),
                purpose: "Remoção de avisos de cookies forçados e overlays".to_string(),
                sha256_checksum: String::new(),
                last_updated: Utc::now(),
                rule_count: 14000,
                errors_count: 0,
                is_enabled: true,
            },
            FilterListMetadata {
                id: "brazilian-regional".to_string(),
                name: "Lista Regional Brasileira".to_string(),
                source_url: "https://raw.githubusercontent.com/easylist/easylist/master/easylist_regional/brazil.txt".to_string(),
                maintainer: "Comunidade Brasileira de Filtros".to_string(),
                license: "CC BY-SA 3.0".to_string(),
                purpose: "Regras específicas para portais e mídia do Brasil".to_string(),
                sha256_checksum: String::new(),
                last_updated: Utc::now(),
                rule_count: 6200,
                errors_count: 0,
                is_enabled: true,
            },
        ]
    }

    /// Salva uma lista de filtros atomicamente com verificação de integridade e rollback de segurança.
    pub fn save_list_atomic(
        &self,
        list_id: &str,
        content: &str,
        expected_sha256: Option<&str>,
    ) -> Result<FilterListMetadata, String> {
        let final_path = self.storage_dir.join(format!("{}.txt", list_id));
        let backup_path = self.storage_dir.join(format!("{}.txt.bak", list_id));
        let temp_path = self.storage_dir.join(format!("{}.txt.tmp", list_id));

        let computed_hash = Self::compute_sha256(content.as_bytes());

        // Verificar integridade se hash esperado foi fornecido
        if let Some(expected) = expected_sha256 {
            if computed_hash.to_lowercase() != expected.to_lowercase() {
                return Err(format!(
                    "Violação de integridade de lista de filtros! Esperado: {}, Obtido: {}",
                    expected, computed_hash
                ));
            }
        }

        // Escrever em arquivo temporário
        {
            let mut file = File::create(&temp_path).map_err(|e| e.to_string())?;
            file.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
        }

        // Fazer backup do arquivo existente para possibilidade de rollback
        if final_path.exists() {
            fs::copy(&final_path, &backup_path).ok();
        }

        // Troca atômica (rename)
        fs::rename(&temp_path, &final_path).map_err(|e| e.to_string())?;

        let rule_count = content.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('!')).count();

        let meta = FilterListMetadata {
            id: list_id.to_string(),
            name: list_id.to_string(),
            source_url: String::new(),
            maintainer: "BruNya Filter Service".to_string(),
            license: "Comunitária".to_string(),
            purpose: "Filtragem nativa".to_string(),
            sha256_checksum: computed_hash,
            last_updated: Utc::now(),
            rule_count,
            errors_count: 0,
            is_enabled: true,
        };

        info!("Lista de filtros '{}' atualizada e verificada com sucesso ({} regras)", list_id, rule_count);
        Ok(meta)
    }

    /// Executa rollback para a versão anterior em caso de corrupção ou erro grave de parsing.
    pub fn rollback(&self, list_id: &str) -> Result<bool, String> {
        let final_path = self.storage_dir.join(format!("{}.txt", list_id));
        let backup_path = self.storage_dir.join(format!("{}.txt.bak", list_id));

        if backup_path.exists() {
            warn!("Executando rollback de segurança para lista de filtros '{}'", list_id);
            fs::copy(&backup_path, &final_path).map_err(|e| e.to_string())?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn load_list(&self, list_id: &str) -> Option<String> {
        let final_path = self.storage_dir.join(format!("{}.txt", list_id));
        fs::read_to_string(final_path).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_filter_update_and_rollback() {
        let temp_dir = std::env::temp_dir().join(format!("brunya_filters_test_{}", Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        let mgr = FilterListManager::new(&temp_dir);

        let initial_content = "||adservice.google.com^\n||tracker.com^\n";
        let meta1 = mgr.save_list_atomic("testlist", initial_content, None).unwrap();
        assert_eq!(meta1.rule_count, 2);

        // Nova versão
        let updated_content = "||adservice.google.com^\n||tracker.com^\n||evil-spyware.org^\n";
        let meta2 = mgr.save_list_atomic("testlist", updated_content, None).unwrap();
        assert_eq!(meta2.rule_count, 3);

        // Testar rollback
        let did_rollback = mgr.rollback("testlist").unwrap();
        assert!(did_rollback);

        let rolled_back_content = mgr.load_list("testlist").unwrap();
        assert_eq!(rolled_back_content, initial_content);

        // Limpeza
        fs::remove_dir_all(&temp_dir).ok();
    }
}
