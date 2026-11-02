//! Módulo de Filtragem Cosmética e Ocultação de Elementos Intrusivos

pub struct CosmeticFilter;

impl CosmeticFilter {
    /// Gera CSS de filtragem cosmética padrão para injeção na página web
    pub fn default_cosmetic_stylesheet() -> &'static str {
        r#"
        /* Ocultação de banners e blocos publicitários */
        .ad, .ads, .advertisement, [id^="google_ads_"], [class*="sponsored-post"],
        [data-ad-client], .dfp-ad, .ad-banner, .banner-ad,
        /* Ocultação de sobreposições intrusivas de newsletter e avisos de cookies forçados */
        .cookie-notice-banner, .gdpr-modal-overlay, .newsletter-signup-overlay {
            display: none !important;
            visibility: hidden !important;
            height: 0 !important;
            max-height: 0 !important;
            opacity: 0 !important;
            pointer-events: none !important;
        }
        "#
    }
}
