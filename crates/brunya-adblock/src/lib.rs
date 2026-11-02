//! Motor integrado de bloqueio de anúncios e rastreadores do Brunya Browser.
//!
//! Fornece parsing de regras sintáticas ABP/EasyList, modos de proteção
//! (Balanceado, Rigoroso, Personalizado), rastreamento detalhado de bloqueios por aba,
//! atualização atômica de listas com SHA-256 e capacidade de rollback.

pub mod ffi;
pub mod modules;
pub mod rule;
pub mod stats;
pub mod updater;

pub use ffi::*;
pub use modules::*;
pub use rule::{ParsedRule, ResourceType, RuleKind};
pub use stats::{AdblockStats, BlockedEntry, CategorizedStatistics, TrackerCategory};
pub use updater::{FilterListManager, FilterListMetadata};

use brunya_core::traits::{NetworkInterceptor, RequestAction};
use brunya_core::types::TabId;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};
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
    profile: RwLock<ShieldProfile>,
    modules: RwLock<ShieldModulesConfig>,
    rules: RwLock<Vec<ParsedRule>>,
    whitelist_domains: RwLock<HashSet<String>>,
    total_blocked_counter: AtomicU64,
    stats: Arc<AdblockStats>,
    updater: Arc<FilterListManager>,
}

impl AdblockEngine {
    pub fn new() -> Self {
        let storage_dir = if let Ok(home) = std::env::var("HOME") {
            std::path::PathBuf::from(home).join(".local/share/brunya/filters")
        } else {
            std::path::PathBuf::from("/tmp/brunya/filters")
        };

        let updater = Arc::new(FilterListManager::new(&storage_dir));
        let stats = Arc::new(AdblockStats::new());

        let engine = Self {
            mode: RwLock::new(ProtectionMode::Balanced),
            profile: RwLock::new(ShieldProfile::Balanced),
            modules: RwLock::new(ShieldModulesConfig::default()),
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

    pub fn set_profile(&self, profile: ShieldProfile) {
        *self.profile.write() = profile;
        *self.modules.write() = ShieldModulesConfig::for_profile(profile);
        match profile {
            ShieldProfile::Essential => *self.mode.write() = ProtectionMode::Balanced,
            ShieldProfile::Balanced => *self.mode.write() = ProtectionMode::Balanced,
            ShieldProfile::Strict => *self.mode.write() = ProtectionMode::Strict,
            ShieldProfile::Custom => *self.mode.write() = ProtectionMode::Custom,
        }
    }

    pub fn profile(&self) -> ShieldProfile {
        *self.profile.read()
    }

    pub fn modules_config(&self) -> ShieldModulesConfig {
        *self.modules.read()
    }

    pub fn set_modules_config(&self, cfg: ShieldModulesConfig) {
        *self.modules.write() = cfg;
        *self.profile.write() = ShieldProfile::Custom;
    }

    pub fn reload_rules(&self) {
        let mut rules = self.rules.write();
        rules.clear();
        drop(rules);
        self.load_default_protection_list();
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

    pub fn remove_site_exception(&self, domain: &str) {
        self.whitelist_domains.write().remove(&domain.to_lowercase());
    }

    pub fn is_site_allowed(&self, domain: &str) -> bool {
        self.whitelist_domains.read().contains(&domain.to_lowercase())
    }

    pub fn add_rule(&self, rule_str: &str) {
        if let Some(rule) = ParsedRule::parse(rule_str) {
            self.rules.write().push(rule);
        }
    }

    /// Carrega as regras fundamentais de proteção do BruNya Shield (EasyList + EasyPrivacy + uBlock).
    fn load_default_protection_list(&self) {
        let mut rules = self.rules.write();

        if let Ok(rules_val) = serde_json::from_str::<serde_json::Value>(include_str!("../resources/adblock_rules.json")) {
            if let Some(arr) = rules_val.as_array() {
                for item in arr {
                    if let Some(trigger) = item.get("trigger") {
                        if let Some(pattern) = trigger.get("url-filter").and_then(|u| u.as_str()) {
                            let clean_p = pattern.trim_matches('.').trim_matches('*');
                            if !clean_p.is_empty() {
                                let rule_str = format!("||{}^", clean_p.replace("\\.", "."));
                                if let Some(parsed) = ParsedRule::parse(&rule_str) {
                                    rules.push(parsed);
                                }
                            }
                        }
                    }
                }
            }
        }

        info!("BruNya Shield carregou {} regras fundamentais de proteção", rules.len());
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

        let modules = self.modules_config();

        // 2. Módulo 03 — Threat Intelligence (Phishing / Malware)
        if modules.malicious_sites {
            let threat = ThreatIntelligence::inspect(raw_url);
            if threat.is_threat {
                self.record_threat_block(tab_id, raw_url, &threat.reason);
                return true;
            }
        }

        // 3. Módulo 07 — Anti-Crypto Mining
        if modules.anti_crypto_mining {
            if url_lower.contains("coinhive")
                || url_lower.contains("cryptonight")
                || url_lower.contains("miner.monero")
            {
                self.record_block(tab_id, raw_url, "Anti-CryptoMining: Bloqueado script de mineração");
                return true;
            }
        }

        // 4. Módulo 10 — Bloqueio de Anúncios em Vídeo
        if modules.video_ads {
            if url_lower.contains("googleads.g.doubleclick.net/pagead/ads?client=ca-video")
                || url_lower.contains("imasdk.googleapis.com")
                || url_lower.contains("video-ad-tracker")
            {
                self.record_block(tab_id, raw_url, "VideoAds: Bloqueado anúncio em stream de vídeo");
                return true;
            }
        }

        let rules = self.rules.read();

        // 5. Verificar regras de exceção primeiro (@@)
        for rule in rules.iter() {
            if rule.kind == RuleKind::Whitelist && rule.matches(&url_lower, &host, source_domain, resource_type) {
                return false;
            }
        }

        // 6. Modo Rigoroso: bloqueia frames de terceiros e rastreadores adicionais
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

        // 7. Módulo 01 & Módulo 02 — Regras de Rede de Anúncios e Rastreadores
        if modules.adblock || modules.anti_tracking {
            for rule in rules.iter() {
                if rule.kind == RuleKind::Network && rule.matches(&url_lower, &host, source_domain, resource_type) {
                    self.record_block(tab_id, raw_url, &rule.raw);
                    return true;
                }
            }
        }

        false
    }

    fn record_block(&self, tab_id: TabId, url: &str, rule: &str) {
        self.total_blocked_counter.fetch_add(1, Ordering::Relaxed);
        self.stats.record_block(tab_id, url, rule);
        debug!("BruNya Shield bloqueou requisição: {} (regra: {})", url, rule);
    }

    fn record_threat_block(&self, tab_id: TabId, url: &str, reason: &str) {
        self.total_blocked_counter.fetch_add(1, Ordering::Relaxed);
        self.stats.record_threat_detected();
        self.stats.record_block(tab_id, url, reason);
        warn!("BruNya Shield Threat Intelligence bloqueou ameaça: {} ({})", url, reason);
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
                reason: "Bloqueado pelo BruNya Shield (Regra de Proteção Ativa)".to_string(),
            }
        } else {
            RequestAction::Allow
        }
    }
}
