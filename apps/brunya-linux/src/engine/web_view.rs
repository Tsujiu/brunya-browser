use brunya_adblock::AdblockEngine;
use brunya_core::traits::NetworkInterceptor;
use brunya_core::types::{NavigationState, SecurityLevel, TabId};
use brunya_extensions::store::{parse_store_input, StoreRequest};
use brunya_extensions::types::ExtensionId;
use brunya_extensions::ExtensionSystem;
use brunya_i18n::t;
use brunya_settings::SettingsManager;
use brunya_storage::StorageManager;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation, ProgressBar};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{info, warn};
use webkit6::prelude::*;
use webkit6::{LoadEvent, NetworkSession, WebView as WebKitWebView};
use crate::engine::extension_host::ExtensionBridge;
use crate::engine::FaviconManager;

pub struct WebView {
    pub container: GtkBox,
    pub tab_id: TabId,
    internal_area: GtkBox,
    pub webkit_view: WebKitWebView,
    progress_bar: ProgressBar,
    store_bar: GtkBox,
    store_label: Label,
    store_button: Button,
    store_url: RefCell<Option<String>>,
    store_busy: RefCell<bool>,
    current_state: RefCell<NavigationState>,
    pub current_favicon: RefCell<Option<gdk4::Texture>>,
    adblock: Arc<AdblockEngine>,
    storage: Arc<StorageManager>,
    settings: Arc<SettingsManager>,
    extension_system: Option<Arc<ExtensionSystem>>,
    extension_bridge: Option<Rc<ExtensionBridge>>,
    on_extensions_changed: Rc<dyn Fn(&str)>,
    _is_private: bool,
    is_loading: AtomicBool,
    on_state_changed: Rc<dyn Fn(&NavigationState)>,
}

impl WebView {
    pub fn new(
        tab_id: TabId,
        adblock: Arc<AdblockEngine>,
        storage: Arc<StorageManager>,
        settings: Arc<SettingsManager>,
        extension_system: Option<Arc<ExtensionSystem>>,
        extension_bridge: Option<Rc<ExtensionBridge>>,
        is_private: bool,
        on_state_changed: Rc<dyn Fn(&NavigationState)>,
        on_extensions_changed: Rc<dyn Fn(&str)>,
    ) -> Rc<Self> {
        let container = GtkBox::new(Orientation::Vertical, 0);
        container.set_vexpand(true);
        container.set_hexpand(true);

        let progress_bar = ProgressBar::builder()
            .fraction(0.0)
            .visible(false)
            .build();
        container.append(&progress_bar);

        let store_bar = GtkBox::new(Orientation::Horizontal, 12);
        store_bar.add_css_class("store-install-bar");
        store_bar.set_visible(false);
        let store_label = Label::builder()
            .halign(Align::Start)
            .hexpand(true)
            .wrap(true)
            .css_classes(["store-install-label"])
            .build();
        let store_button = Button::builder()
            .css_classes(["settings-pill", "suggested-action"])
            .build();
        store_bar.append(&store_label);
        store_bar.append(&store_button);
        container.append(&store_bar);

        let internal_area = GtkBox::new(Orientation::Vertical, 0);
        internal_area.set_vexpand(true);
        internal_area.set_hexpand(true);
        container.append(&internal_area);

        // Configurar sessão de rede com banco SQLite persistente para cookies e dados
        let home = std::env::var("HOME").unwrap_or_default();
        let data_dir = format!("{}/.local/share/brunya/data", home);
        let cache_dir = format!("{}/.cache/brunya/cache", home);
        std::fs::create_dir_all(&data_dir).ok();
        std::fs::create_dir_all(&cache_dir).ok();

        let session = if is_private {
            NetworkSession::new_ephemeral()
        } else {
            let s = NetworkSession::new(Some(&data_dir), Some(&cache_dir));
            if let Some(cm) = s.cookie_manager() {
                let cookie_db = format!("{}/cookies.sqlite", data_dir);
                cm.set_persistent_storage(&cookie_db, webkit6::CookiePersistentStorage::Sqlite);
                cm.set_accept_policy(webkit6::CookieAcceptPolicy::Always);
            }
            s
        };

        let webkit_view = WebKitWebView::builder()
            .network_session(&session)
            .vexpand(true)
            .hexpand(true)
            .build();

        // Configurar cor de fundo escura nativa no WebKit para evitar tela branca ao carregar
        let dark_bg = gdk4::RGBA::new(0.06, 0.07, 0.09, 1.0);
        webkit_view.set_background_color(&dark_bg);
        webkit_view.set_visible(false);
        container.append(&webkit_view);

        // Configurar WebKit Settings com User-Agent neutro que não requer Client Hints do Chrome
        if let Some(wk_settings) = webkit6::prelude::WebViewExt::settings(&webkit_view) {
            wk_settings.set_enable_developer_extras(true);
            wk_settings.set_enable_page_cache(true);
            wk_settings.set_javascript_can_access_clipboard(true);
            wk_settings.set_enable_smooth_scrolling(true);
            wk_settings.set_user_agent(Some(
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
            ));
        }

        // Injetar filtros cosméticos nativos e filtro de conteúdo WebKit (bloqueio de subrecursos e rastreadores)
        if let Some(content_manager) = webkit_view.user_content_manager() {
            let css = brunya_adblock::CosmeticFilter::default_cosmetic_stylesheet();
            let user_style = webkit6::UserStyleSheet::new(
                css,
                webkit6::UserContentInjectedFrames::AllFrames,
                webkit6::UserStyleLevel::User,
                &[],
                &[],
            );
            content_manager.add_style_sheet(&user_style);

            // Injetar scriptlet nativo de bloqueio de anúncios e telemetria do YouTube (Módulo 10)
            let yt_script_code = include_str!("../../../../crates/brunya-adblock/resources/youtube_adblock.js");
            let yt_user_script = webkit6::UserScript::new(
                yt_script_code,
                webkit6::UserContentInjectedFrames::AllFrames,
                webkit6::UserScriptInjectionTime::Start,
                &["*://*.youtube.com/*", "*://youtube.com/*"],
                &[],
            );
            content_manager.add_script(&yt_user_script);
            content_manager.add_script(&webkit6::UserScript::new(
                STORE_HOOK_JS,
                webkit6::UserContentInjectedFrames::AllFrames,
                webkit6::UserScriptInjectionTime::Start,
                &[
                    "*://chromewebstore.google.com/*",
                    "*://chrome.google.com/*",
                    "*://microsoftedge.microsoft.com/*",
                    "*://addons.mozilla.org/*",
                    "*://addons.opera.com/*",
                ],
                &[],
            ));

            // Carregar ou compilar o filtro de conteúdo nativo do WebKit
            let home = std::env::var("HOME").unwrap_or_default();
            let filter_store_path = format!("{}/.local/share/brunya/filters/webkit", home);
            std::fs::create_dir_all(&filter_store_path).ok();

            let filter_store = webkit6::UserContentFilterStore::new(&filter_store_path);
            let rules_bytes = glib::Bytes::from_static(include_bytes!(
                "../../../../crates/brunya-adblock/resources/adblock_rules.json"
            ));

            let cm_clone = content_manager.clone();
            let store_clone = filter_store.clone();
            filter_store.load("brunya_shield_rules", None::<&gio::Cancellable>, move |res| {
                if let Ok(filter) = res {
                    cm_clone.add_filter(&filter);
                    info!("BruNya Shield carregou o filtro de conteúdo compilado imediatamente");
                } else {
                    let cm_fallback = cm_clone.clone();
                    store_clone.save("brunya_shield_rules", &rules_bytes, None::<&gio::Cancellable>, move |save_res| {
                        if let Ok(filter) = save_res {
                            cm_fallback.add_filter(&filter);
                            info!("BruNya Shield compilou e registrou o filtro de conteúdo nativo do WebKit");
                        }
                    });
                }
            });
        }

        let default_start = if is_private { "brunya://private" } else { "brunya://newtab" };
        let default_title = if is_private { t!("common.private_window_title") } else { t!("common.new_tab") };

        let initial_state = NavigationState {
            url: default_start.to_string(),
            title: default_title.to_string(),
            security_level: SecurityLevel::Internal,
            is_loading: false,
            ..Default::default()
        };

        if let Some(bridge) = &extension_bridge {
            bridge.attach_content_view(&webkit_view, tab_id.0);
        }

        let view = Rc::new(Self {
            container,
            tab_id,
            internal_area,
            webkit_view: webkit_view.clone(),
            progress_bar: progress_bar.clone(),
            store_bar: store_bar.clone(),
            store_label: store_label.clone(),
            store_button: store_button.clone(),
            store_url: RefCell::new(None),
            store_busy: RefCell::new(false),
            current_state: RefCell::new(initial_state),
            current_favicon: RefCell::new(None),
            adblock: adblock.clone(),
            storage: storage.clone(),
            settings,
            extension_system,
            extension_bridge: extension_bridge.clone(),
            on_extensions_changed,
            _is_private: is_private,
            is_loading: AtomicBool::new(false),
            on_state_changed: on_state_changed.clone(),
        });

        // 0. Notificação de Favicon real do site
        let this_fav = Rc::downgrade(&view);
        let on_fav_changed = on_state_changed.clone();
        webkit_view.connect_favicon_notify(move |v| {
            if let Some(this) = this_fav.upgrade() {
                let fav = v.favicon();
                *this.current_favicon.borrow_mut() = fav;
                let state = this.current_state.borrow().clone();
                on_fav_changed(&state);
            }
        });

        // 1. Notificação de URI
        let this_uri = Rc::downgrade(&view);
        let storage_uri = storage.clone();
        let is_priv = is_private;
        webkit_view.connect_uri_notify(move |v| {
            if let Some(uri) = v.uri() {
                if let Some(this) = this_uri.upgrade() {
                    let mut state = this.current_state.borrow_mut();
                    state.url = uri.to_string();
                    let is_https = uri.starts_with("https://");
                    state.security_level = if is_https {
                        SecurityLevel::Secure
                    } else {
                        SecurityLevel::Insecure
                    };
                    state.can_go_back = v.can_go_back();
                    state.can_go_forward = v.can_go_forward();
                    (this.on_state_changed)(&state);

                    // Atualizar favicon do site automaticamente
                    if let Some(domain) = FaviconManager::extract_domain(&uri) {
                        if let Some(tex) = FaviconManager::load_cached_favicon(&domain) {
                            *this.current_favicon.borrow_mut() = Some(tex);
                        } else {
                            let this_f = Rc::downgrade(&this);
                            FaviconManager::fetch_favicon_async(domain, move |tex| {
                                if let Some(th) = this_f.upgrade() {
                                    *th.current_favicon.borrow_mut() = Some(tex);
                                    let st = th.current_state.borrow().clone();
                                    (th.on_state_changed)(&st);
                                }
                            });
                        }
                    }

                    if !is_priv && !uri.starts_with("brunya://") {
                        storage_uri.record_history_visit(&uri, &state.title).ok();
                    }
                }
            }
        });

        // 2. Notificação de Título
        let this_title = Rc::downgrade(&view);
        webkit_view.connect_title_notify(move |v| {
            if let Some(t) = v.title() {
                if let Some(this) = this_title.upgrade() {
                    let mut state = this.current_state.borrow_mut();
                    state.title = t.to_string();
                    (this.on_state_changed)(&state);
                }
            }
        });

        // 3. Progresso estimado
        let pb_progress = progress_bar.clone();
        let this_progress = Rc::downgrade(&view);
        webkit_view.connect_estimated_load_progress_notify(move |v| {
            let p = v.estimated_load_progress();
            pb_progress.set_fraction(p);
            if let Some(this) = this_progress.upgrade() {
                let mut state = this.current_state.borrow_mut();
                state.progress = p;
                (this.on_state_changed)(&state);
            }
        });

        // 4. Mudança de carregamento
        let pb_load = progress_bar.clone();
        let this_load = Rc::downgrade(&view);
        webkit_view.connect_load_changed(move |v, event| {
            match event {
                LoadEvent::Started => {
                    pb_load.set_visible(true);
                    if let Some(this) = this_load.upgrade() {
                        this.is_loading.store(true, Ordering::SeqCst);
                        if let Some(bridge) = &this.extension_bridge {
                            bridge.reset_injection(&this.webkit_view);
                        }
                        let mut state = this.current_state.borrow_mut();
                        state.is_loading = true;
                        state.can_go_back = v.can_go_back();
                        state.can_go_forward = v.can_go_forward();
                        (this.on_state_changed)(&state);
                    }
                }
                LoadEvent::Committed => {
                    if let Some(this) = this_load.upgrade() {
                        if let Some(uri) = v.uri() {
                            this.sync_store_offer(uri.as_str());
                            if let Some(bridge) = this.extension_bridge.clone() {
                                bridge.inject_content(&this.webkit_view, uri.as_str(), "document_start");
                                bridge.inject_content(&this.webkit_view, uri.as_str(), "document_end");
                            }
                        }
                    }
                }
                LoadEvent::Finished => {
                    pb_load.set_visible(false);
                    if let Some(this) = this_load.upgrade() {
                        this.is_loading.store(false, Ordering::SeqCst);
                        if let (Some(bridge), Some(uri)) = (this.extension_bridge.clone(), v.uri()) {
                            bridge.inject_content(&this.webkit_view, uri.as_str(), "document_idle");
                        }
                        let mut state = this.current_state.borrow_mut();
                        state.is_loading = false;
                        state.can_go_back = v.can_go_back();
                        state.can_go_forward = v.can_go_forward();
                        (this.on_state_changed)(&state);
                    }
                }
                _ => {}
            }
        });

        // 5. Interceptador de Adblock na política de navegação
        let ab_policy = adblock.clone();
        webkit_view.connect_decide_policy(move |_v, decision, decision_type| {
            if decision_type == webkit6::PolicyDecisionType::NavigationAction {
                if let Some(nav_decision) = decision.downcast_ref::<webkit6::NavigationPolicyDecision>() {
                    if let Some(req) = nav_decision.navigation_action().and_then(|a| a.request()) {
                        if let Some(uri) = req.uri() {
                            if ab_policy.should_block(tab_id, uri.as_str(), None, None) {
                                decision.ignore();
                                return true;
                            }
                        }
                    }
                }
            }
            false
        });

        if let Some(manager) = webkit_view.user_content_manager() {
            manager.register_script_message_handler("brunyaStore", None);
            let this_store = Rc::downgrade(&view);
            manager.connect_script_message_received(Some("brunyaStore"), move |_manager, value| {
                if let Some(this) = this_store.upgrade() {
                    this.on_store_message(value);
                }
            });
        }
        let this_store_btn = Rc::downgrade(&view);
        store_button.connect_clicked(move |_| {
            if let Some(this) = this_store_btn.upgrade() {
                this.start_store_install();
            }
        });

        // Carregar página inicial
        view.load_url(default_start);
        view
    }

    pub fn navigation_state(&self) -> NavigationState {
        self.current_state.borrow().clone()
    }

    fn sync_store_offer(&self, url: &str) {
        if self.extension_system.is_none() || !self.webkit_view.is_visible() {
            self.store_bar.set_visible(false);
            return;
        }
        let Ok(request) = parse_store_input(url) else {
            self.store_bar.set_visible(false);
            if !*self.store_busy.borrow() {
                *self.store_url.borrow_mut() = None;
            }
            return;
        };
        *self.store_url.borrow_mut() = Some(url.to_string());
        self.store_bar.set_visible(true);
        if *self.store_busy.borrow() {
            return;
        }
        let installed = self.store_already_installed(&request);
        let label = if installed {
            t!("extensions.store_bar_installed")
        } else {
            t!("extensions.store_bar_title", store => request.store_name())
        };
        let action = if installed {
            t!("extensions.store_bar_reinstall")
        } else {
            t!("extensions.store_bar_action")
        };
        self.store_label.set_label(&label);
        self.store_button.set_label(&action);
        self.store_button.set_sensitive(true);
    }

    fn store_already_installed(&self, request: &StoreRequest) -> bool {
        let Some(system) = &self.extension_system else { return false };
        let Some(id) = request.explicit_id() else { return false };
        system
            .manager
            .get_extension(&ExtensionId::new(id.to_string()))
            .is_some()
    }

    fn start_store_install(self: &Rc<Self>) {
        if *self.store_busy.borrow() {
            return;
        }
        let Some(url) = self.store_url.borrow().clone() else { return };
        let Some(system) = self.extension_system.clone() else { return };
        *self.store_busy.borrow_mut() = true;
        self.store_button.set_sensitive(false);
        let this_status = Rc::clone(self);
        let this_done = Rc::clone(self);
        crate::ui::internal_views::begin_store_install(
            url,
            &self.store_button,
            system,
            move |status| this_status.store_label.set_label(&status),
            move |end| {
                *this_done.store_busy.borrow_mut() = false;
                this_done.store_button.set_sensitive(true);
                if matches!(end, crate::ui::internal_views::StoreInstallEnd::Installed(_)) {
                    (this_done.on_extensions_changed)("refresh");
                }
                if this_done.webkit_view.is_visible() {
                    if let Some(uri) = this_done.webkit_view.uri() {
                        this_done.sync_store_offer(uri.as_str());
                    }
                } else {
                    this_done.store_bar.set_visible(false);
                }
            },
        );
    }

    fn on_store_message(self: &Rc<Self>, value: &webkit6::javascriptcore::Value) {
        let raw = script_message_text(value);
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(&raw) else { return };
        let Some(url) = msg.get("url").and_then(|value| value.as_str()) else { return };
        if url.is_empty() {
            return;
        }
        match msg.get("op").and_then(|value| value.as_str()).unwrap_or("") {
            "page" => self.sync_store_offer(url),
            "install" => {
                *self.store_url.borrow_mut() = Some(url.to_string());
                self.sync_store_offer(url);
                self.start_store_install();
            }
            _ => {}
        }
    }

    pub fn load_url(self: &Rc<Self>, url: &str) {
        let mut clean_url = url.trim().to_string();
        info!("Aba {} navegando para: {}", self.tab_id, clean_url);
        self.store_bar.set_visible(false);

        // 1. Módulo 08 — URL Cleaner (limpeza prévia de parâmetros invasivos de rastreamento)
        let (cleaned_url, stripped) = brunya_adblock::UrlCleaner::clean_url(&clean_url);
        if stripped > 0 {
            clean_url = cleaned_url;
            self.adblock.stats().record_url_cleaned(stripped);
            info!("BruNya Shield URL Cleaner removeu {} parâmetros de rastreamento", stripped);
        }

        // 2. Módulo 03 — Threat Intelligence (detecção de phishing, malware e fraudes)
        if !clean_url.starts_with("brunya://") && self.adblock.modules_config().malicious_sites {
            let threat = brunya_adblock::ThreatIntelligence::inspect(&clean_url);
            if threat.is_threat {
                let domain = FaviconManager::extract_domain(&clean_url).unwrap_or_default();
                if !self.adblock.is_site_allowed(&domain) {
                    self.render_threat_warning_page(
                        &clean_url,
                        threat.category.unwrap_or(brunya_adblock::ThreatCategory::Malware),
                        &threat.reason,
                    );
                    return;
                }
            }
        }

        // 3. Interceptação pelo Bloqueador de Anúncios e Rastreadores
        let action = self.adblock.inspect_request(self.tab_id, &clean_url, "main_frame");
        if let brunya_core::traits::RequestAction::Block { reason } = action {
            warn!("Navegação bloqueada pelo Brunya Shield: {} ({})", clean_url, reason);
            self.render_error_page(&clean_url, &reason);
            return;
        }

        // 4. Alternar entre Páginas Internas nativas do Brunya e Web Real
        if clean_url.starts_with("brunya://") {
            *self.current_favicon.borrow_mut() = None;
            self.webkit_view.set_visible(false);
            self.internal_area.set_visible(true);
            self.render_internal_page(&clean_url);
        } else {
            self.internal_area.set_visible(false);
            self.webkit_view.set_visible(true);
            self.sync_store_offer(&clean_url);

            // Carregar favicon do domínio antecipadamente do cache ou disparar fetch assíncrono
            if let Some(domain) = FaviconManager::extract_domain(&clean_url) {
                if let Some(tex) = FaviconManager::load_cached_favicon(&domain) {
                    *self.current_favicon.borrow_mut() = Some(tex);
                } else {
                    let this_f = Rc::downgrade(self);
                    FaviconManager::fetch_favicon_async(domain, move |tex| {
                        if let Some(th) = this_f.upgrade() {
                            *th.current_favicon.borrow_mut() = Some(tex);
                            let st = th.current_state.borrow().clone();
                            (th.on_state_changed)(&st);
                        }
                    });
                }
            }

            self.webkit_view.load_uri(&clean_url);
        }
    }

    fn render_internal_page(self: &Rc<Self>, url: &str) {
        self.clear_internal_area();

        let (title, widget) = match url {
            "brunya://newtab" => {
                let self_clone = self.clone();
                (t!("common.new_tab"), crate::ui::internal_views::build_newtab_view(
                    self.settings.clone(),
                    self.storage.clone(),
                    move |target| {
                        self_clone.load_url(target);
                    },
                ))
            }
            "brunya://settings" => {
                let self_clone = self.clone();
                (t!("common.settings"), crate::ui::internal_views::build_settings_view(
                    self.settings.clone(),
                    self.storage.clone(),
                    move |_lang| {
                        self_clone.render_internal_page("brunya://settings");
                    },
                ))
            }
            "brunya://history" => {
                let self_clone = self.clone();
                (t!("common.history"), crate::ui::internal_views::build_history_view(
                    self.storage.clone(),
                    move |target| {
                        self_clone.load_url(target);
                    },
                ))
            }
            "brunya://downloads" => {
                (t!("common.downloads"), crate::ui::internal_views::build_downloads_view(
                    self.storage.clone(),
                ))
            }
            "brunya://bookmarks" => {
                let self_clone = self.clone();
                (t!("common.bookmarks"), crate::ui::internal_views::build_bookmarks_view(
                    self.storage.clone(),
                    move |target| {
                        self_clone.load_url(target);
                    },
                ))
            }
            "brunya://extensions" => {
                let title = t!("extensions.extensions_title");
                let widget = if let Some(ref ext_sys) = self.extension_system {
                    let self_clone = self.clone();
                    let on_changed = self.on_extensions_changed.clone();
                    crate::ui::internal_views::build_extensions_view(
                        ext_sys.clone(),
                        self.storage.clone(),
                        move |target| {
                            self_clone.load_url(target);
                        },
                        move |event| on_changed(event),
                    )
                } else {
                    let fallback = GtkBox::new(Orientation::Vertical, 0);
                    let label = Label::builder()
                        .label("Sistema de extensões não disponível nesta compilação.")
                        .css_classes(["dim-label"])
                        .margin_top(48)
                        .halign(Align::Center)
                        .build();
                    fallback.append(&label);
                    fallback.upcast()
                };
                (title, widget)
            }
            "brunya://private" => (t!("common.private_window_title"), crate::ui::private_view::build_private_info_view()),
            "brunya://about" => (t!("common.about"), crate::ui::internal_views::build_about_view()),
            _ => (t!("app_name"), crate::ui::internal_views::build_about_view()),
        };

        widget.set_vexpand(true);
        widget.set_hexpand(true);
        self.internal_area.append(&widget);

        // Atualizar estado
        let mut state = self.current_state.borrow_mut();
        state.url = url.to_string();
        state.title = title.to_string();
        state.security_level = SecurityLevel::Internal;
        state.is_loading = false;
        state.progress = 1.0;
        state.can_go_back = false;
        state.can_go_forward = false;
        (self.on_state_changed)(&state);
    }

    fn render_error_page(&self, url: &str, error_msg: &str) {
        self.store_bar.set_visible(false);
        self.clear_internal_area();
        self.webkit_view.set_visible(false);
        self.internal_area.set_visible(true);
        self.progress_bar.set_visible(false);
        self.is_loading.store(false, Ordering::SeqCst);

        let err_box = GtkBox::new(Orientation::Vertical, 16);
        err_box.set_valign(Align::Center);
        err_box.set_halign(Align::Center);
        err_box.set_margin_top(48);

        let icon = Label::builder()
            .label("⚠️")
            .css_classes(["display-1"])
            .build();

        let title = Label::builder()
            .label(&t!("errors.connection_failed"))
            .css_classes(["title-2", "heading"])
            .build();

        let desc = Label::builder()
            .label(&format!("{}\n{}", &t!("common.connection_failed_title", url => url), error_msg))
            .css_classes(["dim-label"])
            .justify(gtk4::Justification::Center)
            .build();

        let reload_btn = Button::builder()
            .label(&t!("errors.reload_page"))
            .css_classes(["suggested-action", "pill"])
            .build();

        err_box.append(&icon);
        err_box.append(&title);
        err_box.append(&desc);
        err_box.append(&reload_btn);
        err_box.set_vexpand(true);
        err_box.set_hexpand(true);

        self.internal_area.append(&err_box);

        let mut state = self.current_state.borrow_mut();
        state.url = url.to_string();
        state.title = "Erro de Conexão".to_string();
        state.security_level = SecurityLevel::BrokenTls;
        state.is_loading = false;
        (self.on_state_changed)(&state);
    }

    fn render_threat_warning_page(
        self: &Rc<Self>,
        url: &str,
        category: brunya_adblock::ThreatCategory,
        reason: &str,
    ) {
        self.store_bar.set_visible(false);
        self.clear_internal_area();
        self.webkit_view.set_visible(false);
        self.internal_area.set_visible(true);
        self.progress_bar.set_visible(false);
        self.is_loading.store(false, Ordering::SeqCst);

        let warn_box = GtkBox::new(Orientation::Vertical, 16);
        warn_box.set_valign(Align::Center);
        warn_box.set_halign(Align::Center);
        warn_box.set_margin_top(48);
        warn_box.set_margin_bottom(48);
        warn_box.set_margin_start(32);
        warn_box.set_margin_end(32);
        warn_box.set_width_request(560);

        let icon = Label::builder()
            .label("🚨")
            .css_classes(["display-1"])
            .build();

        let title = Label::builder()
            .label("Alerta de Segurança do BruNya Shield")
            .css_classes(["title-2", "heading", "bold"])
            .build();

        let desc = Label::builder()
            .label(&format!(
                "O BruNya Shield bloqueou o acesso a esta página para proteger seus dados e seu dispositivo.\n\nEndereço: {}\nCategoria: {}\nMotivo: {}",
                url, category.display_name(), reason
            ))
            .wrap(true)
            .justify(gtk4::Justification::Center)
            .build();

        let buttons_row = GtkBox::new(Orientation::Horizontal, 12);
        buttons_row.set_halign(Align::Center);

        let back_btn = Button::builder()
            .label("Voltar para Segurança (Recomendado)")
            .css_classes(["suggested-action", "pill"])
            .build();

        let this_back = self.clone();
        back_btn.connect_clicked(move |_| {
            this_back.go_back();
        });

        let proceed_btn = Button::builder()
            .label("Continuar mesmo assim (Inseguro)")
            .css_classes(["flat", "pill", "destructive-action"])
            .build();

        let this_proceed = self.clone();
        let target_url = url.to_string();
        let domain = FaviconManager::extract_domain(url).unwrap_or_default();
        let ab = self.adblock.clone();
        proceed_btn.connect_clicked(move |_| {
            ab.allow_site(&domain);
            this_proceed.load_url(&target_url);
        });

        buttons_row.append(&back_btn);
        buttons_row.append(&proceed_btn);

        warn_box.append(&icon);
        warn_box.append(&title);
        warn_box.append(&desc);
        warn_box.append(&buttons_row);
        warn_box.set_vexpand(true);
        warn_box.set_hexpand(true);

        self.internal_area.append(&warn_box);

        let mut state = self.current_state.borrow_mut();
        state.url = url.to_string();
        state.title = "Alerta de Segurança".to_string();
        state.security_level = SecurityLevel::Insecure;
        state.is_loading = false;
        (self.on_state_changed)(&state);
    }

    fn clear_internal_area(&self) {
        while let Some(child) = self.internal_area.first_child() {
            self.internal_area.remove(&child);
        }
    }

    pub fn go_back(self: &Rc<Self>) {
        if self.webkit_view.is_visible() && self.webkit_view.can_go_back() {
            self.webkit_view.go_back();
        } else {
            self.load_url("brunya://newtab");
        }
    }

    pub fn go_forward(self: &Rc<Self>) {
        if self.webkit_view.is_visible() && self.webkit_view.can_go_forward() {
            self.webkit_view.go_forward();
        }
    }

    pub fn reload(self: &Rc<Self>) {
        if self.webkit_view.is_visible() {
            self.webkit_view.reload();
        } else {
            let u = self.current_state.borrow().url.clone();
            self.load_url(&u);
        }
    }

    pub fn stop(self: &Rc<Self>) {
        if self.webkit_view.is_visible() {
            self.webkit_view.stop_loading();
        }
    }

    pub fn calculate_ram_mb(&self, tab_index: usize, url: &str) -> u64 {
        let is_internal = url.starts_with("brunya://");
        brunya_diagnostics::get_distinct_tab_memory_mb(tab_index, url, is_internal)
    }
}

fn script_message_text(value: &webkit6::javascriptcore::Value) -> String {
    if let Some(json) = value.to_json(0) {
        let text = json.to_string();
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(inner) = parsed.as_str() {
                return inner.to_string();
            }
            return text;
        }
    }
    value.to_str().to_string()
}

const STORE_HOOK_JS: &str = r#"
(() => {
  if (window.__brunyaStoreHook) return;
  window.__brunyaStoreHook = true;

  const call = (callback, value) => {
    if (typeof callback === "function") callback(value);
  };
  const post = (op) => {
    try {
      const bridge = window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.brunyaStore;
      if (bridge) bridge.postMessage(JSON.stringify({ op: op, url: String(location.href) }));
    } catch (e) {}
  };
  const chromeApi = window.chrome || {};
  chromeApi.runtime = chromeApi.runtime || {
    sendMessage() {},
    connect() { return { onMessage: { addListener() {} }, postMessage() {}, disconnect() {} }; }
  };
  chromeApi.webstorePrivate = chromeApi.webstorePrivate || {
    getExtensionStatus(id, manifest, callback) {
      call(typeof manifest === "function" ? manifest : callback, "installable");
    },
    beginInstallWithManifest3(details, callback) {
      post("install");
      call(callback, "");
    },
    completeInstall(id, callback) { call(callback, ""); },
    isInIncognitoMode(callback) { call(callback, false); },
    getReferrerChain(callback) { call(callback, "EgIIAA=="); },
    getBrowserLogin(callback) { call(callback); }
  };
  try { window.chrome = chromeApi; } catch (e) {}
  if (!navigator.userAgentData) {
    const brands = [
      { brand: "Chromium", version: "143" },
      { brand: "Google Chrome", version: "143" },
      { brand: "Not_A Brand", version: "24" }
    ];
    const data = {
      brands: brands,
      mobile: false,
      platform: "Linux",
      getHighEntropyValues() {
        return Promise.resolve({
          brands: brands,
          mobile: false,
          platform: "Linux",
          architecture: "x86",
          bitness: "64",
          model: "",
          platformVersion: "6.8.0",
          uaFullVersion: "143.0.7499.41",
          fullVersionList: brands
        });
      }
    };
    try {
      Object.defineProperty(navigator, "userAgentData", { configurable: true, get: () => data });
    } catch (e) {}
  }

  if (window.top !== window) return;
  let last = "";
  const notify = () => {
    const url = String(location.href);
    if (url === last) return;
    last = url;
    const old = document.getElementById("brunya-store-install");
    if (old) old.remove();
    post("page");
    scheduleButton();
  };
  const phrases = [
    "add to chrome",
    "adicionar ao chrome",
    "usar no chrome",
    "adicionar ao brunya",
    "add to brunya",
    "add to edge",
    "adicionar ao edge",
    "add to firefox",
    "adicionar ao firefox",
    "add to opera",
    "adicionar ao opera"
  ];
  const unavailable = [
    "item indisponível no momento",
    "item currently unavailable",
    "this item is not available",
    "este item não está disponível"
  ];
  const isInstall = (value) => {
    const text = String(value || "").replace(/\s+/g, " ").trim().toLowerCase();
    if (!text || text.length > 80) return false;
    return phrases.some((phrase) => text === phrase || text.startsWith(phrase));
  };
  let scheduled = false;
  const scheduleButton = () => {
    if (scheduled) return;
    scheduled = true;
    setTimeout(() => { scheduled = false; placeButton(); }, 250);
  };
  const placeButton = () => {
    if (document.getElementById("brunya-store-install")) return;
    if (!/[a-p]{32}/i.test(location.pathname)) return;
    const body = document.body;
    if (!body) return;
    const walker = document.createTreeWalker(body, NodeFilter.SHOW_TEXT);
    let node;
    while ((node = walker.nextNode())) {
      const value = String(node.nodeValue || "").replace(/\s+/g, " ").trim().toLowerCase();
      if (!unavailable.some((phrase) => value.includes(phrase))) continue;
      const host = node.parentElement;
      if (!host) return;
      const button = document.createElement("button");
      button.id = "brunya-store-install";
      button.type = "button";
      button.textContent = "Adicionar ao Brunya";
      button.style.cssText = "display:inline-block;margin:12px 0;padding:10px 18px;border:0;border-radius:999px;background:#1a73e8;color:#fff;font:600 14px/1.2 system-ui,sans-serif;cursor:pointer;";
      host.insertAdjacentElement("afterend", button);
      return;
    }
  };
  window.addEventListener("click", (event) => {
    const path = event.composedPath ? event.composedPath() : [event.target];
    for (const node of path) {
      if (!node || node.nodeType !== 1) continue;
      const aria = node.getAttribute ? (node.getAttribute("aria-label") || "") : "";
      const own = node.innerText || node.textContent || "";
      if (node.id === "brunya-store-install" || isInstall(aria) || isInstall(own)) {
        event.preventDefault();
        event.stopImmediatePropagation();
        post("install");
        return;
      }
    }
  }, true);
  const wrap = (name) => {
    const original = history[name];
    if (typeof original !== "function") return;
    history[name] = function () {
      const result = original.apply(this, arguments);
      setTimeout(notify, 0);
      return result;
    };
  };
  wrap("pushState");
  wrap("replaceState");
  window.addEventListener("popstate", () => setTimeout(notify, 0));
  const root = document.documentElement;
  if (root) {
    new MutationObserver(scheduleButton).observe(root, { childList: true, subtree: true });
  }
  notify();
})();
"#;
