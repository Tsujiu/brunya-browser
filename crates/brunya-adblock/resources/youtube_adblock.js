// BruNya Shield — YouTube Native AdBlocker & Companion Cleaner Scriptlet

(function() {
    'use strict';

    const cleanAds = () => {
        const adSelectors = [
            'ytd-action-companion-ad-renderer',
            'ytd-companion-slot-renderer',
            'ytd-ad-slot-renderer',
            'ytd-in-feed-ad-layout-renderer',
            'ytd-promoted-sparkles-web-renderer',
            'ytd-promoted-video-renderer',
            'ytd-banner-promo-renderer',
            '.video-ads',
            '.ytp-ad-module',
            '.ytp-ad-overlay-container',
            '.ytp-ad-player-overlay',
            '#player-ads',
            'ytd-engagement-panel-section-list-renderer[target-id="engagement-panel-ads"]',
            'ytd-player-legacy-desktop-watch-ads-renderer'
        ];

        for (const sel of adSelectors) {
            const els = document.querySelectorAll(sel);
            for (const el of els) {
                el.style.display = 'none';
                el.remove();
            }
        }

        // Pular anúncios de vídeo em reprodução
        const player = document.querySelector('#movie_player, .html5-video-player');
        const video = document.querySelector('video');
        if (player && player.classList.contains('ad-showing')) {
            if (video && !isNaN(video.duration) && video.duration > 0) {
                video.currentTime = video.duration + 1;
                video.playbackRate = 16.0;
            }
            const skipBtn = document.querySelector(
                '.ytp-ad-skip-button, .ytp-ad-skip-button-modern, .ytp-skip-ad-button, button.ytp-ad-skip-button'
            );
            if (skipBtn) {
                skipBtn.click();
            }
        }
    };

    setInterval(cleanAds, 180);

    if (document.body) {
        const observer = new MutationObserver(cleanAds);
        observer.observe(document.body, { childList: true, subtree: true });
    } else {
        window.addEventListener('DOMContentLoaded', () => {
            cleanAds();
            const observer = new MutationObserver(cleanAds);
            if (document.body) {
                observer.observe(document.body, { childList: true, subtree: true });
            }
        });
    }
})();
