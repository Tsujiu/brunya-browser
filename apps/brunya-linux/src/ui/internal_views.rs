use crate::engine::extension_host::{extension_image, toolbar_icon};
use crate::engine::FaviconManager;
use brunya_extensions::loader::inspect_package;
use brunya_extensions::store::{download_store_package, parse_store_input};
use brunya_extensions::types::{ExtensionState, InstallType};
use brunya_extensions::ExtensionSystem;
use brunya_i18n::{t, I18nManager, Language};
use brunya_settings::{SettingsManager, ShortcutDisplayMode, ThemePreference};
use brunya_storage::StorageManager;
use gtk4::gio;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Entry, GestureClick, Image, Label, ListBox, Orientation, Popover,
    ScrolledWindow, SelectionMode, Switch,
};
use libadwaita::prelude::*;
use libadwaita::ActionRow;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// Constrói a página interna "brunya://newtab" inspirada no buscador Pesquisa.
pub fn build_newtab_view<F: Fn(&str) + 'static>(
    settings_mgr: Arc<SettingsManager>,
    storage_mgr: Arc<StorageManager>,
    on_navigate: F,
) -> gtk4::Widget {
    let main_box = GtkBox::new(Orientation::Vertical, 0);
    main_box.set_vexpand(true);
    main_box.set_hexpand(true);
    main_box.add_css_class("pesquisa-page");
    let on_nav_rc = Rc::new(on_navigate);

    // Container centralizado (Hero section)
    let hero = GtkBox::new(Orientation::Vertical, 16);
    hero.set_valign(Align::Center);
    hero.set_halign(Align::Center);
    hero.set_vexpand(true);
    hero.set_hexpand(true);
    hero.set_margin_start(24);
    hero.set_margin_end(24);
    hero.set_margin_top(48);
    hero.set_margin_bottom(32);

    // 1. Logo "Pesquisa." com suporte à tradução e a mesma fonte serifada do CSS
    let logo_text = t!("common.search_logo");
    let logo = Label::builder()
        .use_markup(true)
        .label(&format!(
            "<span font_family=\"Fraunces, Georgia, DejaVu Serif, serif\" size=\"64000\" weight=\"bold\">{}<span foreground=\"#38bdf8\">.</span></span>",
            logo_text
        ))
        .css_classes(["pesquisa-logo"])
        .build();

    // 2. Tagline com suporte a i18n
    let tagline = Label::builder()
        .label(&t!("common.search_tagline"))
        .css_classes(["pesquisa-tagline"])
        .wrap(true)
        .justify(gtk4::Justification::Center)
        .build();

    hero.append(&logo);
    hero.append(&tagline);

    // 3. Search Box Wrapper estilizado tipo cápsula com Seletor Discreto de Motores de Busca
    let search_box = GtkBox::new(Orientation::Horizontal, 8);
    search_box.set_halign(Align::Center);
    search_box.set_width_request(640);
    search_box.add_css_class("pesquisa-search-container");

    let search_icon = Image::from_icon_name("system-search-symbolic");
    search_icon.set_opacity(0.65);

    // Motor de busca ativo
    let current_engine = settings_mgr.get().search_engine;
    let selected_template = Rc::new(RefCell::new(current_engine.query_template));

    let engine_btn = Button::builder()
        .css_classes(["flat", "pill", "engine-picker-btn"])
        .tooltip_text("Selecionar motor de busca")
        .build();

    let engine_content = GtkBox::new(Orientation::Horizontal, 6);
    let engine_icon_box = GtkBox::new(Orientation::Horizontal, 0);

    let initial_engine_id = if current_engine.name.to_lowercase().contains("startpage") {
        "startpage"
    } else if current_engine.name.to_lowercase().contains("google") {
        "google"
    } else if current_engine.name.to_lowercase().contains("bing") {
        "bing"
    } else {
        "duckduckgo"
    };
    engine_icon_box.append(&crate::ui::engine_popover::get_engine_icon(
        initial_engine_id,
    ));

    let engine_arrow = Image::from_icon_name("pan-down-symbolic");
    engine_arrow.set_pixel_size(10);

    engine_content.append(&engine_icon_box);
    engine_content.append(&engine_arrow);
    engine_btn.set_child(Some(&engine_content));

    let initial_placeholder = format!(
        "{} {}...",
        t!("common.search_placeholder_prefix"),
        current_engine.name
    );
    let search_entry = Entry::builder()
        .placeholder_text(&initial_placeholder)
        .hexpand(true)
        .css_classes(["pesquisa-entry"])
        .build();

    let engine_icon_box_clone = engine_icon_box.clone();
    let entry_for_ph = search_entry.clone();
    let template_store = selected_template.clone();
    let sm_clone = settings_mgr.clone();

    engine_btn.connect_clicked(move |btn| {
        let icon_box_c = engine_icon_box_clone.clone();
        let entry_c = entry_for_ph.clone();
        let tmpl_c = template_store.clone();
        let sm_c = sm_clone.clone();

        let popover = crate::ui::engine_popover::SearchEnginePopover::new(move |engine| {
            while let Some(ch) = icon_box_c.first_child() {
                icon_box_c.remove(&ch);
            }
            icon_box_c.append(&crate::ui::engine_popover::get_engine_icon(engine.id));
            entry_c.set_placeholder_text(Some(&format!(
                "{} {}...",
                t!("common.search_placeholder_prefix"),
                engine.name
            )));
            *tmpl_c.borrow_mut() = engine.query_template.to_string();
            sm_c.update(|s| {
                s.search_engine = brunya_settings::SearchEngine {
                    name: engine.name.to_string(),
                    query_template: engine.query_template.to_string(),
                };
            })
            .ok();
        });
        popover.popover.set_parent(btn);
        popover.popover.popup();
    });

    let clear_btn = Button::builder()
        .icon_name("edit-clear-symbolic")
        .css_classes(["flat", "circular"])
        .tooltip_text("Limpar busca")
        .visible(false)
        .build();

    let entry_for_clear = search_entry.clone();
    let clear_btn_clone = clear_btn.clone();
    search_entry.connect_changed(move |e| {
        clear_btn_clone.set_visible(!e.text().is_empty());
    });

    clear_btn.connect_clicked(move |_| {
        entry_for_clear.set_text("");
        entry_for_clear.grab_focus();
    });

    search_box.append(&search_icon);
    search_box.append(&engine_btn);
    search_box.append(&search_entry);
    search_box.append(&clear_btn);
    hero.append(&search_box);

    // 4. Botão de Pesquisar (apenas ele, sem chips e sem textos de rodapé)
    let buttons_box = GtkBox::new(Orientation::Horizontal, 10);
    buttons_box.set_halign(Align::Center);
    buttons_box.set_margin_top(16);

    let search_btn = Button::builder()
        .label(&t!("common.search_button"))
        .css_classes(["pesquisa-btn-primary"])
        .build();

    let on_nav_clone = on_nav_rc.clone();
    let nav_click = on_nav_clone.clone();
    let entry_clone = search_entry.clone();
    let tmpl_for_click = selected_template.clone();
    search_btn.connect_clicked(move |_| {
        let text = entry_clone.text().trim().to_string();
        if !text.is_empty() {
            let tmpl = tmpl_for_click.borrow().clone();
            let target = brunya_core::types::UrlHelper::parse_input(&text, &tmpl);
            nav_click(&target);
        }
    });

    let nav_enter = on_nav_clone.clone();
    let tmpl_for_enter = selected_template.clone();
    search_entry.connect_activate(move |entry| {
        let text = entry.text().trim().to_string();
        if !text.is_empty() {
            let tmpl = tmpl_for_enter.borrow().clone();
            let target = brunya_core::types::UrlHelper::parse_input(&text, &tmpl);
            nav_enter(&target);
        }
    });

    buttons_box.append(&search_btn);
    hero.append(&buttons_box);

    // ── Shortcuts / Favoritos na página inicial ──
    let shortcuts_box = GtkBox::new(Orientation::Vertical, 12);
    shortcuts_box.set_halign(Align::Center);
    shortcuts_box.set_margin_top(32);
    shortcuts_box.set_vexpand(true);

    let grid = GtkBox::new(Orientation::Horizontal, 16);
    grid.set_halign(Align::Center);
    shortcuts_box.append(&grid);

    // Helper: rebuild shortcut cards (clones Arcs and calls itself recursively via Rc)
    let rebuild_shortcuts: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

    let rebuild_fn = {
        let grid = grid.clone();
        let storage = storage_mgr.clone();
        let settings = settings_mgr.clone();
        let on_nav = on_nav_rc.clone();
        let rebuild_ptr = rebuild_shortcuts.clone();

        move || {
            // Clear grid
            while let Some(child) = grid.first_child() {
                grid.remove(&child);
            }

            let show = settings.get().show_shortcuts_on_newtab;
            if !show {
                return;
            }
            let shortcut_mode = settings.get().shortcut_display_mode;
            let icon_only = matches!(shortcut_mode, ShortcutDisplayMode::IconOnly);

            let bookmarks = storage.list_bookmarks().unwrap_or_default();
            let shortcuts = bookmarks.iter().take(12);

            // Filter to only show shortcuts from the "shortcuts" folder, or all if none
            let scoped: Vec<_> = shortcuts
                .filter(|b| b.folder == "shortcuts" || b.folder.is_empty())
                .collect();

            for bm in &scoped {
                let card = GtkBox::new(Orientation::Vertical, 6);
                card.set_halign(Align::Center);
                card.add_css_class("shortcut-card");
                card.set_tooltip_text(Some(&bm.title));

                // Ícone grande — usa Picture para escalar corretamente
                let icon_container = GtkBox::new(Orientation::Vertical, 0);
                icon_container.set_size_request(72, 72);
                icon_container.set_halign(Align::Center);
                icon_container.set_valign(Align::Center);
                icon_container.add_css_class("shortcut-icon-frame");
                icon_container.set_margin_top(if icon_only { 10 } else { 12 });
                if icon_only {
                    icon_container.set_margin_bottom(10);
                }

                let favicon_img = Image::from_icon_name("web-browser-symbolic");
                favicon_img.set_pixel_size(40);
                favicon_img.set_halign(Align::Center);
                favicon_img.set_valign(Align::Center);

                // Custom icon URL overrides auto-fetch
                let custom_icon_url = bm.custom_icon.as_deref();
                if let Some(custom_url) = custom_icon_url {
                    if !custom_url.is_empty() {
                        // Try loading from custom URL via FaviconManager domain
                        if let Some(cdomain) = FaviconManager::extract_domain(custom_url) {
                            if let Some(tex) = FaviconManager::load_cached_favicon(&cdomain) {
                                favicon_img.set_paintable(Some(&tex));
                            }
                        }
                    }
                } else {
                    // Tenta carregar favicon real do cache
                    if let Some(domain) = FaviconManager::extract_domain(&bm.url) {
                        if let Some(texture) = FaviconManager::load_cached_favicon(&domain) {
                            favicon_img.set_paintable(Some(&texture));
                        } else {
                            // Async fetch
                            let favicon_clone = favicon_img.clone();
                            FaviconManager::fetch_favicon_async(domain, move |tex| {
                                favicon_clone.set_paintable(Some(&tex));
                            });
                        }
                    }
                }

                icon_container.append(&favicon_img);

                card.append(&icon_container);

                if !icon_only {
                    let label = Label::builder()
                        .label(&bm.title)
                        .css_classes(["shortcut-label"])
                        .max_width_chars(12)
                        .ellipsize(gtk4::pango::EllipsizeMode::End)
                        .build();
                    card.append(&label);
                }

                let nav_c = on_nav.clone();
                let url_c = bm.url.clone();
                let g = GestureClick::new();
                g.connect_pressed(move |_, _, _, _| nav_c(&url_c));
                card.add_controller(g);

                grid.append(&card);
            }

            // Card "+"
            let add_card = GtkBox::new(Orientation::Vertical, 6);
            add_card.set_halign(Align::Center);
            add_card.add_css_class("shortcut-card");
            add_card.add_css_class("shortcut-add-card");

            let add_icon_frame = GtkBox::new(Orientation::Vertical, 0);
            add_icon_frame.set_size_request(72, 72);
            add_icon_frame.set_halign(Align::Center);
            add_icon_frame.set_valign(Align::Center);
            add_icon_frame.add_css_class("shortcut-icon-frame");
            add_icon_frame.set_margin_top(if icon_only { 10 } else { 12 });
            if icon_only {
                add_icon_frame.set_margin_bottom(10);
            }

            let add_icon = Label::builder()
                .label("+")
                .css_classes(["shortcut-add-icon"])
                .build();
            let add_label = Label::builder()
                .label("Adicionar")
                .css_classes(["shortcut-label"])
                .build();
            add_icon_frame.append(&add_icon);

            add_card.append(&add_icon_frame);
            if !icon_only {
                add_card.append(&add_label);
            }

            let popover = Popover::builder()
                .has_arrow(true)
                .position(gtk4::PositionType::Top)
                .build();
            let pop_content = GtkBox::new(Orientation::Vertical, 10);
            pop_content.set_margin_top(14);
            pop_content.set_margin_bottom(14);
            pop_content.set_margin_start(14);
            pop_content.set_margin_end(14);

            let title_e = Entry::builder()
                .placeholder_text("Nome do atalho")
                .hexpand(true)
                .css_classes(["settings-custom-entry"])
                .build();
            let url_e = Entry::builder()
                .placeholder_text("https://exemplo.com")
                .hexpand(true)
                .css_classes(["settings-custom-entry"])
                .build();
            let icon_e = Entry::builder()
                .placeholder_text("URL do Ícone (opcional)")
                .hexpand(true)
                .css_classes(["settings-custom-entry"])
                .build();
            let save_btn = Button::builder()
                .label("Salvar")
                .css_classes(["settings-pill"])
                .build();

            pop_content.append(&title_e);
            pop_content.append(&url_e);
            pop_content.append(&icon_e);
            pop_content.append(&save_btn);
            popover.set_child(Some(&pop_content));

            let title_ec = title_e.clone();
            let url_ec = url_e.clone();
            let icon_ec = icon_e.clone();
            let st_c = storage.clone();
            let rb = rebuild_ptr.clone();
            save_btn.connect_clicked(move |_| {
                let title = title_ec.text().trim().to_string();
                let url = url_ec.text().trim().to_string();
                let icon_url = icon_ec.text().trim().to_string();
                if !title.is_empty() && !url.is_empty() {
                    let final_url = if url.starts_with("http://")
                        || url.starts_with("https://")
                        || url.starts_with("brunya://")
                    {
                        url
                    } else {
                        format!("https://{}", url)
                    };
                    let icon_opt = if icon_url.is_empty() {
                        None
                    } else {
                        Some(icon_url.as_str())
                    };
                    let _ = st_c.add_bookmark_with_icon(&final_url, &title, "shortcuts", icon_opt);
                    if let Some(ref f) = *rb.borrow() {
                        f();
                    }
                }
            });

            let popover_c = popover.clone();
            let ag = GestureClick::new();
            ag.connect_pressed(move |_, _, _, _| popover_c.popup());
            add_card.add_controller(ag);
            popover.set_parent(&add_card);

            grid.append(&add_card);
        }
    };

    *rebuild_shortcuts.borrow_mut() = Some(Box::new(rebuild_fn));

    // Initial build
    if let Some(ref f) = *rebuild_shortcuts.borrow() {
        f();
    }

    main_box.append(&shortcuts_box);

    main_box.append(&hero);

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .css_classes(["pesquisa-page"])
        .child(&main_box)
        .build();

    scroll.upcast()
}

/// Constrói a página interna "brunya://settings" — COMPLETAMENTE REDESIGNADA.
pub fn build_settings_view<F: Fn(Language) + 'static>(
    settings_mgr: Arc<SettingsManager>,
    storage_mgr: Arc<StorageManager>,
    on_language_changed: F,
) -> gtk4::Widget {
    let outer_box = GtkBox::new(Orientation::Vertical, 0);
    outer_box.set_vexpand(true);
    outer_box.set_hexpand(true);
    outer_box.add_css_class("settings-page");

    // ── HEADER ──
    let header_box = GtkBox::new(Orientation::Vertical, 2);
    header_box.set_halign(Align::Start);
    header_box.set_margin_start(40);
    header_box.set_margin_end(40);
    header_box.set_margin_top(32);
    header_box.set_margin_bottom(8);
    header_box.add_css_class("settings-page-header");

    let title_box = GtkBox::new(Orientation::Horizontal, 8);
    let title_icon = Image::from_icon_name("emblem-system-symbolic");
    title_icon.set_pixel_size(26);
    let title_lbl = Label::builder()
        .label("Configurações")
        .css_classes(["settings-title"])
        .build();
    title_box.append(&title_icon);
    title_box.append(&title_lbl);
    let subtitle_lbl = Label::builder()
        .label(&t!("settings.page_subtitle"))
        .css_classes(["settings-subtitle"])
        .halign(Align::Start)
        .build();

    header_box.append(&title_box);
    header_box.append(&subtitle_lbl);
    outer_box.append(&header_box);

    let on_lang_rc = Rc::new(on_language_changed);

    // ── HELPERS ──
    // Cria um título de seção com ícone
    let make_section_title_with_icon = |icon_name: &str, text: &str| -> GtkBox {
        let box_ = GtkBox::new(Orientation::Horizontal, 8);
        let icon = Image::from_icon_name(icon_name);
        icon.set_pixel_size(18);
        let lbl = Label::builder()
            .label(text)
            .css_classes(["settings-card-title"])
            .halign(Align::Start)
            .build();
        box_.append(&icon);
        box_.append(&lbl);
        box_
    };
    let make_section_subtitle = |text: &str| -> Label {
        Label::builder()
            .label(text)
            .css_classes(["settings-card-subtitle"])
            .halign(Align::Start)
            .build()
    };
    // Cria uma label de linha de config
    let make_row_label = |text: &str| -> Label {
        Label::builder()
            .label(text)
            .css_classes(["settings-row-label"])
            .halign(Align::Start)
            .hexpand(true)
            .build()
    };
    let make_row_desc = |text: &str| -> Label {
        Label::builder()
            .label(text)
            .css_classes(["settings-row-desc"])
            .halign(Align::Start)
            .build()
    };

    let current_theme = settings_mgr.get().theme;

    // ════════════════════════════════════════════
    //  CARD 1: APARÊNCIA (Idioma + Tema visual)
    // ════════════════════════════════════════════
    let card1 = GtkBox::new(Orientation::Vertical, 0);
    card1.add_css_class("settings-card");

    card1.append(&make_section_title_with_icon(
        "preferences-desktop-theme-symbolic",
        &t!("settings.appearance"),
    ));
    card1.append(&make_section_subtitle(
        "Idioma, tema visual e estilos do navegador",
    ));

    // ── Linha: Idioma ──
    let lang_row_box = GtkBox::new(Orientation::Horizontal, 0);
    lang_row_box.add_css_class("settings-row");
    lang_row_box.set_hexpand(true);

    let lang_info = GtkBox::new(Orientation::Vertical, 0);
    lang_info.set_hexpand(true);
    lang_info.append(&make_row_label(&t!("settings.language")));
    lang_info.append(&make_row_desc(&t!("settings.language_desc")));

    let lang_btns = GtkBox::new(Orientation::Horizontal, 6);
    let pt_btn = Button::builder()
        .label("🇧🇷  Português")
        .css_classes(["settings-pill"])
        .build();
    let en_btn = Button::builder()
        .label("🇺🇸  English")
        .css_classes(["settings-pill"])
        .build();

    // Marcar o idioma ativo
    let current_lang = I18nManager::global().current_language();
    match current_lang {
        Language::PtBr => pt_btn.add_css_class("active"),
        Language::En => en_btn.add_css_class("active"),
    }

    let lang_cb_pt = on_lang_rc.clone();
    let s_mgr_pt = settings_mgr.clone();
    let pt_btn_c = pt_btn.clone();
    let en_btn_c = en_btn.clone();
    pt_btn.connect_clicked(move |_| {
        I18nManager::global().set_language(Language::PtBr);
        s_mgr_pt.update(|s| s.language = "pt-BR".to_string()).ok();
        pt_btn_c.add_css_class("active");
        en_btn_c.remove_css_class("active");
        lang_cb_pt(Language::PtBr);
    });

    let lang_cb_en = on_lang_rc.clone();
    let s_mgr_en = settings_mgr.clone();
    let pt_btn_c2 = pt_btn.clone();
    let en_btn_c2 = en_btn.clone();
    en_btn.connect_clicked(move |_| {
        I18nManager::global().set_language(Language::En);
        s_mgr_en.update(|s| s.language = "en".to_string()).ok();
        en_btn_c2.add_css_class("active");
        pt_btn_c2.remove_css_class("active");
        lang_cb_en(Language::En);
    });

    lang_btns.append(&pt_btn);
    lang_btns.append(&en_btn);
    lang_row_box.append(&lang_info);
    lang_row_box.append(&lang_btns);
    card1.append(&lang_row_box);

    // ── Linha: Tema visual com cards ──
    let theme_row_box = GtkBox::new(Orientation::Vertical, 0);
    theme_row_box.add_css_class("settings-row");
    theme_row_box.set_hexpand(true);

    let theme_label = make_row_label(&t!("settings.theme"));
    theme_row_box.append(&theme_label);
    theme_row_box.append(&make_row_desc(&t!("settings.theme_desc")));

    // Seletor visual de tema (3 cards)
    let theme_selector = GtkBox::new(Orientation::Horizontal, 10);
    theme_selector.set_margin_top(10);
    theme_selector.add_css_class("theme-selector");
    theme_selector.set_halign(Align::Start);

    // Card: Sistema
    let sys_card = GtkBox::new(Orientation::Vertical, 2);
    sys_card.set_halign(Align::Center);
    sys_card.add_css_class("theme-card");
    let sys_icon = Image::from_icon_name("computer-symbolic");
    sys_icon.set_pixel_size(24);
    sys_icon.add_css_class("theme-card-icon");
    let sys_label = Label::builder()
        .label(&t!("settings.theme_auto"))
        .css_classes(["theme-card-label"])
        .build();
    sys_card.append(&sys_icon);
    sys_card.append(&sys_label);

    // Card: Claro
    let light_card = GtkBox::new(Orientation::Vertical, 2);
    light_card.set_halign(Align::Center);
    light_card.add_css_class("theme-card");
    let light_icon = Image::from_icon_name("weather-clear-symbolic");
    light_icon.set_pixel_size(24);
    light_icon.add_css_class("theme-card-icon");
    let light_label = Label::builder()
        .label(&t!("settings.theme_light"))
        .css_classes(["theme-card-label"])
        .build();
    light_card.append(&light_icon);
    light_card.append(&light_label);

    // Card: Escuro
    let dark_card = GtkBox::new(Orientation::Vertical, 2);
    dark_card.set_halign(Align::Center);
    dark_card.add_css_class("theme-card");
    let dark_icon = Image::from_icon_name("weather-clear-night-symbolic");
    dark_icon.set_pixel_size(24);
    dark_icon.add_css_class("theme-card-icon");
    let dark_label = Label::builder()
        .label(&t!("settings.theme_dark"))
        .css_classes(["theme-card-label"])
        .build();
    dark_card.append(&dark_icon);
    dark_card.append(&dark_label);

    // Marcar tema ativo
    match current_theme {
        ThemePreference::System => sys_card.add_css_class("active"),
        ThemePreference::Light => light_card.add_css_class("active"),
        ThemePreference::Dark => dark_card.add_css_class("active"),
    }

    // GestureClick para os cards de tema (Box não tem connect_clicked)
    let s_mgr_t1 = settings_mgr.clone();
    let sys_card_c = sys_card.clone();
    let light_card_c = light_card.clone();
    let dark_card_c = dark_card.clone();
    let sys_gesture = GestureClick::new();
    sys_gesture.connect_pressed(move |_, _, _, _| {
        libadwaita::StyleManager::default().set_color_scheme(libadwaita::ColorScheme::Default);
        s_mgr_t1.update(|s| s.theme = ThemePreference::System).ok();
        sys_card_c.add_css_class("active");
        light_card_c.remove_css_class("active");
        dark_card_c.remove_css_class("active");
    });
    sys_card.add_controller(sys_gesture);

    let s_mgr_t2 = settings_mgr.clone();
    let light_card_c2 = light_card.clone();
    let sys_card_c2 = sys_card.clone();
    let dark_card_c2 = dark_card.clone();
    let light_gesture = GestureClick::new();
    light_gesture.connect_pressed(move |_, _, _, _| {
        libadwaita::StyleManager::default().set_color_scheme(libadwaita::ColorScheme::ForceLight);
        s_mgr_t2.update(|s| s.theme = ThemePreference::Light).ok();
        light_card_c2.add_css_class("active");
        sys_card_c2.remove_css_class("active");
        dark_card_c2.remove_css_class("active");
    });
    light_card.add_controller(light_gesture);

    let s_mgr_t3 = settings_mgr.clone();
    let dark_card_c3 = dark_card.clone();
    let sys_card_c3 = sys_card.clone();
    let light_card_c3 = light_card.clone();
    let dark_gesture = GestureClick::new();
    dark_gesture.connect_pressed(move |_, _, _, _| {
        libadwaita::StyleManager::default().set_color_scheme(libadwaita::ColorScheme::ForceDark);
        s_mgr_t3.update(|s| s.theme = ThemePreference::Dark).ok();
        dark_card_c3.add_css_class("active");
        sys_card_c3.remove_css_class("active");
        light_card_c3.remove_css_class("active");
    });
    dark_card.add_controller(dark_gesture);

    theme_selector.append(&sys_card);
    theme_selector.append(&light_card);
    theme_selector.append(&dark_card);
    theme_row_box.append(&theme_selector);
    card1.append(&theme_row_box);

    outer_box.append(&card1);

    // ════════════════════════════════════════════
    //  CARD 2: DESEMPENHO (Suspensão de abas)
    // ════════════════════════════════════════════
    let card2 = GtkBox::new(Orientation::Vertical, 0);
    card2.add_css_class("settings-card");

    card2.append(&make_section_title_with_icon(
        "computer-symbolic",
        &t!("settings.performance"),
    ));
    card2.append(&make_section_subtitle(&t!("settings.tab_suspension_desc")));

    let susp_row = GtkBox::new(Orientation::Horizontal, 0);
    susp_row.add_css_class("settings-row");
    susp_row.set_hexpand(true);

    let susp_info = GtkBox::new(Orientation::Vertical, 0);
    susp_info.set_hexpand(true);
    susp_info.append(&make_row_label(&t!("settings.tab_suspension")));
    susp_info.append(&make_row_desc(&t!("settings.tab_suspension_short_desc")));

    let susp_switch = Switch::builder()
        .active(settings_mgr.get().tab_suspension_enabled)
        .build();
    susp_switch.set_valign(Align::Center);

    let s_mgr_susp = settings_mgr.clone();
    susp_switch.connect_active_notify(move |sw| {
        let val = sw.is_active();
        s_mgr_susp.update(|s| s.tab_suspension_enabled = val).ok();
    });

    susp_row.append(&susp_info);
    susp_row.append(&susp_switch);
    card2.append(&susp_row);

    // ── Atalhos na página inicial ──
    let shortcuts_row = GtkBox::new(Orientation::Horizontal, 0);
    shortcuts_row.add_css_class("settings-row");
    shortcuts_row.set_hexpand(true);

    let shortcuts_info = GtkBox::new(Orientation::Vertical, 0);
    shortcuts_info.set_hexpand(true);
    shortcuts_info.append(&make_row_label(&t!("settings.show_shortcuts")));
    shortcuts_info.append(&make_row_desc(&t!("settings.show_shortcuts_desc")));

    let short_switch = Switch::builder()
        .active(settings_mgr.get().show_shortcuts_on_newtab)
        .build();
    short_switch.set_valign(Align::Center);

    let s_mgr_short = settings_mgr.clone();
    short_switch.connect_active_notify(move |sw| {
        let val = sw.is_active();
        s_mgr_short
            .update(|s| s.show_shortcuts_on_newtab = val)
            .ok();
    });

    shortcuts_row.append(&shortcuts_info);
    shortcuts_row.append(&short_switch);
    card2.append(&shortcuts_row);

    // ── Modo de exibição dos atalhos ──
    let display_mode_row = GtkBox::new(Orientation::Vertical, 0);
    display_mode_row.add_css_class("settings-row");
    display_mode_row.set_hexpand(true);

    display_mode_row.append(&make_row_label(&t!("settings.shortcut_display_mode")));
    display_mode_row.append(&make_row_desc(&t!("settings.shortcut_display_mode_desc")));

    let mode_btn_box = GtkBox::new(Orientation::Horizontal, 8);
    mode_btn_box.set_margin_top(10);

    let icon_label_btn = Button::builder()
        .label(&t!("settings.shortcut_icon_label"))
        .css_classes(["settings-pill"])
        .build();
    let icon_only_btn = Button::builder()
        .label(&t!("settings.shortcut_icon_only"))
        .css_classes(["settings-pill"])
        .build();

    // Marcar modo ativo
    let current_mode = settings_mgr.get().shortcut_display_mode;
    match current_mode {
        ShortcutDisplayMode::IconAndLabel => icon_label_btn.add_css_class("active"),
        ShortcutDisplayMode::IconOnly => icon_only_btn.add_css_class("active"),
    }

    let all_mode_btns: Vec<Button> = vec![icon_label_btn.clone(), icon_only_btn.clone()];

    let s_mgr_mlbl = settings_mgr.clone();
    let all_mb1 = all_mode_btns.clone();
    let icon_label_btn_c = icon_label_btn.clone();
    icon_label_btn.connect_clicked(move |_| {
        s_mgr_mlbl
            .update(|s| s.shortcut_display_mode = ShortcutDisplayMode::IconAndLabel)
            .ok();
        for b in &all_mb1 {
            b.remove_css_class("active");
        }
        icon_label_btn_c.add_css_class("active");
    });

    let s_mgr_mico = settings_mgr.clone();
    let all_mb2 = all_mode_btns.clone();
    let icon_only_btn_c = icon_only_btn.clone();
    icon_only_btn.connect_clicked(move |_| {
        s_mgr_mico
            .update(|s| s.shortcut_display_mode = ShortcutDisplayMode::IconOnly)
            .ok();
        for b in &all_mb2 {
            b.remove_css_class("active");
        }
        icon_only_btn_c.add_css_class("active");
    });

    mode_btn_box.append(&icon_label_btn);
    mode_btn_box.append(&icon_only_btn);
    display_mode_row.append(&mode_btn_box);
    card2.append(&display_mode_row);

    outer_box.append(&card2);

    // ════════════════════════════════════════════
    //  CARD 2.5: GERENCIAR ATALHOS
    // ════════════════════════════════════════════
    let card_short_mgr = GtkBox::new(Orientation::Vertical, 0);
    card_short_mgr.add_css_class("settings-card");

    card_short_mgr.append(&make_section_title_with_icon(
        "bookmarks-symbolic",
        &t!("settings.manage_shortcuts"),
    ));
    card_short_mgr.append(&make_section_subtitle(&t!(
        "settings.manage_shortcuts_desc"
    )));

    let shortcuts_list_box = GtkBox::new(Orientation::Vertical, 6);
    shortcuts_list_box.set_margin_top(10);

    // Helper to rebuild the management list
    let rebuild_mgr_list: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

    let rebuild_mgr_fn = {
        let storage_mgr_c = storage_mgr.clone();
        let list_box_c = shortcuts_list_box.clone();
        let rebuild_ptr = rebuild_mgr_list.clone();

        move || {
            while let Some(child) = list_box_c.first_child() {
                list_box_c.remove(&child);
            }

            let bookmarks = storage_mgr_c.list_bookmarks().unwrap_or_default();
            let scoped: Vec<_> = bookmarks
                .iter()
                .filter(|b| b.folder == "shortcuts")
                .cloned()
                .collect();

            if scoped.is_empty() {
                let empty_lbl = Label::builder()
                    .label("Nenhum atalho adicionado ainda.")
                    .css_classes(["shortcut-label"])
                    .halign(Align::Start)
                    .margin_top(6)
                    .build();
                list_box_c.append(&empty_lbl);
                return;
            }

            for bm in &scoped {
                let item_row = GtkBox::new(Orientation::Horizontal, 10);
                item_row.add_css_class("settings-row");
                item_row.set_hexpand(true);

                // Favicon mini
                let mini_icon = Image::from_icon_name("web-browser-symbolic");
                mini_icon.set_pixel_size(20);

                // Title + URL
                let info_col = GtkBox::new(Orientation::Vertical, 0);
                info_col.set_hexpand(true);
                let name_lbl = Label::builder()
                    .label(&bm.title)
                    .css_classes(["settings-row-label"])
                    .halign(Align::Start)
                    .build();
                let url_lbl = Label::builder()
                    .label(&bm.url)
                    .css_classes(["shortcut-label"])
                    .halign(Align::Start)
                    .max_width_chars(40)
                    .ellipsize(gtk4::pango::EllipsizeMode::End)
                    .build();
                info_col.append(&name_lbl);
                info_col.append(&url_lbl);

                // ── Edit Button → popover ──
                let edit_btn = Button::builder()
                    .label(&t!("settings.shortcut_edit"))
                    .css_classes(["settings-pill"])
                    .build();

                let popover = Popover::builder()
                    .has_arrow(true)
                    .position(gtk4::PositionType::Left)
                    .build();
                let pop_content = GtkBox::new(Orientation::Vertical, 10);
                pop_content.set_margin_top(14);
                pop_content.set_margin_bottom(14);
                pop_content.set_margin_start(14);
                pop_content.set_margin_end(14);

                let ed_title = Entry::builder()
                    .text(&bm.title)
                    .placeholder_text(&t!("settings.shortcut_name"))
                    .hexpand(true)
                    .css_classes(["settings-custom-entry"])
                    .build();
                let ed_url = Entry::builder()
                    .text(&bm.url)
                    .placeholder_text(&t!("settings.shortcut_url"))
                    .hexpand(true)
                    .css_classes(["settings-custom-entry"])
                    .build();
                let ed_icon = Entry::builder()
                    .text(bm.custom_icon.as_deref().unwrap_or(""))
                    .placeholder_text(&t!("settings.shortcut_icon_url"))
                    .hexpand(true)
                    .css_classes(["settings-custom-entry"])
                    .build();
                let save_edit_btn = Button::builder()
                    .label(&t!("settings.shortcut_save"))
                    .css_classes(["settings-pill"])
                    .build();

                pop_content.append(&ed_title);
                pop_content.append(&ed_url);
                pop_content.append(&ed_icon);
                pop_content.append(&save_edit_btn);
                popover.set_child(Some(&pop_content));

                let bm_id = bm.id;
                let st_mgr_edit = storage_mgr_c.clone();
                let rebuilt_ptr_e = rebuild_ptr.clone();
                let popover_c = popover.clone();
                save_edit_btn.connect_clicked(move |_| {
                    let title = ed_title.text().trim().to_string();
                    let url = ed_url.text().trim().to_string();
                    let icon = ed_icon.text().trim().to_string();
                    if !title.is_empty() && !url.is_empty() {
                        let clean_url = if url.starts_with("http://")
                            || url.starts_with("https://")
                            || url.starts_with("brunya://")
                        {
                            url
                        } else {
                            format!("https://{}", url)
                        };
                        let icon_opt = if icon.is_empty() {
                            None
                        } else {
                            Some(icon.as_str())
                        };
                        st_mgr_edit
                            .update_bookmark(bm_id, &clean_url, &title, icon_opt)
                            .ok();
                        popover_c.popdown();
                        if let Some(ref f) = *rebuilt_ptr_e.borrow() {
                            f();
                        }
                    }
                });

                let popover_c2 = popover.clone();
                edit_btn.connect_clicked(move |_| {
                    popover_c2.popup();
                });
                popover.set_parent(&edit_btn);

                // ── Delete Button → confirmation ──
                let del_btn = Button::builder()
                    .label(&t!("settings.shortcut_delete"))
                    .css_classes(["settings-pill", "destructive"])
                    .build();

                let st_mgr_del = storage_mgr_c.clone();
                let bm_id_del = bm.id;
                let rebuilt_ptr_d = rebuild_ptr.clone();
                del_btn.connect_clicked(move |_| {
                    st_mgr_del.delete_bookmark(bm_id_del).ok();
                    if let Some(ref f) = *rebuilt_ptr_d.borrow() {
                        f();
                    }
                });

                item_row.append(&mini_icon);
                item_row.append(&info_col);
                item_row.append(&edit_btn);
                item_row.append(&del_btn);

                list_box_c.append(&item_row);
            }
        }
    };

    *rebuild_mgr_list.borrow_mut() = Some(Box::new(rebuild_mgr_fn));

    if let Some(ref f) = *rebuild_mgr_list.borrow() {
        f();
    }

    let scroll_list = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .min_content_height(200)
        .hexpand(true)
        .child(&shortcuts_list_box)
        .build();
    card_short_mgr.append(&scroll_list);

    outer_box.append(&card_short_mgr);

    // ════════════════════════════════════════════
    //  CARD 3: PRIVACIDADE E SEGURANÇA
    // ════════════════════════════════════════════
    let card3 = GtkBox::new(Orientation::Vertical, 0);
    card3.add_css_class("settings-card");

    card3.append(&make_section_title_with_icon(
        "security-high-symbolic",
        &t!("settings.privacy_and_security"),
    ));
    card3.append(&make_section_subtitle(
        "Controles de proteção de dados e privacidade",
    ));

    // ── Cookies de terceiros ──
    let cookie_row = GtkBox::new(Orientation::Horizontal, 0);
    cookie_row.add_css_class("settings-row");
    cookie_row.set_hexpand(true);

    let cookie_info = GtkBox::new(Orientation::Vertical, 0);
    cookie_info.set_hexpand(true);
    cookie_info.append(&make_row_label(&t!("privacy.cookies_blocked")));
    cookie_info.append(&make_row_desc(&t!("settings.cookies_blocked_desc")));

    let cookie_switch = Switch::builder()
        .active(settings_mgr.get().block_third_party_cookies)
        .build();
    cookie_switch.set_valign(Align::Center);

    let s_mgr_ck = settings_mgr.clone();
    cookie_switch.connect_active_notify(move |sw| {
        let val = sw.is_active();
        s_mgr_ck.update(|s| s.block_third_party_cookies = val).ok();
    });

    cookie_row.append(&cookie_info);
    cookie_row.append(&cookie_switch);
    card3.append(&cookie_row);

    // ── Do Not Track ──
    let dnt_row = GtkBox::new(Orientation::Horizontal, 0);
    dnt_row.add_css_class("settings-row");
    dnt_row.set_hexpand(true);

    let dnt_info = GtkBox::new(Orientation::Vertical, 0);
    dnt_info.set_hexpand(true);
    dnt_info.append(&make_row_label(&t!("settings.do_not_track")));
    dnt_info.append(&make_row_desc(&t!("settings.do_not_track_desc")));

    let dnt_switch = Switch::builder()
        .active(settings_mgr.get().do_not_track)
        .build();
    dnt_switch.set_valign(Align::Center);

    let s_mgr_dnt = settings_mgr.clone();
    dnt_switch.connect_active_notify(move |sw| {
        let val = sw.is_active();
        s_mgr_dnt.update(|s| s.do_not_track = val).ok();
    });

    dnt_row.append(&dnt_info);
    dnt_row.append(&dnt_switch);
    card3.append(&dnt_row);

    // ── DNS Seguro (DoH) ──
    let dns_row_box = GtkBox::new(Orientation::Vertical, 0);
    dns_row_box.add_css_class("settings-row");
    dns_row_box.set_hexpand(true);

    dns_row_box.append(&make_row_label(&t!("settings.dns_secure")));
    dns_row_box.append(&make_row_desc(&t!("settings.dns_secure_desc")));

    let dns_btn_box = GtkBox::new(Orientation::Horizontal, 8);
    dns_btn_box.set_margin_top(10);

    let sys_dns_btn = Button::builder()
        .label(&t!("settings.dns_system"))
        .css_classes(["dns-pill"])
        .build();
    let cf_btn = Button::builder()
        .label("Cloudflare")
        .css_classes(["dns-pill"])
        .build();
    let quad9_btn = Button::builder()
        .label("Quad9")
        .css_classes(["dns-pill"])
        .build();
    let google_btn = Button::builder()
        .label("Google")
        .css_classes(["dns-pill"])
        .build();

    // Marcar DNS ativo
    let current_dns = settings_mgr.get().secure_dns_mode;
    match current_dns {
        brunya_settings::SecureDnsMode::System => sys_dns_btn.add_css_class("active"),
        brunya_settings::SecureDnsMode::Cloudflare => cf_btn.add_css_class("active"),
        brunya_settings::SecureDnsMode::Quad9 => quad9_btn.add_css_class("active"),
        brunya_settings::SecureDnsMode::Google => google_btn.add_css_class("active"),
        brunya_settings::SecureDnsMode::Custom => {}
    }

    // Helper para atualizar botões DNS
    let all_dns_btns: Vec<Button> = vec![
        sys_dns_btn.clone(),
        cf_btn.clone(),
        quad9_btn.clone(),
        google_btn.clone(),
    ];

    // Sistema
    let all_c = all_dns_btns.clone();
    let s_mgr_sys = settings_mgr.clone();
    let sys_btn = sys_dns_btn.clone();
    sys_dns_btn.connect_clicked(move |_| {
        s_mgr_sys
            .update(|s| s.secure_dns_mode = brunya_settings::SecureDnsMode::System)
            .ok();
        for b in &all_c {
            b.remove_css_class("active");
        }
        sys_btn.add_css_class("active");
    });

    // Cloudflare
    let all_c = all_dns_btns.clone();
    let s_mgr_cf = settings_mgr.clone();
    let cf_btn_c = cf_btn.clone();
    cf_btn.connect_clicked(move |_| {
        s_mgr_cf
            .update(|s| s.secure_dns_mode = brunya_settings::SecureDnsMode::Cloudflare)
            .ok();
        for b in &all_c {
            b.remove_css_class("active");
        }
        cf_btn_c.add_css_class("active");
    });

    // Quad9
    let all_c = all_dns_btns.clone();
    let s_mgr_q9 = settings_mgr.clone();
    let quad9_btn_c = quad9_btn.clone();
    quad9_btn.connect_clicked(move |_| {
        s_mgr_q9
            .update(|s| s.secure_dns_mode = brunya_settings::SecureDnsMode::Quad9)
            .ok();
        for b in &all_c {
            b.remove_css_class("active");
        }
        quad9_btn_c.add_css_class("active");
    });

    // Google
    let all_c = all_dns_btns.clone();
    let s_mgr_gg = settings_mgr.clone();
    let google_btn_c = google_btn.clone();
    google_btn.connect_clicked(move |_| {
        s_mgr_gg
            .update(|s| s.secure_dns_mode = brunya_settings::SecureDnsMode::Google)
            .ok();
        for b in &all_c {
            b.remove_css_class("active");
        }
        google_btn_c.add_css_class("active");
    });

    dns_btn_box.append(&sys_dns_btn);
    dns_btn_box.append(&cf_btn);
    dns_btn_box.append(&quad9_btn);
    dns_btn_box.append(&google_btn);
    dns_row_box.append(&dns_btn_box);

    // ── DNS Personalizado ──
    let custom_box = GtkBox::new(Orientation::Horizontal, 8);
    custom_box.set_margin_top(12);

    let custom_entry = Entry::builder()
        .placeholder_text(&t!("settings.dns_custom_placeholder"))
        .hexpand(true)
        .css_classes(["settings-custom-entry"])
        .text(&settings_mgr.get().secure_dns_custom_url)
        .build();

    let apply_custom_btn = Button::builder()
        .label(&t!("settings.dns_apply"))
        .css_classes(["settings-pill"])
        .build();

    let dns_custom_active = Label::builder()
        .label(&t!("settings.dns_custom_badge"))
        .css_classes(["dns-pill", "active"])
        .build();

    // Mostrar o badge "Personalizado" se estiver nesse modo
    if current_dns == brunya_settings::SecureDnsMode::Custom {
        dns_custom_active.set_visible(true);
    } else {
        dns_custom_active.set_visible(false);
    }

    let sm_custom = settings_mgr.clone();
    let dns_custom_active_c = dns_custom_active.clone();
    let all_c2 = all_dns_btns.clone();
    let custom_entry_c = custom_entry.clone();
    apply_custom_btn.connect_clicked(move |_| {
        let url = custom_entry_c.text().trim().to_string();
        sm_custom
            .update(|s| {
                s.secure_dns_mode = brunya_settings::SecureDnsMode::Custom;
                s.secure_dns_custom_url = url;
            })
            .ok();
        for b in &all_c2 {
            b.remove_css_class("active");
        }
        dns_custom_active_c.set_visible(true);
    });

    // Conectar Enter no entry também
    let dns_custom_active_c2 = dns_custom_active.clone();
    let all_c3 = all_dns_btns.clone();
    let sm_custom2 = settings_mgr.clone();
    custom_entry.connect_activate(move |entry| {
        let url = entry.text().trim().to_string();
        sm_custom2
            .update(|s| {
                s.secure_dns_mode = brunya_settings::SecureDnsMode::Custom;
                s.secure_dns_custom_url = url;
            })
            .ok();
        for b in &all_c3 {
            b.remove_css_class("active");
        }
        dns_custom_active_c2.set_visible(true);
    });

    custom_box.append(&custom_entry);
    custom_box.append(&apply_custom_btn);
    custom_box.append(&dns_custom_active);
    dns_row_box.append(&custom_box);

    card3.append(&dns_row_box);

    outer_box.append(&card3);

    // ── Scroll wrapper ──
    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .child(&outer_box)
        .css_classes(["settings-page"])
        .build();

    scroll.upcast()
}

/// Constrói a página interna "brunya://history".
pub fn build_history_view<F: Fn(&str) + 'static>(
    storage_mgr: Arc<StorageManager>,
    on_navigate: F,
) -> gtk4::Widget {
    let container = GtkBox::new(Orientation::Vertical, 16);
    container.set_vexpand(true);
    container.set_hexpand(true);
    container.set_margin_top(24);
    container.set_margin_bottom(24);
    container.set_margin_start(32);
    container.set_margin_end(32);

    let on_nav_rc = Rc::new(on_navigate);

    let header_box = GtkBox::new(Orientation::Horizontal, 16);
    let title = Label::builder()
        .label(&t!("common.history"))
        .css_classes(["title-1", "heading"])
        .hexpand(true)
        .halign(Align::Start)
        .build();

    let search_entry = Entry::builder()
        .placeholder_text(&t!("common.history_search_placeholder"))
        .width_request(300)
        .build();

    let clear_btn = Button::builder()
        .label(&t!("privacy.clear_browsing_data"))
        .css_classes(["destructive-action", "pill"])
        .build();

    header_box.append(&title);
    header_box.append(&search_entry);
    header_box.append(&clear_btn);
    container.append(&header_box);

    let list_box = ListBox::builder()
        .selection_mode(SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    let history_items = storage_mgr.get_recent_history(200).unwrap_or_default();
    let row_widgets = Rc::new(RefCell::new(Vec::<(ActionRow, String, String)>::new()));

    if history_items.is_empty() {
        let empty_label = Label::builder()
            .label(&t!("common.history_empty"))
            .css_classes(["dim-label"])
            .margin_top(32)
            .build();
        container.append(&empty_label);
    } else {
        for item in history_items {
            let row = ActionRow::builder()
                .title(&item.title)
                .subtitle(&format!(
                    "{} • {} visitas • {}",
                    item.url,
                    item.visit_count,
                    item.last_visit.format("%d/%m/%Y %H:%M")
                ))
                .activatable(true)
                .build();

            let target_url = item.url.clone();
            let nav_cb = on_nav_rc.clone();
            row.connect_activated(move |_| {
                nav_cb(&target_url);
            });

            row_widgets.borrow_mut().push((
                row.clone(),
                item.title.to_lowercase(),
                item.url.to_lowercase(),
            ));
            list_box.append(&row);
        }
        container.append(&list_box);
    }

    // Filtragem em tempo real ao digitar na busca
    let rows_filter = row_widgets;
    search_entry.connect_changed(move |entry| {
        let query = entry.text().to_lowercase();
        for (row, title, url) in rows_filter.borrow().iter() {
            let matches = query.is_empty() || title.contains(&query) || url.contains(&query);
            row.set_visible(matches);
        }
    });

    let st_clear = storage_mgr.clone();
    let list_box_clear = list_box.clone();
    clear_btn.connect_clicked(move |_| {
        st_clear.clear_history().ok();
        while let Some(child) = list_box_clear.first_child() {
            list_box_clear.remove(&child);
        }
    });

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .child(&container)
        .build();

    scroll.upcast()
}

/// Constrói a página interna "brunya://downloads".
pub fn build_downloads_view(storage_mgr: Arc<StorageManager>) -> gtk4::Widget {
    let container = GtkBox::new(Orientation::Vertical, 16);
    container.set_vexpand(true);
    container.set_hexpand(true);
    container.set_margin_top(24);
    container.set_margin_bottom(24);
    container.set_margin_start(32);
    container.set_margin_end(32);

    let header_box = GtkBox::new(Orientation::Horizontal, 16);
    let title = Label::builder()
        .label(&t!("downloads.title"))
        .css_classes(["title-1", "heading"])
        .hexpand(true)
        .halign(Align::Start)
        .build();

    let clear_btn = Button::builder()
        .label(&t!("downloads.clear_all"))
        .css_classes(["flat", "pill"])
        .build();

    header_box.append(&title);
    header_box.append(&clear_btn);
    container.append(&header_box);

    let list_box = ListBox::builder()
        .selection_mode(SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    let downloads = storage_mgr.list_downloads(50).unwrap_or_default();

    if downloads.is_empty() {
        let empty_label = Label::builder()
            .label(&t!("downloads.empty"))
            .css_classes(["dim-label"])
            .margin_top(32)
            .build();
        container.append(&empty_label);
    } else {
        for dl in downloads {
            let row = ActionRow::builder()
                .title(&dl.filename)
                .subtitle(&format!(
                    "{} • {} ({:.0}%)",
                    dl.url,
                    brunya_storage::DownloadItem::format_size(dl.downloaded_bytes),
                    dl.progress_percent()
                ))
                .activatable(false)
                .build();

            let actions_box = GtkBox::new(Orientation::Horizontal, 8);
            let path_clone = dl.target_path.clone();

            if dl.state == brunya_storage::DownloadState::Completed {
                let open_btn = Button::builder()
                    .label(&t!("downloads.open_file"))
                    .css_classes(["flat"])
                    .build();

                let p_file = path_clone.clone();
                open_btn.connect_clicked(move |_| {
                    let uri = format!("file://{}", p_file);
                    gio::AppInfo::launch_default_for_uri(&uri, None::<&gio::AppLaunchContext>).ok();
                });
                actions_box.append(&open_btn);
            }

            let folder_btn = Button::builder()
                .icon_name("folder-symbolic")
                .tooltip_text(&t!("downloads.open_folder"))
                .css_classes(["flat"])
                .build();

            let p_folder = path_clone;
            folder_btn.connect_clicked(move |_| {
                if let Some(parent) = std::path::Path::new(&p_folder).parent() {
                    let uri = format!("file://{}", parent.display());
                    gio::AppInfo::launch_default_for_uri(&uri, None::<&gio::AppLaunchContext>).ok();
                }
            });
            actions_box.append(&folder_btn);

            row.add_suffix(&actions_box);
            list_box.append(&row);
        }
        container.append(&list_box);
    }

    let st_clear = storage_mgr.clone();
    clear_btn.connect_clicked(move |_| {
        st_clear.clear_completed_downloads().ok();
    });

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .child(&container)
        .build();

    scroll.upcast()
}

/// Constrói a página interna "brunya://bookmarks".
pub fn build_bookmarks_view<F: Fn(&str) + 'static>(
    storage_mgr: Arc<StorageManager>,
    on_navigate: F,
) -> gtk4::Widget {
    let container = GtkBox::new(Orientation::Vertical, 16);
    container.set_vexpand(true);
    container.set_hexpand(true);
    container.set_margin_top(24);
    container.set_margin_bottom(24);
    container.set_margin_start(32);
    container.set_margin_end(32);

    let on_nav_rc = Rc::new(on_navigate);

    let header_box = GtkBox::new(Orientation::Horizontal, 16);
    let title = Label::builder()
        .label(&t!("common.bookmarks"))
        .css_classes(["title-1", "heading"])
        .hexpand(true)
        .halign(Align::Start)
        .build();

    header_box.append(&title);
    container.append(&header_box);

    let list_box = ListBox::builder()
        .selection_mode(SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    let bookmarks = storage_mgr.list_bookmarks().unwrap_or_default();

    if bookmarks.is_empty() {
        let empty_label = Label::builder()
            .label(&t!("common.bookmarks_empty"))
            .css_classes(["dim-label"])
            .margin_top(32)
            .build();
        container.append(&empty_label);
    } else {
        for bm in bookmarks {
            let row = ActionRow::builder()
                .title(&bm.title)
                .subtitle(&bm.url)
                .activatable(true)
                .build();

            let target_url = bm.url.clone();
            let nav_cb = on_nav_rc.clone();
            row.connect_activated(move |_| {
                nav_cb(&target_url);
            });

            let del_btn = Button::builder()
                .icon_name("user-trash-symbolic")
                .tooltip_text(&t!("common.bookmark_delete"))
                .css_classes(["flat", "circular"])
                .build();

            let st_del = storage_mgr.clone();
            let bmid = bm.id;
            let row_clone = row.clone();
            let list_box_clone = list_box.clone();
            del_btn.connect_clicked(move |_| {
                st_del.delete_bookmark(bmid).ok();
                list_box_clone.remove(&row_clone);
            });

            row.add_suffix(&del_btn);
            list_box.append(&row);
        }
        container.append(&list_box);
    }

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .child(&container)
        .build();

    scroll.upcast()
}

/// Constrói a página interna "brunya://about".
pub fn build_about_view() -> gtk4::Widget {
    let container = GtkBox::new(Orientation::Vertical, 16);
    container.set_valign(Align::Center);
    container.set_halign(Align::Center);
    container.set_vexpand(true);
    container.set_hexpand(true);
    container.set_margin_top(48);
    container.set_margin_bottom(48);

    let logo = Label::builder()
        .label("🌐")
        .css_classes(["display-1"])
        .build();

    let title = Label::builder()
        .label("Brunya Browser")
        .css_classes(["title-1", "heading"])
        .build();

    let version = Label::builder()
        .label(&t!("settings.version", version => "0.3.0-alpha"))
        .css_classes(["title-4", "dim-label"])
        .build();

    let engine_info = Label::builder()
        .label("Motor: Chromium / CEF 156.0.15 (x86_64)")
        .css_classes(["dim-label"])
        .build();

    let license = Label::builder()
        .label(&t!("settings.license"))
        .css_classes(["caption", "dim-label"])
        .build();

    container.append(&logo);
    container.append(&title);
    container.append(&version);
    container.append(&engine_info);
    container.append(&license);

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .child(&container)
        .build();

    scroll.upcast()
}

/// Constrói a página interna "brunya://extensions" — Gerenciador de Extensões.
pub fn build_extensions_view<F, C>(
    ext_system: Arc<ExtensionSystem>,
    _storage_mgr: Arc<StorageManager>,
    on_navigate: F,
    on_changed: C,
) -> gtk4::Widget
where
    F: Fn(&str) + 'static + Clone,
    C: Fn(&str) + 'static + Clone,
{
    let container = GtkBox::new(Orientation::Vertical, 0);
    container.set_vexpand(true);
    container.set_hexpand(true);
    container.add_css_class("settings-page");

    // ── HEADER ──
    let header_box = GtkBox::new(Orientation::Vertical, 2);
    header_box.set_halign(Align::Start);
    header_box.set_margin_start(40);
    header_box.set_margin_end(40);
    header_box.set_margin_top(32);
    header_box.set_margin_bottom(8);
    header_box.add_css_class("settings-page-header");

    let title_box = GtkBox::new(Orientation::Horizontal, 8);
    let title_icon =
        if let Some(texture) = toolbar_icon(libadwaita::StyleManager::default().is_dark()) {
            let image = Image::from_paintable(Some(&texture));
            image.set_pixel_size(26);
            image
        } else {
            let image = Image::from_icon_name("application-x-addon");
            image.set_pixel_size(26);
            image
        };
    let title_lbl = Label::builder()
        .label(&t!("extensions.extensions_title"))
        .css_classes(["settings-title"])
        .build();
    title_box.append(&title_icon);
    title_box.append(&title_lbl);
    let subtitle_lbl = Label::builder()
        .label(&t!("extensions.extensions_subtitle"))
        .css_classes(["settings-subtitle"])
        .halign(Align::Start)
        .build();

    header_box.append(&title_box);
    header_box.append(&subtitle_lbl);
    container.append(&header_box);

    let store_row = GtkBox::new(Orientation::Horizontal, 8);
    store_row.set_margin_start(40);
    store_row.set_margin_end(40);
    store_row.set_margin_top(16);
    let store_entry = Entry::builder()
        .placeholder_text(&t!("extensions.store_placeholder"))
        .hexpand(true)
        .build();
    let store_btn = Button::builder()
        .label(&t!("extensions.store_install"))
        .css_classes(["settings-pill", "suggested-action"])
        .build();
    store_row.append(&store_entry);
    store_row.append(&store_btn);
    container.append(&store_row);

    let status_lbl = Label::builder()
        .label(&t!("extensions.store_hint"))
        .css_classes(["caption", "dim-label"])
        .halign(Align::Start)
        .wrap(true)
        .margin_start(40)
        .margin_end(40)
        .margin_top(6)
        .build();
    container.append(&status_lbl);

    // ── Ações ──
    let actions_box = GtkBox::new(Orientation::Horizontal, 8);
    actions_box.set_margin_start(40);
    actions_box.set_margin_end(40);
    actions_box.set_margin_top(12);
    actions_box.set_margin_bottom(8);

    let open_store_btn = Button::builder()
        .label(&t!("extensions.open_store"))
        .css_classes(["settings-pill", "suggested-action"])
        .build();
    let load_unpacked_btn = Button::builder()
        .label(&t!("extensions.load_unpacked"))
        .css_classes(["settings-pill"])
        .build();
    let check_updates_btn = Button::builder()
        .label(&t!("extensions.check_updates"))
        .css_classes(["settings-pill"])
        .build();
    let file_btn = Button::builder()
        .label(&t!("extensions.install_crx"))
        .css_classes(["settings-pill"])
        .build();

    let on_open_store = on_navigate.clone();
    open_store_btn.connect_clicked(move |_| {
        on_open_store("https://chromewebstore.google.com/category/extensions");
    });

    actions_box.append(&open_store_btn);
    actions_box.append(&load_unpacked_btn);
    actions_box.append(&file_btn);
    actions_box.append(&check_updates_btn);
    container.append(&actions_box);

    // ── Lista de extensões ──
    let ext_list_box = GtkBox::new(Orientation::Vertical, 6);
    ext_list_box.set_margin_start(40);
    ext_list_box.set_margin_end(40);
    ext_list_box.set_margin_top(8);

    let rebuild_list = Rc::new(RefCell::new(Option::<Box<dyn Fn()>>::None));

    // Clone rebuild_list before the move closure captures it
    let rebuild_list_fn = rebuild_list.clone();
    let rebuild_fn = {
        let ext_system_c = ext_system.clone();
        let list_box_c = ext_list_box.clone();
        let on_nav = on_navigate.clone();
        let on_changed_list = on_changed.clone();
        move || {
            while let Some(child) = list_box_c.first_child() {
                list_box_c.remove(&child);
            }
            let extensions = ext_system_c.manager.list_extensions();
            if extensions.is_empty() {
                let empty_box = GtkBox::new(Orientation::Vertical, 8);
                empty_box.set_margin_top(24);
                empty_box.set_halign(Align::Center);
                let empty_icon = if let Some(texture) =
                    toolbar_icon(libadwaita::StyleManager::default().is_dark())
                {
                    Image::from_paintable(Some(&texture))
                } else {
                    Image::from_icon_name("application-x-addon")
                };
                empty_icon.set_pixel_size(48);
                empty_icon.set_opacity(0.3);
                let empty_label = Label::builder()
                    .label(t!("extensions.no_extensions"))
                    .css_classes(["dim-label"])
                    .wrap(true)
                    .build();
                empty_box.append(&empty_icon);
                empty_box.append(&empty_label);
                list_box_c.append(&empty_box);
                return;
            }
            for ext in extensions {
                let ext_card = GtkBox::new(Orientation::Vertical, 0);
                ext_card.add_css_class("settings-card");
                ext_card.set_margin_bottom(6);
                let header_row = GtkBox::new(Orientation::Horizontal, 12);
                header_row.set_margin_top(12);
                header_row.set_margin_bottom(8);
                header_row.set_margin_start(16);
                header_row.set_margin_end(16);
                let icon = extension_image(&ext, 32);
                let info_col = GtkBox::new(Orientation::Vertical, 2);
                info_col.set_hexpand(true);
                let name_lbl = Label::builder()
                    .label(&ext.name)
                    .css_classes(["settings-row-label"])
                    .halign(Align::Start)
                    .build();
                let version_str = format!("v{}", ext.version);
                let version_lbl = Label::builder()
                    .label(&version_str)
                    .css_classes(["caption", "dim-label"])
                    .halign(Align::Start)
                    .build();
                info_col.append(&name_lbl);
                info_col.append(&version_lbl);
                let state_label = match ext.state {
                    ExtensionState::Active => t!("extensions.state_active"),
                    ExtensionState::Disabled => t!("extensions.state_disabled"),
                    ExtensionState::DeveloperMode => t!("extensions.state_developer"),
                    ExtensionState::Error => t!("extensions.state_error"),
                    ExtensionState::Incompatible => t!("extensions.state_incompatible"),
                    _ => "—".to_string(),
                };
                let state_badge = Label::builder()
                    .label(&state_label)
                    .css_classes(match ext.state {
                        ExtensionState::Active => vec!["badge", "success"],
                        ExtensionState::Disabled => vec!["badge"],
                        ExtensionState::DeveloperMode => vec!["badge", "accent"],
                        ExtensionState::Error => vec!["badge", "warning"],
                        ExtensionState::Incompatible => vec!["badge", "error"],
                        _ => vec!["badge"],
                    })
                    .build();
                header_row.append(&icon);
                header_row.append(&info_col);
                header_row.append(&state_badge);
                let actions_bar = GtkBox::new(Orientation::Horizontal, 6);
                actions_bar.set_margin_start(16);
                actions_bar.set_margin_end(16);
                actions_bar.set_margin_bottom(12);
                let enable_label = if ext.state == ExtensionState::Disabled {
                    t!("extensions.enable")
                } else {
                    t!("extensions.disable")
                };
                let enable_btn = Button::builder()
                    .label(&enable_label)
                    .css_classes(["settings-pill"])
                    .build();
                let ext_id_clone = ext.id.clone();
                let ext_sys_e = ext_system_c.clone();
                let rebuild_e = rebuild_list_fn.clone();
                let on_changed_e = on_changed_list.clone();
                enable_btn.connect_clicked(move |_| {
                    if ext_sys_e
                        .registry
                        .get(&ext_id_clone)
                        .map(|e| e.state == ExtensionState::Disabled)
                        .unwrap_or(false)
                    {
                        ext_sys_e.manager.enable_extension(&ext_id_clone).ok();
                    } else {
                        ext_sys_e.manager.disable_extension(&ext_id_clone).ok();
                    }
                    on_changed_e("refresh");
                    if let Some(ref f) = *rebuild_e.borrow() {
                        f();
                    }
                });
                let details_label = t!("extensions.details");
                let details_btn = Button::builder()
                    .label(&details_label)
                    .css_classes(["settings-pill"])
                    .build();
                let ext_id_d = ext.id.clone();
                let on_nav_d = on_nav.clone();
                details_btn.connect_clicked(move |_| {
                    on_nav_d(&format!("brunya://extensions?id={}", ext_id_d.0));
                });
                let remove_label = t!("extensions.remove");
                let remove_btn = Button::builder()
                    .label(&remove_label)
                    .css_classes(["settings-pill", "destructive-action"])
                    .build();
                let ext_id_r = ext.id.clone();
                let ext_sys_rm = ext_system_c.clone();
                let rebuild_rm = rebuild_list_fn.clone();
                let on_changed_rm = on_changed_list.clone();
                remove_btn.connect_clicked(move |_| {
                    ext_sys_rm.manager.remove_extension(&ext_id_r).ok();
                    on_changed_rm("refresh");
                    if let Some(ref f) = *rebuild_rm.borrow() {
                        f();
                    }
                });
                if !ext.permissions.is_empty() {
                    let perms_lbl = Label::builder()
                        .label(format!("🔑 {}", ext.permissions.join(", ")))
                        .css_classes(["caption", "dim-label"])
                        .halign(Align::Start)
                        .hexpand(true)
                        .ellipsize(gtk4::pango::EllipsizeMode::End)
                        .build();
                    let detail_row = GtkBox::new(Orientation::Horizontal, 12);
                    detail_row.set_margin_start(16);
                    detail_row.set_margin_end(16);
                    detail_row.set_margin_bottom(8);
                    detail_row.append(&perms_lbl);
                    ext_card.append(&detail_row);
                }
                actions_bar.append(&enable_btn);
                actions_bar.append(&details_btn);
                actions_bar.append(&remove_btn);
                ext_card.append(&header_row);
                ext_card.append(&actions_bar);
                list_box_c.append(&ext_card);
            }
        }
    };

    *rebuild_list.borrow_mut() = Some(Box::new(rebuild_fn));
    if let Some(ref f) = *rebuild_list.borrow() {
        f();
    }

    let ext_sys_load = ext_system.clone();
    let rebuild_load = rebuild_list.clone();
    let on_changed_load = on_changed.clone();
    let status_load = status_lbl.clone();
    load_unpacked_btn.connect_clicked(move |btn| {
        let dialog = gtk4::FileDialog::builder()
            .title(&t!("extensions.select_directory"))
            .modal(true)
            .build();

        let ext_sys = ext_sys_load.clone();
        let rebuild = rebuild_load.clone();
        let status_load = status_load.clone();
        let on_changed_load = on_changed_load.clone();

        if let Some(root) = btn.root() {
            if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                dialog.select_folder(Some(window), None::<&gio::Cancellable>, move |result| {
                    match result {
                        Ok(file) => {
                            let path = file.path();
                            if let Some(dir) = path {
                                if dir.join("manifest.json").exists() {
                                    match ext_sys.manager.install_from_directory(&dir) {
                                        Ok(info) => {
                                            tracing::info!(
                                                "Extensão carregada: {} v{}",
                                                info.name,
                                                info.version
                                            );
                                            status_load
                                                .set_label(&t!("extensions.extension_installed"));
                                            on_changed_load(&format!("installed:{}", info.id.0));
                                            if let Some(ref f) = *rebuild.borrow() {
                                                f();
                                            }
                                        }
                                        Err(e) => {
                                            status_load.set_label(&e.to_string());
                                            tracing::warn!("Falha ao carregar extensão: {}", e);
                                        }
                                    }
                                } else {
                                    tracing::warn!("Nenhum manifest.json encontrado em {:?}", dir);
                                }
                            }
                        }
                        Err(e) => {
                            if !e
                                .kind::<gio::IOErrorEnum>()
                                .is_some_and(|k| k == gio::IOErrorEnum::Cancelled)
                            {
                                tracing::warn!("Erro no seletor de pasta: {}", e);
                            }
                        }
                    }
                });
            }
        }
    });

    let ext_sys_up = ext_system.clone();
    let status_up = status_lbl.clone();
    check_updates_btn.connect_clicked(move |_| {
        ext_sys_up.update_mgr.check_for_updates();
        status_up.set_label(&t!("extensions.up_to_date"));
    });

    let ext_sys_store = ext_system.clone();
    let rebuild_store = rebuild_list.clone();
    let on_changed_store = on_changed.clone();
    let status_store = status_lbl.clone();
    let entry_store = store_entry.clone();
    store_btn.connect_clicked(move |btn| {
        let query = entry_store.text().to_string();
        btn.set_sensitive(false);
        let button = btn.clone();
        let status = status_store.clone();
        let status_done = status.clone();
        let system = ext_sys_store.clone();
        let rebuild = rebuild_store.clone();
        let on_changed = on_changed_store.clone();
        begin_store_install(
            query,
            btn,
            system,
            move |text| status.set_label(&text),
            move |end| {
                button.set_sensitive(true);
                match end {
                    StoreInstallEnd::Installed(id) => {
                        on_changed(&format!("installed:{id}"));
                        if let Some(ref refresh) = *rebuild.borrow() {
                            refresh();
                        }
                    }
                    StoreInstallEnd::Cancelled => {
                        status_done.set_label(&t!("extensions.store_hint"))
                    }
                    StoreInstallEnd::Failed => {}
                }
            },
        );
    });

    let ext_sys_file = ext_system.clone();
    let rebuild_file = rebuild_list.clone();
    let on_changed_file = on_changed.clone();
    let status_file = status_lbl.clone();
    file_btn.connect_clicked(move |btn| {
        let dialog = gtk4::FileDialog::builder()
            .title(&t!("extensions.select_crx"))
            .modal(true)
            .build();
        let filter = gtk4::FileFilter::new();
        filter.add_pattern("*.crx");
        filter.add_pattern("*.zip");
        filter.add_pattern("*.xpi");
        filter.set_name(Some("CRX, ZIP, XPI"));
        dialog.set_default_filter(Some(&filter));
        let system = ext_sys_file.clone();
        let rebuild = rebuild_file.clone();
        let on_changed = on_changed_file.clone();
        let status = status_file.clone();
        if let Some(root) = btn.root() {
            if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                let parent = window.clone();
                dialog.open(Some(window), None::<&gio::Cancellable>, move |result| {
                    let Ok(file) = result else { return };
                    let Some(path) = file.path() else { return };
                    let bytes = match std::fs::read(&path) {
                        Ok(bytes) => bytes,
                        Err(err) => {
                            status.set_label(&err.to_string());
                            return;
                        }
                    };
                    let preview = match inspect_package(&bytes, None) {
                        Ok(preview) => preview,
                        Err(err) => {
                            status.set_label(&err.to_string());
                            return;
                        }
                    };
                    let detail = permission_text(&preview, "arquivo local");
                    let title = t!("extensions.install_confirm_title");
                    confirm_install(
                        Some(parent),
                        title,
                        detail,
                        move || match system.manager.install_from_bytes(
                            &bytes,
                            Some(&preview.id),
                            InstallType::Normal,
                        ) {
                            Ok(info) => {
                                status.set_label(&t!("extensions.extension_installed"));
                                on_changed(&format!("installed:{}", info.id.0));
                                if let Some(ref refresh) = *rebuild.borrow() {
                                    refresh();
                                }
                            }
                            Err(err) => status.set_label(&err.to_string()),
                        },
                        || {},
                    );
                });
            }
        }
    });

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .child(&container)
        .css_classes(["settings-page"])
        .build();
    scroll.upcast()
}

/// Resultado do download de uma extensão a partir de uma loja.
pub enum StoreInstallEnd {
    Installed(String),
    Cancelled,
    Failed,
}

/// Baixa o pacote da loja, pede confirmação das permissões e instala.
pub fn begin_store_install<S, F, W>(
    query: String,
    anchor: &W,
    system: Arc<ExtensionSystem>,
    on_status: S,
    on_finished: F,
) where
    S: Fn(String) + 'static,
    F: Fn(StoreInstallEnd) + 'static,
    W: IsA<gtk4::Widget> + Clone,
{
    let request = match parse_store_input(&query) {
        Ok(request) => request,
        Err(err) => {
            on_status(err.to_string());
            on_finished(StoreInstallEnd::Failed);
            return;
        }
    };
    on_status(t!("extensions.store_installing"));
    let on_status = Rc::new(on_status);
    let on_finished = Rc::new(on_finished);
    let anchor = W::clone(anchor).upcast::<gtk4::Widget>();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(download_store_package(&request));
    });
    glib::timeout_add_local(
        std::time::Duration::from_millis(150),
        move || match receiver.try_recv() {
            Ok(Ok(package)) => {
                let detail = permission_text(&package.preview, &package.store_name);
                let title = t!("extensions.install_confirm_title");
                let parent = anchor
                    .root()
                    .and_then(|root| root.downcast::<gtk4::Window>().ok());
                let bytes = package.bytes;
                let extension_id = package.preview.id;
                let system = system.clone();
                let on_status_ok = Rc::clone(&on_status);
                let on_finished_ok = Rc::clone(&on_finished);
                let on_finished_cancel = Rc::clone(&on_finished);
                confirm_install(
                    parent,
                    title,
                    detail,
                    move || match system.manager.install_from_bytes(
                        &bytes,
                        Some(&extension_id),
                        InstallType::FromStore,
                    ) {
                        Ok(info) => {
                            (&*on_status_ok)(t!("extensions.extension_installed"));
                            (&*on_finished_ok)(StoreInstallEnd::Installed(info.id.0));
                        }
                        Err(err) => {
                            (&*on_status_ok)(err.to_string());
                            (&*on_finished_ok)(StoreInstallEnd::Failed);
                        }
                    },
                    move || (&*on_finished_cancel)(StoreInstallEnd::Cancelled),
                );
                glib::ControlFlow::Break
            }
            Ok(Err(err)) => {
                (&*on_status)(err.to_string());
                (&*on_finished)(StoreInstallEnd::Failed);
                glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                (&*on_status)(t!("extensions.extension_error"));
                (&*on_finished)(StoreInstallEnd::Failed);
                glib::ControlFlow::Break
            }
        },
    );
}

fn permission_text(preview: &brunya_extensions::loader::PackagePreview, source: &str) -> String {
    let mut lines = vec![
        format!("{} v{}", preview.name, preview.version),
        source.to_string(),
    ];
    if let Some(description) = &preview.description {
        if !description.is_empty() {
            lines.push(description.clone());
        }
    }
    let mut permissions = preview.permissions.clone();
    permissions.extend(preview.host_permissions.iter().cloned());
    if permissions.is_empty() {
        lines.push(t!("extensions.no_permissions"));
    } else {
        lines.push(t!("extensions.permissions_title"));
        for permission in permissions {
            lines.push(format!("• {permission}"));
        }
    }
    lines.join("\n")
}

fn confirm_install(
    parent: Option<gtk4::Window>,
    title: String,
    detail: String,
    on_accept: impl FnOnce() + 'static,
    on_cancel: impl FnOnce() + 'static,
) {
    let cancel = t!("extensions.install_cancel");
    let accept = t!("extensions.install_accept");
    let dialog = gtk4::AlertDialog::builder()
        .modal(true)
        .message(&title)
        .detail(&detail)
        .buttons([cancel.as_str(), accept.as_str()])
        .cancel_button(0)
        .default_button(1)
        .build();
    dialog.choose(
        parent.as_ref(),
        None::<&gio::Cancellable>,
        move |response| {
            if response == Ok(1) {
                on_accept();
            } else {
                on_cancel();
            }
        },
    );
}
