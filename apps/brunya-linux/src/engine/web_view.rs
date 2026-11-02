use brunya_adblock::AdblockEngine;
use brunya_core::traits::NetworkInterceptor;
use brunya_core::types::{NavigationState, SecurityLevel, TabId};
use brunya_i18n::t;
use brunya_settings::SettingsManager;
use brunya_storage::StorageManager;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Label, Orientation, ProgressBar, ScrolledWindow,
    TextBuffer, TextTagTable, TextView,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{error, info, warn};

enum NetworkResponse {
    Success {
        url: String,
        title: String,
        body: String,
        status: i32,
        sec_level: SecurityLevel,
    },
    Error {
        url: String,
        message: String,
    },
}

pub struct WebView {
    pub container: GtkBox,
    pub tab_id: TabId,
    content_area: GtkBox,
    progress_bar: ProgressBar,
    current_state: RefCell<NavigationState>,
    history_stack: RefCell<Vec<String>>,
    history_index: RefCell<usize>,
    adblock: Arc<AdblockEngine>,
    storage: Arc<StorageManager>,
    settings: Arc<SettingsManager>,
    is_private: bool,
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

        let progress_bar = ProgressBar::builder()
            .fraction(0.0)
            .visible(false)
            .build();
        container.append(&progress_bar);

        let content_area = GtkBox::new(Orientation::Vertical, 0);
        content_area.set_vexpand(true);
        content_area.set_hexpand(true);
        container.append(&content_area);

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
            content_area,
            progress_bar,
            current_state: RefCell::new(initial_state),
            history_stack: RefCell::new(vec![default_start.to_string()]),
            history_index: RefCell::new(0),
            adblock,
            storage,
            settings,
            is_private,
            is_loading: AtomicBool::new(false),
            on_state_changed,
        });

        // Carregar página inicial
        view.load_url(default_start);
        view
    }

    pub fn navigation_state(&self) -> NavigationState {
        self.current_state.borrow().clone()
    }

    pub fn load_url(self: &Rc<Self>, url: &str) {
        let clean_url = url.trim().to_string();
        info!("Aba {} navegando para: {}", self.tab_id, clean_url);

        // 1. Interceptação pelo Bloqueador de Anúncios e Rastreadores
        let action = self.adblock.inspect_request(self.tab_id, &clean_url, "main_frame");
        if let brunya_core::traits::RequestAction::Block { reason } = action {
            warn!("Navegação bloqueada pelo Brunya Shield: {} ({})", clean_url, reason);
            self.render_error_page(&clean_url, &reason);
            return;
        }

        // 2. Atualizar histórico interno de navegação (Back/Forward)
        {
            let mut stack = self.history_stack.borrow_mut();
            let mut idx = self.history_index.borrow_mut();
            if *idx < stack.len() && stack[*idx] != clean_url {
                stack.truncate(*idx + 1);
                stack.push(clean_url.clone());
                *idx = stack.len() - 1;
            }
        }

        // 3. Renderizar páginas internas ou carregar da Web
        if clean_url.starts_with("brunya://") {
            self.render_internal_page(&clean_url);
        } else {
            self.fetch_and_render_web_page(&clean_url);
        }
    }

    fn render_internal_page(self: &Rc<Self>, url: &str) {
        self.clear_content();

        let (title, widget) = match url {
            "brunya://newtab" => {
                let self_clone = self.clone();
                ("Nova Aba", crate::ui::internal_views::build_newtab_view(move |target| {
                    self_clone.load_url(target);
                }))
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

        self.content_area.append(&widget);

        // Atualizar estado
        let mut state = self.current_state.borrow_mut();
        state.url = url.to_string();
        state.title = title.to_string();
        state.security_level = SecurityLevel::Internal;
        state.is_loading = false;
        state.progress = 1.0;
        self.update_can_go(&mut state);
        (self.on_state_changed)(&state);
    }

    fn fetch_and_render_web_page(self: &Rc<Self>, url: &str) {
        self.is_loading.store(true, Ordering::SeqCst);
        self.progress_bar.set_visible(true);
        self.progress_bar.set_fraction(0.3);

        let is_https = url.starts_with("https://");
        let sec_level = if is_https {
            SecurityLevel::Secure
        } else {
            SecurityLevel::Insecure
        };

        // Atualizar estado inicial de carregamento
        {
            let mut state = self.current_state.borrow_mut();
            state.url = url.to_string();
            state.title = "Carregando...".to_string();
            state.security_level = sec_level;
            state.is_loading = true;
            state.progress = 0.3;
            self.update_can_go(&mut state);
            (self.on_state_changed)(&state);
        }

        let target_url = url.to_string();
        let (sender, receiver) = std::sync::mpsc::channel::<NetworkResponse>();

        let self_weak = Rc::downgrade(self);
        glib::idle_add_local(move || {
            match receiver.try_recv() {
                Ok(msg) => {
                    if let Some(this) = self_weak.upgrade() {
                        match msg {
                            NetworkResponse::Success {
                                url,
                                title,
                                body,
                                status,
                                sec_level,
                            } => {
                                this.render_html_content(&url, &title, &body, status, sec_level);
                            }
                            NetworkResponse::Error { url, message } => {
                                this.render_error_page(&url, &message);
                            }
                        }
                    }
                    glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
            }
        });

        // Carregar em thread separada de I/O de rede
        std::thread::spawn(move || {
            let user_agent = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/156.0.0.0 Safari/537.36 Brunya/0.3.0";

            let config_res = ureq::Agent::config_builder()
                .timeout_global(Some(std::time::Duration::from_secs(12)))
                .build();
            let agent = ureq::Agent::new_with_config(config_res);

            match agent.get(&target_url).header("User-Agent", user_agent).call() {
                Ok(mut response) => {
                    let status = response.status().as_u16() as i32;
                    let body = response.body_mut().read_to_string().unwrap_or_default();
                    let title = Self::extract_html_title(&body)
                        .unwrap_or_else(|| target_url.clone());

                    sender
                        .send(NetworkResponse::Success {
                            url: target_url,
                            title,
                            body,
                            status,
                            sec_level,
                        })
                        .ok();
                }
                Err(err) => {
                    error!("Erro ao carregar URL '{}': {}", target_url, err);
                    sender
                        .send(NetworkResponse::Error {
                            url: target_url,
                            message: err.to_string(),
                        })
                        .ok();
                }
            }
        });
    }

    fn render_html_content(
        &self,
        url: &str,
        title: &str,
        html_body: &str,
        status_code: i32,
        sec_level: SecurityLevel,
    ) {
        self.clear_content();
        self.progress_bar.set_fraction(1.0);
        self.progress_bar.set_visible(false);
        self.is_loading.store(false, Ordering::SeqCst);

        // Salvar no histórico persistente SQLite (apenas em modo normal, NUNCA em modo privado)
        if !self.is_private {
            self.storage.record_history_visit(url, title).ok();
        }

        // Container rolável do conteúdo web
        let scroll = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Automatic)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .build();

        let page_box = GtkBox::new(Orientation::Vertical, 16);
        page_box.set_margin_top(24);
        page_box.set_margin_bottom(24);
        page_box.set_margin_start(32);
        page_box.set_margin_end(32);

        // Barra informativa da página carregada
        let info_card = GtkBox::new(Orientation::Horizontal, 12);
        info_card.set_margin_bottom(8);
        info_card.add_css_class("card");

        let status_badge = Label::builder()
            .label(&format!("Status: HTTP {}", status_code))
            .css_classes(["caption", "bold"])
            .build();

        let url_badge = Label::builder()
            .label(url)
            .css_classes(["caption", "dim-label"])
            .hexpand(true)
            .halign(Align::Start)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();

        info_card.append(&status_badge);
        info_card.append(&url_badge);
        page_box.append(&info_card);

        // Renderização do texto limpo
        let text_buffer = TextBuffer::new(None::<&TextTagTable>);
        let clean_text = Self::strip_html_to_readable_text(html_body);
        text_buffer.set_text(&clean_text);

        let text_view = TextView::builder()
            .buffer(&text_buffer)
            .editable(false)
            .cursor_visible(false)
            .wrap_mode(gtk4::WrapMode::Word)
            .vexpand(true)
            .hexpand(true)
            .css_classes(["card"])
            .build();

        page_box.append(&text_view);
        scroll.set_child(Some(&page_box));
        self.content_area.append(&scroll);

        // Atualizar estado
        let mut state = self.current_state.borrow_mut();
        state.url = url.to_string();
        state.title = title.to_string();
        state.security_level = sec_level;
        state.status_code = Some(status_code);
        state.is_loading = false;
        state.progress = 1.0;
        self.update_can_go(&mut state);
        (self.on_state_changed)(&state);
    }

    fn render_error_page(&self, url: &str, error_msg: &str) {
        self.clear_content();
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

        self.content_area.append(&err_box);

        let mut state = self.current_state.borrow_mut();
        state.url = url.to_string();
        state.title = "Erro de Conexão".to_string();
        state.security_level = SecurityLevel::BrokenTls;
        state.is_loading = false;
        self.update_can_go(&mut state);
        (self.on_state_changed)(&state);
    }

    fn clear_content(&self) {
        while let Some(child) = self.content_area.first_child() {
            self.content_area.remove(&child);
        }
    }

    fn update_can_go(&self, state: &mut NavigationState) {
        let stack = self.history_stack.borrow();
        let idx = *self.history_index.borrow();
        state.can_go_back = idx > 0;
        state.can_go_forward = idx + 1 < stack.len();
    }

    pub fn go_back(self: &Rc<Self>) {
        let prev_url = {
            let stack = self.history_stack.borrow();
            let mut idx = self.history_index.borrow_mut();
            if *idx > 0 {
                *idx -= 1;
                Some(stack[*idx].clone())
            } else {
                None
            }
        };

        if let Some(url) = prev_url {
            if url.starts_with("brunya://") {
                self.render_internal_page(&url);
            } else {
                self.fetch_and_render_web_page(&url);
            }
        }
    }

    pub fn go_forward(self: &Rc<Self>) {
        let next_url = {
            let stack = self.history_stack.borrow();
            let mut idx = self.history_index.borrow_mut();
            if *idx + 1 < stack.len() {
                *idx += 1;
                Some(stack[*idx].clone())
            } else {
                None
            }
        };

        if let Some(url) = next_url {
            if url.starts_with("brunya://") {
                self.render_internal_page(&url);
            } else {
                self.fetch_and_render_web_page(&url);
            }
        }
    }

    pub fn reload(self: &Rc<Self>) {
        let url = self.current_state.borrow().url.clone();
        self.load_url(&url);
    }

    fn extract_html_title(html: &str) -> Option<String> {
        let lower = html.to_lowercase();
        let start_tag = "<title>";
        let end_tag = "</title>";

        let start_pos = lower.find(start_tag)? + start_tag.len();
        let end_pos = lower[start_pos..].find(end_tag)? + start_pos;

        let raw_title = &html[start_pos..end_pos];
        Some(raw_title.trim().replace('\n', " ").replace('\r', ""))
    }

    fn strip_html_to_readable_text(html: &str) -> String {
        let mut in_tag = false;
        let mut in_script = false;
        let mut in_style = false;
        let mut text = String::with_capacity(html.len() / 2);
        let mut lower_tracker = String::new();

        for ch in html.chars() {
            if ch == '<' {
                in_tag = true;
                lower_tracker.clear();
            } else if ch == '>' {
                in_tag = false;
                let tag_lower = lower_tracker.to_lowercase();
                if tag_lower.starts_with("script") {
                    in_script = true;
                } else if tag_lower.starts_with("/script") {
                    in_script = false;
                } else if tag_lower.starts_with("style") {
                    in_style = true;
                } else if tag_lower.starts_with("/style") {
                    in_style = false;
                } else if tag_lower == "p" || tag_lower == "/p" || tag_lower == "br" || tag_lower.starts_with("h") {
                    text.push('\n');
                }
            } else if in_tag {
                lower_tracker.push(ch);
            } else if !in_script && !in_style {
                text.push(ch);
            }
        }

        let mut cleaned = String::new();
        let mut was_whitespace = false;
        for c in text.chars() {
            if c.is_whitespace() {
                if c == '\n' {
                    cleaned.push('\n');
                    was_whitespace = false;
                } else if !was_whitespace {
                    cleaned.push(' ');
                    was_whitespace = true;
                }
            } else {
                cleaned.push(c);
                was_whitespace = false;
            }
        }

        cleaned
    }
}
