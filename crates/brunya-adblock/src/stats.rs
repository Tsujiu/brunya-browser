use brunya_core::types::TabId;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackerCategory {
    Advertisement,
    AnalyticsAndTracking,
    SocialWidget,
    Fingerprinting,
    Cryptomining,
    MaliciousThreat,
    Annoyance,
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
            TrackerCategory::MaliciousThreat => "Ameaça Maliciosa",
            TrackerCategory::Annoyance => "Incômodo / Overlay",
            TrackerCategory::Unknown => "Conteúdo Bloqueado",
        }
    }

    pub fn infer_from_url(url: &str) -> Self {
        let u = url.to_lowercase();
        if u.contains("analytics")
            || u.contains("telemetry")
            || u.contains("stats")
            || u.contains("metric")
        {
            TrackerCategory::AnalyticsAndTracking
        } else if u.contains("facebook")
            || u.contains("tiktok")
            || u.contains("twitter")
            || u.contains("linkedin")
        {
            TrackerCategory::SocialWidget
        } else if u.contains("coin") || u.contains("miner") || u.contains("crypto") {
            TrackerCategory::Cryptomining
        } else if u.contains("fingerprint")
            || u.contains("canvas")
            || u.contains("audio-fingerprint")
        {
            TrackerCategory::Fingerprinting
        } else if u.contains("phishing") || u.contains("malware") || u.contains("trojan") {
            TrackerCategory::MaliciousThreat
        } else if u.contains("cookie-notice") || u.contains("newsletter-popup") {
            TrackerCategory::Annoyance
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

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct CategorizedStatistics {
    pub total_blocked: u64,
    pub ads_blocked: u64,
    pub trackers_blocked: u64,
    pub threats_detected: u64,
    pub annoyances_removed: u64,
    pub urls_cleaned: u64,
    pub cookies_blocked: u64,
    pub miners_blocked: u64,
}

pub struct AdblockStats {
    per_tab_entries: RwLock<HashMap<TabId, Vec<BlockedEntry>>>,
    ads_counter: AtomicU64,
    trackers_counter: AtomicU64,
    threats_counter: AtomicU64,
    annoyances_counter: AtomicU64,
    urls_cleaned_counter: AtomicU64,
    cookies_counter: AtomicU64,
    miners_counter: AtomicU64,
}

impl AdblockStats {
    pub fn new() -> Self {
        Self {
            per_tab_entries: RwLock::new(HashMap::new()),
            ads_counter: AtomicU64::new(0),
            trackers_counter: AtomicU64::new(0),
            threats_counter: AtomicU64::new(0),
            annoyances_counter: AtomicU64::new(0),
            urls_cleaned_counter: AtomicU64::new(0),
            cookies_counter: AtomicU64::new(0),
            miners_counter: AtomicU64::new(0),
        }
    }

    pub fn record_block(&self, tab_id: TabId, url: &str, rule: &str) {
        let parsed_domain = url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.to_string()))
            .unwrap_or_else(|| "desconhecido".to_string());

        let category = TrackerCategory::infer_from_url(url);

        match category {
            TrackerCategory::Advertisement => {
                self.ads_counter.fetch_add(1, Ordering::Relaxed);
            }
            TrackerCategory::AnalyticsAndTracking
            | TrackerCategory::SocialWidget
            | TrackerCategory::Fingerprinting => {
                self.trackers_counter.fetch_add(1, Ordering::Relaxed);
            }
            TrackerCategory::Cryptomining => {
                self.miners_counter.fetch_add(1, Ordering::Relaxed);
            }
            TrackerCategory::MaliciousThreat => {
                self.threats_counter.fetch_add(1, Ordering::Relaxed);
            }
            TrackerCategory::Annoyance => {
                self.annoyances_counter.fetch_add(1, Ordering::Relaxed);
            }
            TrackerCategory::Unknown => {
                self.ads_counter.fetch_add(1, Ordering::Relaxed);
            }
        }

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

    pub fn record_url_cleaned(&self, count: u32) {
        self.urls_cleaned_counter
            .fetch_add(count as u64, Ordering::Relaxed);
    }

    pub fn record_threat_detected(&self) {
        self.threats_counter.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cookie_blocked(&self) {
        self.cookies_counter.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_tab_entries(&self, tab_id: TabId) -> Vec<BlockedEntry> {
        self.per_tab_entries
            .read()
            .get(&tab_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_statistics(&self) -> CategorizedStatistics {
        let ads = self.ads_counter.load(Ordering::Relaxed);
        let trackers = self.trackers_counter.load(Ordering::Relaxed);
        let threats = self.threats_counter.load(Ordering::Relaxed);
        let annoyances = self.annoyances_counter.load(Ordering::Relaxed);
        let urls = self.urls_cleaned_counter.load(Ordering::Relaxed);
        let cookies = self.cookies_counter.load(Ordering::Relaxed);
        let miners = self.miners_counter.load(Ordering::Relaxed);

        let total = ads + trackers + threats + annoyances + urls + cookies + miners;

        CategorizedStatistics {
            total_blocked: total,
            ads_blocked: ads,
            trackers_blocked: trackers,
            threats_detected: threats,
            annoyances_removed: annoyances,
            urls_cleaned: urls,
            cookies_blocked: cookies,
            miners_blocked: miners,
        }
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
