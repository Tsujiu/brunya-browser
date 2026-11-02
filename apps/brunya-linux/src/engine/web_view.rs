use brunya_adblock::AdblockEngine;
use brunya_core::traits::NetworkInterceptor;
use brunya_core::types::{NavigationState, SecurityLevel, TabId};
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
use crate::engine::FaviconManager;

pub struct WebView {
    pub container: GtkBox,
    pub tab_id: TabId,
    internal_area: GtkBox,
    pub webkit_view: WebKitWebView,
    progress_bar: ProgressBar,
    current_state: RefCell<NavigationState>,
    pub current_favicon: RefCell<Option<gdk4::Texture>>,
    adblock: Arc<AdblockEngine>,
    storage: Arc<StorageManager>,
    settings: Arc<SettingsManager>,
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
        is_private: bool,
        on_state_changed: Rc<dyn Fn(&NavigationState)>,
    ) -> Rc<Self> {
        let container = GtkBox::new(Orientation::Vertical, 0);
        container.set_vexpand(true);
        container.set_hexpand(true);

        let progress_bar = ProgressBar::builder()
            .fraction(0.0)
            .visible(false)
            .build();
        container.append(&progress_bar);

        let internal_area = GtkBox::new(Orientation::Vertical, 0);
        internal_area.set_vexpand(true);
        internal_area.set_hexpand(true);
        container.append(&internal_area);

        // Criar WebView do WebKit 6.0 com isolamento de sessão para modo privado
        let webkit_view = if is_private {
            let session = NetworkSession::new_ephemeral();
            WebKitWebView::builder()
                .network_session(&session)
                .vexpand(true)
                .hexpand(true)
                .build()
        } else {
            WebKitWebView::builder()
                .vexpand(true)
                .hexpand(true)
                .build()
        };
        webkit_view.set_visible(false);
        container.append(&webkit_view);

        // Configurar WebKit Settings
        if let Some(wk_settings) = webkit6::prelude::WebViewExt::settings(&webkit_view) {
            wk_settings.set_enable_developer_extras(true);
            wk_settings.set_enable_page_cache(true);
            wk_settings.set_javascript_can_access_clipboard(true);
            wk_settings.set_enable_smooth_scrolling(true);
            wk_settings.set_user_agent(Some(
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/156.0.0.0 Safari/537.36 Brunya/0.3.0",
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

            // Compilar e registrar o filtro de conteúdo nativo do WebKit
            let home = std::env::var("HOME").unwrap_or_default();
            let filter_store_path = format!("{}/.local/share/brunya/filters/webkit", home);
            std::fs::create_dir_all(&filter_store_path).ok();

            let filter_store = webkit6::UserContentFilterStore::new(&filter_store_path);
            let rules_bytes = glib::Bytes::from_static(include_bytes!(
                "../../../../crates/brunya-adblock/resources/adblock_rules.json"
            ));

            let cm_clone = content_manager.clone();
            filter_store.save("brunya_shield_rules", &rules_bytes, None::<&gio::Cancellable>, move |res| {
                if let Ok(filter) = res {
                    cm_clone.add_filter(&filter);
                    info!("BruNya Shield ativou o filtro de conteúdo nativo do WebKit com sucesso");
                }
            });
        }

        let default_start = if is_private { "brunya://private" } else { "brunya://newtab" };
        let default_title = if is_private { "Janela Privada" } else { "Nova Aba" };

        let initial_state = NavigationState {
            url: default_start.to_string(),
            title: default_title.to_string(),
            security_level: SecurityLevel::Internal,
            is_loading: false,
            ..Default::default()
        };

        let view = Rc::new(Self {
            container,
            tab_id,
            internal_area,
            webkit_view: webkit_view.clone(),
            progress_bar: progress_bar.clone(),
            current_state: RefCell::new(initial_state),
            current_favicon: RefCell::new(None),
            adblock: adblock.clone(),
            storage: storage.clone(),
            settings,
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
                        let mut state = this.current_state.borrow_mut();
                        state.is_loading = true;
                        state.can_go_back = v.can_go_back();
                        state.can_go_forward = v.can_go_forward();
                        (this.on_state_changed)(&state);
                    }
                }
                LoadEvent::Finished => {
                    pb_load.set_visible(false);
                    if let Some(this) = this_load.upgrade() {
                        this.is_loading.store(false, Ordering::SeqCst);
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

        // Carregar página inicial
        view.load_url(default_start);
        view
    }

    pub fn navigation_state(&self) -> NavigationState {
        self.current_state.borrow().clone()
    }

    pub fn load_url(self: &Rc<Self>, url: &str) {
        let mut clean_url = url.trim().to_string();
        info!("Aba {} navegando para: {}", self.tab_id, clean_url);

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
                ("Nova Aba", crate::ui::internal_views::build_newtab_view(
                    self.settings.clone(),
                    move |target| {
                        self_clone.load_url(target);
                    },
                ))
            }
            "brunya://settings" => {
                let self_clone = self.clone();
                ("Configurações", crate::ui::internal_views::build_settings_view(
                    self.settings.clone(),
                    move |_lang| {
                        self_clone.render_internal_page("brunya://settings");
                    },
                ))
            }
            "brunya://history" => {
                let self_clone = self.clone();
                ("Histórico", crate::ui::internal_views::build_history_view(
                    self.storage.clone(),
                    move |target| {
                        self_clone.load_url(target);
                    },
                ))
            }
            "brunya://downloads" => {
                ("Downloads", crate::ui::internal_views::build_downloads_view(
                    self.storage.clone(),
                ))
            }
            "brunya://bookmarks" => {
                let self_clone = self.clone();
                ("Favoritos", crate::ui::internal_views::build_bookmarks_view(
                    self.storage.clone(),
                    move |target| {
                        self_clone.load_url(target);
                    },
                ))
            }
            "brunya://private" => ("Janela Privada", crate::ui::private_view::build_private_info_view()),
            "brunya://about" => ("Sobre o Brunya", crate::ui::internal_views::build_about_view()),
            _ => ("Página Não Encontrada", crate::ui::internal_views::build_about_view()),
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
            .label(&format!("Falha ao conectar a '{}'\n{}", url, error_msg))
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
