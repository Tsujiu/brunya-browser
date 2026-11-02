// BruNya Shield — YouTube Native AdBlocker Scriptlet

(function() {
    'use strict';

    // 1. Interceptar JSON.parse para limpar adPlacements e adSlots da resposta da API do YouTube
    const origParse = JSON.parse;
    JSON.parse = function() {
        const data = origParse.apply(this, arguments);
        if (data && typeof data === 'object') {
            if (data.adPlacements) data.adPlacements = [];
            if (data.adSlots) data.adSlots = [];
            if (data.playerAds) data.playerAds = [];
            if (data.instreamAdPlayer) delete data.instreamAdPlayer;
        }
        return data;
    };

    // 2. Interceptar window.fetch para interceptar requisições /youtubei/v1/player
    if (typeof window.fetch === 'function') {
        const origFetch = window.fetch;
        window.fetch = async function(...args) {
            const response = await origFetch.apply(this, args);
            const url = (args[0] && args[0].url) || args[0] || '';
            if (typeof url === 'string' && url.includes('/youtubei/v1/player')) {
                try {
                    const clone = response.clone();
                    const json = await clone.json();
                    if (json && typeof json === 'object') {
                        if (json.adPlacements) json.adPlacements = [];
                        if (json.adSlots) json.adSlots = [];
                        if (json.playerAds) json.playerAds = [];
                        return new Response(JSON.stringify(json), {
                            status: response.status,
                            statusText: response.statusText,
                            headers: response.headers
                        });
                    }
                } catch (e) {}
            }
            return response;
        };
    }

    // 3. Ocultação e remoção contínua de anúncios visuais e Companion Ads
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

        // Pular anúncios de vídeo em execução
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

    setInterval(cleanAds, 150);

    if (document.body) {
        const observer = new MutationObserver(cleanAds);
        observer.observe(document.body, { childList: true, subtree: true });
    } else {
        window.addEventListener('DOMContentLoaded', () => {
            const observer = new MutationObserver(cleanAds);
            observer.observe(document.body, { childList: true, subtree: true });
        });
    }
})();
