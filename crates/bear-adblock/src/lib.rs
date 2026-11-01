//! Motor integrado de bloqueio de anúncios e rastreadores do BEAR Browser.
//!
//! Intercepta requisições de rede antes do tráfego de dados,
//! com suporte a listas de filtros, exceções e estatísticas de proteção.

use bear_core::traits::{NetworkInterceptor, RequestAction};
use bear_core::types::TabId;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, info};
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProtectionMode {
    #[default]
    Balanced,
    Strict,
    Custom,
    Disabled,
}

#[derive(Debug, Clone)]
pub struct FilterRule {
    pub pattern: String,
    pub is_whitelist: bool,
    pub domains: Option<HashSet<String>>,
}

pub struct AdblockEngine {
    mode: RwLock<ProtectionMode>,
    blocked_patterns: RwLock<Vec<String>>,
    whitelist_domains: RwLock<HashSet<String>>,
    total_blocked_counter: AtomicU64,
}

impl AdblockEngine {
    pub fn new() -> Self {
        let engine = Self {
            mode: RwLock::new(ProtectionMode::Balanced),
            blocked_patterns: RwLock::new(Vec::new()),
            whitelist_domains: RwLock::new(HashSet::new()),
            total_blocked_counter: AtomicU64::new(0),
        };
        engine.load_default_protection_list();
        engine
    }

    pub fn set_mode(&self, mode: ProtectionMode) {
        *self.mode.write() = mode;
    }

    pub fn mode(&self) -> ProtectionMode {
        *self.mode.read()
    }

    pub fn total_blocked(&self) -> u64 {
        self.total_blocked_counter.load(Ordering::Relaxed)
    }

    pub fn allow_site(&self, domain: &str) {
        self.whitelist_domains.write().insert(domain.to_lowercase());
    }

    pub fn is_site_allowed(&self, domain: &str) -> bool {
        self.whitelist_domains.read().contains(&domain.to_lowercase())
    }

    /// Carrega lista de regras embutida de alta eficiência (telemetria invasiva, ad networks globais).
    fn load_default_protection_list(&self) {
        let default_rules = [
            "doubleclick.net",
            "google-analytics.com",
            "googletagmanager.com",
            "adnxs.com",
            "criteo.com",
            "scorecardresearch.com",
            "taboola.com",
            "outbrain.com",
            "facebook.net/signals",
            "connect.facebook.net/en_US/fbevents.js",
            "analytics.tiktok.com",
            "hotjar.com",
            "clarity.ms",
            "telemetry",
            "/pagead/",
            "/ads.js",
            "/adserver/",
            "/pixel.gif",
            "adservice.google.",
        ];

        let mut patterns = self.blocked_patterns.write();
        for rule in default_rules {
            patterns.push(rule.to_string());
        }
        info!("BEAR Adblock carregou {} regras fundamentais de privacidade", patterns.len());
    }

    pub fn add_rule(&self, rule: &str) {
        let trimmed = rule.trim();
        if trimmed.is_empty() || trimmed.starts_with('!') || trimmed.starts_with('#') {
            return; // Comentários
        }

        if let Some(exception) = trimmed.strip_prefix("@@") {
            self.allow_site(exception);
        } else {
            self.blocked_patterns.write().push(trimmed.to_string());
        }
    }

    pub fn should_block_url(&self, raw_url: &str) -> bool {
        let mode = self.mode();
        if mode == ProtectionMode::Disabled {
            return false;
        }

        let parsed_url = match Url::parse(raw_url) {
            Ok(u) => u,
            Err(_) => return false,
        };

        let host = parsed_url.host_str().unwrap_or("").to_lowercase();

        // 1. Verificar lista de permissões
        if self.is_site_allowed(&host) {
            return false;
        }

        let patterns = self.blocked_patterns.read();
        for pattern in patterns.iter() {
            if raw_url.contains(pattern) || host.contains(pattern) {
                self.total_blocked_counter.fetch_add(1, Ordering::Relaxed);
                debug!("BEAR Adblock bloqueou requisição: {} (regra: {})", raw_url, pattern);
                return true;
            }
        }

        false
    }
}

impl Default for AdblockEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkInterceptor for AdblockEngine {
    fn inspect_request(&self, _tab_id: TabId, url: &str, _resource_type: &str) -> RequestAction {
        if self.should_block_url(url) {
            RequestAction::Block {
                reason: "Bloqueado pelo Escudo de Privacidade BEAR".to_string(),
            }
        } else {
            RequestAction::Allow
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adblock_filtering() {
        let engine = AdblockEngine::new();

        assert!(engine.should_block_url("https://securepubads.g.doubleclick.net/gampad/ads"));
        assert!(engine.should_block_url("https://www.google-analytics.com/analytics.js"));
        assert!(!engine.should_block_url("https://kernel.org/pub/linux/kernel/"));
        assert!(!engine.should_block_url("https://docs.rs/gtk4/latest/"));

        // Testar exceção de site
        engine.allow_site("google-analytics.com");
        assert!(!engine.should_block_url("https://google-analytics.com/analytics.js"));

        assert!(engine.total_blocked() >= 2);
    }
}
