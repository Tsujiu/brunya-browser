use crate::engine::WebView;
use crate::ui::find_bar::FindBar;
use crate::ui::header::BrowserHeader;
use crate::ui::privacy_popover::PrivacyShieldPopover;
use crate::ui::tab_bar::BrowserTabBar;
use bear_adblock::AdblockEngine;
use bear_core::types::{TabId, UrlHelper};
use bear_i18n::t;
use bear_settings::SettingsManager;
use bear_storage::StorageManager;
use bear_tabs::TabManager;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, EventControllerKey, Orientation, Stack};
use libadwaita::Application;
use libadwaita::ApplicationWindow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

pub struct BearWindow {
    pub window: ApplicationWindow,
    header: Rc<BrowserHeader>,
    tab_bar: Rc<RefCell<BrowserTabBar>>,
    find_bar: Rc<FindBar>,
    tab_stack: Stack,
    tab_views: Rc<RefCell<HashMap<TabId, Rc<WebView>>>>,
    tab_manager: Arc<TabManager>,
    adblock_engine: Arc<AdblockEngine>,
    storage_manager: Arc<StorageManager>,
    settings_manager: Arc<SettingsManager>,
}

impl BearWindow {
    pub fn new(
        app: &Application,
        tab_manager: Arc<TabManager>,
        adblock_engine: Arc<AdblockEngine>,
        storage_manager: Arc<StorageManager>,
        settings_manager: Arc<SettingsManager>,
    ) -> Rc<Self> {
        let root_box = GtkBox::new(Orientation::Vertical, 0);

        // Header bar
        let header = Rc::new(BrowserHeader::new());
        root_box.append(&header.container);

        // Tab bar
        let tab_bar = Rc::new(RefCell::new(BrowserTabBar::new()));
        root_box.append(&tab_bar.borrow().container);

        // Barra de busca na página (Ctrl+F)
        let find_bar = FindBar::new();
        root_box.append(&find_bar.container);

        // Stack para visualização do conteúdo das abas
        let tab_stack = Stack::new();
        tab_stack.set_vexpand(true);
        tab_stack.set_hexpand(true);
        root_box.append(&tab_stack);

        let window = ApplicationWindow::builder()
            .application(app)
            .title(&t!("common.app_name"))
            .default_width(1200)
            .default_height(800)
            .content(&root_box)
            .build();

        let bear_window = Rc::new(Self {
            window,
            header,
            tab_bar,
            find_bar,
            tab_stack,
            tab_views: Rc::new(RefCell::new(HashMap::new())),
            tab_manager,
            adblock_engine,
            storage_manager,
            settings_manager,
        });

        bear_window.setup_event_handlers();
        bear_window.setup_shortcuts();

        // Abrir primeira aba
        bear_window.open_new_tab("bear://newtab", &t!("common.new_tab"));

        bear_window
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn navigate_active_tab(self: &Rc<Self>, url: &str) {
        if let Some(active) = self.tab_manager.active_tab() {
            if let Some(view) = self.tab_views.borrow().get(&active.id) {
                view.load_url(url);
            }
        }
    }

    pub fn open_new_tab(self: &Rc<Self>, url: &str, title: &str) {
        let tab_desc = self.tab_manager.open_tab(url, title);
        let tid = tab_desc.id;

        let this = Rc::downgrade(self);
        let on_state_changed: Rc<dyn Fn(&bear_core::types::NavigationState)> =
            Rc::new(move |state| {
                if let Some(win) = this.upgrade() {
                    win.on_active_tab_state_changed(tid, state);
                }
            });

        let view = WebView::new(
            tid,
            self.adblock_engine.clone(),
            self.storage_manager.clone(),
            self.settings_manager.clone(),
            on_state_changed,
        );

        if url != "bear://newtab" {
            view.load_url(url);
        }

        let tab_name = format!("tab_{}", tid.0);
        self.tab_stack.add_named(&view.container, Some(&tab_name));
        self.tab_views.borrow_mut().insert(tid, view);

        self.switch_to_tab(tid);
    }

    pub fn close_tab(self: &Rc<Self>, tab_id: TabId) {
        if let Some(view) = self.tab_views.borrow_mut().remove(&tab_id) {
            self.tab_stack.remove(&view.container);
        }

        self.tab_manager.close_tab(tab_id);

        if let Some(active) = self.tab_manager.active_tab() {
            self.switch_to_tab(active.id);
        } else {
            // Se fechou a última aba, abre uma nova aba limpa
            self.open_new_tab("bear://newtab", &t!("common.new_tab"));
        }
    }

    pub fn switch_to_tab(self: &Rc<Self>, tab_id: TabId) {
        if self.tab_manager.set_active_tab(tab_id) {
            let tab_name = format!("tab_{}", tab_id.0);
            self.tab_stack.set_visible_child_name(&tab_name);

            // Atualizar barra de abas
            self.refresh_tab_bar();

            // Atualizar barra de navegação com o estado desta aba
            if let Some(view) = self.tab_views.borrow().get(&tab_id) {
                let st = view.navigation_state();
                self.sync_header_with_state(&st);
            }
        }
    }

    fn refresh_tab_bar(self: &Rc<Self>) {
        let tabs = self.tab_manager.list_tabs();
        let active_id = self.tab_manager.active_tab().map(|t| t.id);

        let this_sw = self.clone();
        let on_switch = move |tid: TabId| {
            this_sw.switch_to_tab(tid);
        };

        let this_cl = self.clone();
        let on_close = move |tid: TabId| {
            this_cl.close_tab(tid);
        };

        let this_dup = self.clone();
        let on_dup = move |tid: TabId| {
            if let Some(tab) = this_dup.tab_manager.list_tabs().iter().find(|t| t.id == tid) {
                this_dup.open_new_tab(&tab.url, &format!("{} (Cópia)", tab.title));
            }
        };

        let this_pin = self.clone();
        let on_pin = move |tid: TabId| {
            this_pin.tab_manager.toggle_pin_tab(tid);
            this_pin.refresh_tab_bar();
        };

        self.tab_bar
            .borrow_mut()
            .render_tabs(&tabs, active_id, on_switch, on_close, on_dup, on_pin);
    }

    fn on_active_tab_state_changed(self: &Rc<Self>, tab_id: TabId, state: &bear_core::types::NavigationState) {
        // Atualizar metadados no gerenciador de abas
        self.tab_manager
            .update_tab_metadata(tab_id, &state.url, &state.title, state.favicon_url.clone());

        // Se a aba notificada for a aba atualmente ativa, atualiza a UI
        if let Some(active) = self.tab_manager.active_tab() {
            if active.id == tab_id {
                self.sync_header_with_state(state);
            }
        }

        self.refresh_tab_bar();
    }

    pub fn bookmark_current_page(self: &Rc<Self>) {
        if let Some(active) = self.tab_manager.active_tab() {
            if !active.url.is_empty() && active.url != "bear://newtab" {
                let title = if active.title.is_empty() { "Favorito" } else { &active.title };
                self.storage_manager.add_bookmark(&active.url, title, "").ok();
                self.header.bookmark_btn.set_icon_name("starred-symbolic");
            }
        }
    }

    pub fn duplicate_current_tab(self: &Rc<Self>) {
        if let Some(active) = self.tab_manager.active_tab() {
            self.open_new_tab(&active.url, &format!("{} (Cópia)", active.title));
        }
    }

    pub fn next_tab(self: &Rc<Self>) {
        let tabs = self.tab_manager.list_tabs();
        if tabs.len() <= 1 {
            return;
        }
        if let Some(active) = self.tab_manager.active_tab() {
            if let Some(pos) = tabs.iter().position(|t| t.id == active.id) {
                let next_idx = (pos + 1) % tabs.len();
                self.switch_to_tab(tabs[next_idx].id);
            }
        }
    }

    pub fn previous_tab(self: &Rc<Self>) {
        let tabs = self.tab_manager.list_tabs();
        if tabs.len() <= 1 {
            return;
        }
        if let Some(active) = self.tab_manager.active_tab() {
            if let Some(pos) = tabs.iter().position(|t| t.id == active.id) {
                let prev_idx = if pos == 0 { tabs.len() - 1 } else { pos - 1 };
                self.switch_to_tab(tabs[prev_idx].id);
            }
        }
    }

    pub fn open_downloads(self: &Rc<Self>) {
        self.open_new_tab("bear://downloads", &t!("downloads.title"));
    }

    pub fn open_bookmarks(self: &Rc<Self>) {
        self.open_new_tab("bear://bookmarks", &t!("common.bookmarks"));
    }

    fn sync_header_with_state(&self, state: &bear_core::types::NavigationState) {
        self.header.address_entry.set_text(&state.url);
        self.header.set_security_level(state.security_level);
        self.header.back_btn.set_sensitive(state.can_go_back);
        self.header.forward_btn.set_sensitive(state.can_go_forward);
        self.header.set_loading(state.is_loading);
        self.header.set_blocked_count(self.adblock_engine.total_blocked());

        let win_title = if state.title.is_empty() {
            t!("common.app_name")
        } else {
            format!("{} — {}", state.title, t!("common.app_name"))
        };
        self.window.set_title(Some(&win_title));
    }

    fn setup_event_handlers(self: &Rc<Self>) {
        // 1. Enter na barra de endereço
        let this_nav = self.clone();
        self.header.address_entry.connect_activate(move |entry| {
            let input = entry.text();
            let template = this_nav.settings_manager.get().search_engine.query_template;
            let target_url = UrlHelper::parse_input(&input, &template);

            if let Some(active) = this_nav.tab_manager.active_tab() {
                if let Some(view) = this_nav.tab_views.borrow().get(&active.id) {
                    view.load_url(&target_url);
                }
            }
        });

        // 2. Botão Voltar
        let this_back = self.clone();
        self.header.back_btn.connect_clicked(move |_| {
            if let Some(active) = this_back.tab_manager.active_tab() {
                if let Some(view) = this_back.tab_views.borrow().get(&active.id) {
                    view.go_back();
                }
            }
        });

        // 3. Botão Avançar
        let this_fwd = self.clone();
        self.header.forward_btn.connect_clicked(move |_| {
            if let Some(active) = this_fwd.tab_manager.active_tab() {
                if let Some(view) = this_fwd.tab_views.borrow().get(&active.id) {
                    view.go_forward();
                }
            }
        });

        // 4. Botão Recarregar
        let this_rel = self.clone();
        self.header.reload_btn.connect_clicked(move |_| {
            if let Some(active) = this_rel.tab_manager.active_tab() {
                if let Some(view) = this_rel.tab_views.borrow().get(&active.id) {
                    view.reload();
                }
            }
        });

        // 5. Botão Favorito
        let this_bm = self.clone();
        self.header.bookmark_btn.connect_clicked(move |_| {
            this_bm.bookmark_current_page();
        });

        // 6. Botão Downloads
        let this_dl = self.clone();
        self.header.downloads_btn.connect_clicked(move |_| {
            this_dl.open_downloads();
        });

        // 7. Botão Nova Aba
        let this_new = self.clone();
        self.header.new_tab_btn.connect_clicked(move |_| {
            this_new.open_new_tab("bear://newtab", &t!("common.new_tab"));
        });

        // 8. Botão Escudo de Privacidade
        let this_shield = self.clone();
        self.header.shield_btn.connect_clicked(move |btn| {
            let current_site = this_shield
                .tab_manager
                .active_tab()
                .map(|t| t.url)
                .unwrap_or_default();
            let popover = PrivacyShieldPopover::new(this_shield.adblock_engine.clone(), current_site);
            popover.popover.set_parent(btn);
            popover.popover.popup();
        });

        // 9. Menu Principal
        let menu_model = gio::Menu::new();
        menu_model.append(Some(&t!("common.new_tab")), Some("app.new_tab"));
        menu_model.append(Some(&t!("common.duplicate_tab")), Some("app.duplicate_tab"));
        menu_model.append(Some(&t!("common.bookmarks")), Some("app.open_bookmarks"));
        menu_model.append(Some(&t!("common.history")), Some("app.open_history"));
        menu_model.append(Some(&t!("common.downloads")), Some("app.open_downloads"));
        menu_model.append(Some(&t!("common.settings")), Some("app.open_settings"));
        menu_model.append(Some(&t!("common.about")), Some("app.open_about"));
        menu_model.append(Some(&t!("common.quit")), Some("app.quit"));
        self.header.menu_btn.set_menu_model(Some(&menu_model));
    }

    fn setup_shortcuts(self: &Rc<Self>) {
        let key_controller = EventControllerKey::new();

        let this_key = self.clone();
        key_controller.connect_key_pressed(move |_, keyval, _keycode, state| {
            if keyval == gdk4::Key::Escape && this_key.find_bar.container.is_visible() {
                this_key.find_bar.hide();
                return glib::Propagation::Stop;
            }

            let is_ctrl = state.contains(gdk4::ModifierType::CONTROL_MASK);
            let is_shift = state.contains(gdk4::ModifierType::SHIFT_MASK);

            if is_ctrl {
                match keyval {
                    gdk4::Key::t if !is_shift => {
                        this_key.open_new_tab("bear://newtab", &t!("common.new_tab"));
                        return glib::Propagation::Stop;
                    }
                    gdk4::Key::T if is_shift => {
                        if let Some(tab) = this_key.tab_manager.restore_last_closed_tab() {
                            this_key.open_new_tab(&tab.url, &tab.title);
                        }
                        return glib::Propagation::Stop;
                    }
                    gdk4::Key::w => {
                        if let Some(active) = this_key.tab_manager.active_tab() {
                            this_key.close_tab(active.id);
                        }
                        return glib::Propagation::Stop;
                    }
                    gdk4::Key::r => {
                        if let Some(active) = this_key.tab_manager.active_tab() {
                            if let Some(view) = this_key.tab_views.borrow().get(&active.id) {
                                view.reload();
                            }
                        }
                        return glib::Propagation::Stop;
                    }
                    gdk4::Key::l => {
                        this_key.header.address_entry.grab_focus();
                        this_key.header.address_entry.select_region(0, -1);
                        return glib::Propagation::Stop;
                    }
                    gdk4::Key::h => {
                        this_key.open_new_tab("bear://history", &t!("common.history"));
                        return glib::Propagation::Stop;
                    }
                    gdk4::Key::d => {
                        this_key.bookmark_current_page();
                        return glib::Propagation::Stop;
                    }
                    gdk4::Key::j => {
                        this_key.open_downloads();
                        return glib::Propagation::Stop;
                    }
                    gdk4::Key::b => {
                        this_key.open_bookmarks();
                        return glib::Propagation::Stop;
                    }
                    gdk4::Key::f => {
                        this_key.find_bar.toggle();
                        return glib::Propagation::Stop;
                    }
                    gdk4::Key::Tab if !is_shift => {
                        this_key.next_tab();
                        return glib::Propagation::Stop;
                    }
                    gdk4::Key::ISO_Left_Tab | gdk4::Key::Tab if is_shift => {
                        this_key.previous_tab();
                        return glib::Propagation::Stop;
                    }
                    _ => {}
                }
            }

            glib::Propagation::Proceed
        });

        self.window.add_controller(key_controller);
    }
}
