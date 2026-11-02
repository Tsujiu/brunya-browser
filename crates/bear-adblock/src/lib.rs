//! Motor integrado de bloqueio de anúncios e rastreadores do BEAR Browser.
//!
//! Fornece parsing de regras sintáticas ABP/EasyList, modos de proteção
//! (Balanceado, Rigoroso, Personalizado), rastreamento detalhado de bloqueios por aba,
//! atualização atômica de listas com SHA-256 e capacidade de rollback.

pub mod rule;
pub mod stats;
pub mod updater;

pub use rule::{ParsedRule, ResourceType, RuleKind};
pub use stats::{AdblockStats, BlockedEntry, TrackerCategory};
pub use updater::{FilterListManager, FilterListMetadata};

use bear_core::traits::{NetworkInterceptor, RequestAction};
use bear_core::types::TabId;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, info};
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProtectionMode {
    #[default]
    Balanced, // Padrão: bloqueia anúncios, rastreadores e telemetria mantendo compatibilidade
    Strict,   // Rigoroso: bloqueio agressivo de scripts de terceiros e marketing
    Custom,   // Personalizado pelo usuário
    Disabled, // Desativado
}

pub struct AdblockEngine {
    mode: RwLock<ProtectionMode>,
    rules: RwLock<Vec<ParsedRule>>,
    whitelist_domains: RwLock<HashSet<String>>,
    total_blocked_counter: AtomicU64,
    stats: Arc<AdblockStats>,
    updater: Arc<FilterListManager>,
}

impl AdblockEngine {
    pub fn new() -> Self {
        let storage_dir = if let Ok(home) = std::env::var("HOME") {
            std::path::PathBuf::from(home).join(".local/share/bear/filters")
        } else {
            std::path::PathBuf::from("/tmp/bear/filters")
        };

        let updater = Arc::new(FilterListManager::new(&storage_dir));
        let stats = Arc::new(AdblockStats::new());

        let engine = Self {
            mode: RwLock::new(ProtectionMode::Balanced),
            rules: RwLock::new(Vec::new()),
            whitelist_domains: RwLock::new(HashSet::new()),
            total_blocked_counter: AtomicU64::new(0),
            stats,
            updater,
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

    pub fn stats(&self) -> Arc<AdblockStats> {
        self.stats.clone()
    }

    pub fn updater(&self) -> Arc<FilterListManager> {
        self.updater.clone()
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

    pub fn add_rule(&self, rule_str: &str) {
        if let Some(rule) = ParsedRule::parse(rule_str) {
            self.rules.write().push(rule);
        }
    }

    /// Carrega as regras fundamentais de privacidade do BEAR (EasyList + EasyPrivacy essenciais).
    fn load_default_protection_list(&self) {
        let default_rules = [
            "||doubleclick.net^$third-party",
            "||google-analytics.com^",
            "||googletagmanager.com^",
            "||adnxs.com^",
            "||criteo.com^",
            "||criteo.net^",
            "||scorecardresearch.com^",
            "||taboola.com^",
            "||outbrain.com^",
            "||connect.facebook.net/*/fbevents.js",
            "||analytics.tiktok.com^",
            "||hotjar.com^",
            "||clarity.ms^",
            "||telemetry.api.*^",
            "/pagead/js/adsbygoogle.js",
            "||adservice.google.*^",
            "||amazon-adsystem.com^",
            "||moatads.com^",
            "||quantserve.com^",
            "||chartbeat.com^",
        ];

        let mut rules = self.rules.write();
        for r in default_rules {
            if let Some(parsed) = ParsedRule::parse(r) {
                rules.push(parsed);
            }
        }
        info!("BEAR Adblock carregou {} regras fundamentais de privacidade", rules.len());
    }

    pub fn should_block(
        &self,
        tab_id: TabId,
        raw_url: &str,
        source_domain: Option<&str>,
        resource_type: Option<ResourceType>,
    ) -> bool {
        let mode = self.mode();
        if mode == ProtectionMode::Disabled {
            return false;
        }

        let parsed_url = match Url::parse(raw_url) {
            Ok(u) => u,
            Err(_) => return false,
        };

        let host = parsed_url.host_str().unwrap_or("").to_lowercase();
        let url_lower = raw_url.to_lowercase();

        // 1. Verificar Whitelist do site
        if self.is_site_allowed(&host) {
            return false;
        }
        if let Some(src) = source_domain {
            if self.is_site_allowed(src) {
                return false;
            }
        }

        let rules = self.rules.read();

        // 2. Verificar regras de exceção primeiro (@@)
        for rule in rules.iter() {
            if rule.kind == RuleKind::Whitelist && rule.matches(&url_lower, &host, source_domain, resource_type) {
                return false;
            }
        }

        // 3. Modo Rigoroso: bloqueia frames de terceiros e rastreadores adicionais
        if mode == ProtectionMode::Strict {
            if let Some(src) = source_domain {
                if !host.is_empty() && !ParsedRule::base_domain(src).eq_ignore_ascii_case(ParsedRule::base_domain(&host)) {
                    if let Some(ResourceType::Subdocument) = resource_type {
                        self.record_block(tab_id, raw_url, "Strict Mode: Subdocument de Terceiro Bloqueado");
                        return true;
                    }
                }
            }
        }

        // 4. Verificar regras de rede
        for rule in rules.iter() {
            if rule.kind == RuleKind::Network && rule.matches(&url_lower, &host, source_domain, resource_type) {
                self.record_block(tab_id, raw_url, &rule.raw);
                return true;
            }
        }

        false
    }

    fn record_block(&self, tab_id: TabId, url: &str, rule: &str) {
        self.total_blocked_counter.fetch_add(1, Ordering::Relaxed);
        self.stats.record_block(tab_id, url, rule);
        debug!("BEAR Adblock bloqueou requisição: {} (regra: {})", url, rule);
    }
}

impl Default for AdblockEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkInterceptor for AdblockEngine {
    fn inspect_request(&self, tab_id: TabId, url: &str, resource_type_str: &str) -> RequestAction {
        let rt = ResourceType::from_str(resource_type_str);
        if self.should_block(tab_id, url, None, rt) {
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
    fn test_adblock_engine_full() {
        let engine = AdblockEngine::new();
        let tab = TabId::new();

        // Testar bloqueio de publicidade padrão
        assert!(engine.should_block(
            tab,
            "https://securepubads.g.doubleclick.net/gampad/ads",
            Some("example.com"),
            Some(ResourceType::Script),
        ));

        // Testar estatísticas
        let entries = engine.stats().get_tab_entries(tab);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].domain, "securepubads.g.doubleclick.net");

        // Testar site não bloqueado
        assert!(!engine.should_block(
            tab,
            "https://kernel.org/pub/linux/kernel/",
            Some("kernel.org"),
            None,
        ));
    }
}
