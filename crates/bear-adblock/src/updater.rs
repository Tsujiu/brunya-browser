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
    pub sha256_checksum: String,
    pub last_updated: DateTime<Utc>,
    pub rule_count: usize,
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
            sha256_checksum: computed_hash,
            last_updated: Utc::now(),
            rule_count,
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
        let temp_dir = std::env::temp_dir().join(format!("bear_filters_test_{}", Utc::now().timestamp_nanos_opt().unwrap_or(0)));
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
