//! Perfis de Proteção e Configuração dos 10 Módulos do BruNya Shield

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ShieldProfile {
    Essential,
    #[default]
    Balanced, // Padrão recomendado
    Strict,
    Custom,
}

impl ShieldProfile {
    pub fn display_name(&self) -> &'static str {
        match self {
            ShieldProfile::Essential => "Essencial",
            ShieldProfile::Balanced => "Equilibrado (Recomendado)",
            ShieldProfile::Strict => "Rigoroso",
            ShieldProfile::Custom => "Personalizado",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShieldModulesConfig {
    pub adblock: bool,
    pub anti_tracking: bool,
    pub malicious_sites: bool,
    pub anti_popup: bool,
    pub anti_annoyances: bool,
    pub anti_fingerprinting: bool,
    pub anti_crypto_mining: bool,
    pub url_cleaner: bool,
    pub third_party_cookies: bool,
    pub video_ads: bool,
}

impl Default for ShieldModulesConfig {
    fn default() -> Self {
        // No perfil Equilibrado (padrão), todos os 10 módulos estão ativos
        Self {
            adblock: true,
            anti_tracking: true,
            malicious_sites: true,
            anti_popup: true,
            anti_annoyances: true,
            anti_fingerprinting: true,
            anti_crypto_mining: true,
            url_cleaner: true,
            third_party_cookies: true,
            video_ads: true,
        }
    }
}

impl ShieldModulesConfig {
    pub fn for_profile(profile: ShieldProfile) -> Self {
        match profile {
            ShieldProfile::Essential => Self {
                adblock: true,
                anti_tracking: true,
                malicious_sites: true,
                anti_popup: true,
                anti_annoyances: false,
                anti_fingerprinting: false,
                anti_crypto_mining: false,
                url_cleaner: false,
                third_party_cookies: false,
                video_ads: false,
            },
            ShieldProfile::Balanced => Self::default(),
            ShieldProfile::Strict => Self {
                adblock: true,
                anti_tracking: true,
                malicious_sites: true,
                anti_popup: true,
                anti_annoyances: true,
                anti_fingerprinting: true,
                anti_crypto_mining: true,
                url_cleaner: true,
                third_party_cookies: true,
                video_ads: true,
            },
            ShieldProfile::Custom => Self::default(),
        }
    }
}
