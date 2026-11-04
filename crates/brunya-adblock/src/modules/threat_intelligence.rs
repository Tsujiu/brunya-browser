//! Módulo 03 — Threat Intelligence (Proteção contra Sites Maliciosos, Phishing e Malware)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreatCategory {
    Phishing,
    Malware,
    FraudulentDomain,
    Cryptojacking,
}

impl ThreatCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            ThreatCategory::Phishing => "Tentativa de Phishing (Roubo de Dados)",
            ThreatCategory::Malware => "Distribuição de Malware",
            ThreatCategory::FraudulentDomain => "Domínio Fraudulento / Golpe",
            ThreatCategory::Cryptojacking => "Mineração Não Autorizada de Criptomoeda",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatDecision {
    pub is_threat: bool,
    pub category: Option<ThreatCategory>,
    pub reason: String,
}

pub struct ThreatIntelligence;

impl ThreatIntelligence {
    const KNOWN_THREAT_SIGNATURES: &'static [(&'static str, ThreatCategory, &'static str)] = &[
        (
            "account-verification-login",
            ThreatCategory::Phishing,
            "Padrão de phishing para credenciais bancárias",
        ),
        (
            "secure-banking-update",
            ThreatCategory::Phishing,
            "Tentativa de personificação de instituição financeira",
        ),
        (
            "crypto-airdrop-claim",
            ThreatCategory::FraudulentDomain,
            "Golpe de engenharia social em criptoativos",
        ),
        (
            "free-giftcard-generator",
            ThreatCategory::FraudulentDomain,
            "Página maliciosa de captura de dados",
        ),
        (
            "system-driver-update.exe",
            ThreatCategory::Malware,
            "Executável malicioso disfarçado de atualização",
        ),
        (
            "payload.bin",
            ThreatCategory::Malware,
            "Distribuição de binário perigoso conhecido",
        ),
        (
            "coinhive.min.js",
            ThreatCategory::Cryptojacking,
            "Script de mineração de Monero não autorizada",
        ),
        (
            "cryptonight.wasm",
            ThreatCategory::Cryptojacking,
            "Binário WebAssembly de mineração oculta",
        ),
        (
            "miner.monero",
            ThreatCategory::Cryptojacking,
            "Pool de mineração de criptomoedas",
        ),
    ];

    /// Inspeciona uma URL contra a base de dados de inteligência de ameaças do BruNya Shield
    pub fn inspect(url: &str) -> ThreatDecision {
        let lower = url.to_lowercase();

        for (pattern, cat, reason) in Self::KNOWN_THREAT_SIGNATURES {
            if lower.contains(pattern) {
                return ThreatDecision {
                    is_threat: true,
                    category: Some(*cat),
                    reason: reason.to_string(),
                };
            }
        }

        ThreatDecision {
            is_threat: false,
            category: None,
            reason: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_phishing() {
        let dec = ThreatIntelligence::inspect("https://fakebank.com/secure-banking-update/auth");
        assert!(dec.is_threat);
        assert_eq!(dec.category, Some(ThreatCategory::Phishing));
    }

    #[test]
    fn test_detect_mining() {
        let dec = ThreatIntelligence::inspect("https://cdn.example.com/scripts/coinhive.min.js");
        assert!(dec.is_threat);
        assert_eq!(dec.category, Some(ThreatCategory::Cryptojacking));
    }

    #[test]
    fn test_safe_url() {
        let dec = ThreatIntelligence::inspect("https://linuxmint.com/");
        assert!(!dec.is_threat);
    }
}
