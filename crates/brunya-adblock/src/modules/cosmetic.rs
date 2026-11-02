//! Módulo de Filtragem Cosmética e Ocultação de Elementos Intrusivos

pub struct CosmeticFilter;

impl CosmeticFilter {
    /// Gera CSS de filtragem cosmética padrão para injeção na página web
    pub fn default_cosmetic_stylesheet() -> &'static str {
        r#"
        /* Ocultação de banners e blocos publicitários */
        .ad, .ads, .advertisement, .adbox, .banner_ads, .adsbox, [id^="google_ads_"],
        [class*="sponsored-post"], [class*="banner_ads"], [class*="adsbox"],
        [data-ad-client], .dfp-ad, .ad-banner, .banner-ad, #pagead, #ad-container,
        /* Ocultação de blocos e anúncios de vídeo no YouTube */
        ytd-action-companion-ad-renderer, ytd-companion-slot-renderer,
        ytd-ad-slot-renderer, ytd-in-feed-ad-layout-renderer,
        ytd-promoted-sparkles-web-renderer, ytd-promoted-video-renderer,
        ytd-banner-promo-renderer, .video-ads, .ytp-ad-module,
        .ytp-ad-overlay-container, .ytp-ad-player-overlay,
        .ytp-ad-skip-button-container, .ytp-ad-message-container,
        #player-ads, ytd-engagement-panel-section-list-renderer[target-id="engagement-panel-ads"],
        ytd-player-legacy-desktop-watch-ads-renderer,
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
