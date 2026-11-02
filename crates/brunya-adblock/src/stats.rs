use brunya_core::types::TabId;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackerCategory {
    Advertisement,
    AnalyticsAndTracking,
    SocialWidget,
    Fingerprinting,
    Cryptomining,
    Unknown,
}

impl TrackerCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            TrackerCategory::Advertisement => "Anúncio Publicitário",
            TrackerCategory::AnalyticsAndTracking => "Rastreador Analítico",
            TrackerCategory::SocialWidget => "Rastreador Social",
            TrackerCategory::Fingerprinting => "Tentativa de Fingerprinting",
            TrackerCategory::Cryptomining => "Minerador de Criptomoeda",
            TrackerCategory::Unknown => "Conteúdo Bloqueado",
        }
    }

    pub fn infer_from_url(url: &str) -> Self {
        let u = url.to_lowercase();
        if u.contains("analytics") || u.contains("telemetry") || u.contains("stats") || u.contains("metric") {
            TrackerCategory::AnalyticsAndTracking
        } else if u.contains("facebook") || u.contains("tiktok") || u.contains("twitter") || u.contains("linkedin") {
            TrackerCategory::SocialWidget
        } else if u.contains("coin") || u.contains("miner") || u.contains("crypto") {
            TrackerCategory::Cryptomining
        } else if u.contains("fingerprint") || u.contains("canvas") || u.contains("audio-fingerprint") {
            TrackerCategory::Fingerprinting
        } else {
            TrackerCategory::Advertisement
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedEntry {
    pub url: String,
    pub domain: String,
    pub rule: String,
    pub category: TrackerCategory,
    pub timestamp: DateTime<Utc>,
}

pub struct AdblockStats {
    per_tab_entries: RwLock<HashMap<TabId, Vec<BlockedEntry>>>,
}

impl AdblockStats {
    pub fn new() -> Self {
        Self {
            per_tab_entries: RwLock::new(HashMap::new()),
        }
    }

    pub fn record_block(&self, tab_id: TabId, url: &str, rule: &str) {
        let parsed_domain = url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.to_string()))
            .unwrap_or_else(|| "desconhecido".to_string());

        let category = TrackerCategory::infer_from_url(url);

        let entry = BlockedEntry {
            url: url.to_string(),
            domain: parsed_domain,
            rule: rule.to_string(),
            category,
            timestamp: Utc::now(),
        };

        let mut map = self.per_tab_entries.write();
        map.entry(tab_id).or_default().push(entry);
    }

    pub fn get_tab_entries(&self, tab_id: TabId) -> Vec<BlockedEntry> {
        self.per_tab_entries
            .read()
            .get(&tab_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn clear_tab(&self, tab_id: TabId) {
        self.per_tab_entries.write().remove(&tab_id);
    }
}

impl Default for AdblockStats {
    fn default() -> Self {
        Self::new()
    }
}
