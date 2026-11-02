use brunya_i18n::{t, I18nManager, Language};
use brunya_settings::{SettingsManager, ThemePreference};
use brunya_storage::StorageManager;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Entry, Image, Label, ListBox, Orientation,
    ScrolledWindow, SelectionMode,
};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, PreferencesGroup, PreferencesPage, SwitchRow};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// Constrói a página interna "brunya://newtab" inspirada no buscador Pesquisa.
pub fn build_newtab_view<F: Fn(&str) + 'static>(
    settings_mgr: Arc<SettingsManager>,
    on_navigate: F,
) -> gtk4::Widget {
    let main_box = GtkBox::new(Orientation::Vertical, 0);
    main_box.set_vexpand(true);
    main_box.set_hexpand(true);
    main_box.add_css_class("pesquisa-page");

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

    // 1. Logo "Pesquisa." com a mesma fonte serifada e ponto estilizado em azul
    let logo = Label::builder()
        .use_markup(true)
        .label("<span font_family=\"Fraunces, Georgia, DejaVu Serif, serif\" size=\"56000\" weight=\"bold\">Pesquisa<span foreground=\"#93c5fd\">.</span></span>")
        .css_classes(["pesquisa-logo"])
        .build();

    // 2. Tagline
    let tagline = Label::builder()
        .label("Pesquise qualquer coisa, em qualquer lugar — rápido, privado e moderno.")
        .css_classes(["pesquisa-tagline"])
        .wrap(true)
        .justify(gtk4::Justification::Center)
        .build();

    hero.append(&logo);
    hero.append(&tagline);

    // 3. Search Box Wrapper estilizado com Seletor Discreto de Motores de Busca
    let search_box = GtkBox::new(Orientation::Horizontal, 8);
    search_box.set_halign(Align::Center);
    search_box.set_width_request(600);
    search_box.add_css_class("pesquisa-search-container");

    let search_icon = Image::from_icon_name("system-search-symbolic");
    search_icon.set_opacity(0.6);

    // Motor de busca ativo
    let current_engine = settings_mgr.get().search_engine;
    let selected_template = Rc::new(RefCell::new(current_engine.query_template));

    let engine_btn = Button::builder()
        .css_classes(["flat", "pill", "engine-picker-btn"])
        .tooltip_text("Selecionar motor de busca")
        .build();

    let engine_content = GtkBox::new(Orientation::Horizontal, 4);
    let engine_glyph = Label::builder()
        .label("🦆")
        .css_classes(["caption", "bold"])
        .build();
    let engine_arrow = Image::from_icon_name("pan-down-symbolic");
    engine_arrow.set_pixel_size(10);

    engine_content.append(&engine_glyph);
    engine_content.append(&engine_arrow);
    engine_btn.set_child(Some(&engine_content));

    let search_entry = Entry::builder()
        .placeholder_text("Pesquisar no Pesquisa...")
        .hexpand(true)
        .css_classes(["pesquisa-entry"])
        .build();

    let engine_glyph_clone = engine_glyph.clone();
    let entry_for_ph = search_entry.clone();
    let template_store = selected_template.clone();
    let sm_clone = settings_mgr.clone();

    engine_btn.connect_clicked(move |btn| {
        let g_clone = engine_glyph_clone.clone();
        let entry_c = entry_for_ph.clone();
        let tmpl_c = template_store.clone();
        let sm_c = sm_clone.clone();

        let popover = crate::ui::engine_popover::SearchEnginePopover::new(move |engine| {
            g_clone.set_label(engine.glyph);
            entry_c.set_placeholder_text(Some(&format!("Pesquisar no {}...", engine.name)));
            *tmpl_c.borrow_mut() = engine.query_template.to_string();
            sm_c.update(|s| {
                s.search_engine = brunya_settings::SearchEngine {
                    name: engine.name.to_string(),
                    query_template: engine.query_template.to_string(),
                };
            }).ok();
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

    // 4. Botão de Pesquisar
    let buttons_box = GtkBox::new(Orientation::Horizontal, 10);
    buttons_box.set_halign(Align::Center);
    buttons_box.set_margin_top(16);

    let search_btn = Button::builder()
        .label("Pesquisar")
        .css_classes(["pesquisa-btn-primary"])
        .build();

    let on_nav_clone = Rc::new(on_navigate);
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

    // 5. Rodapé discreto com dica de teclado
    let footer_box = GtkBox::new(Orientation::Vertical, 4);
    footer_box.set_halign(Align::Center);
    footer_box.set_valign(Align::End);
    footer_box.set_margin_bottom(20);

    let footer_label = Label::builder()
        .label("Pesquisa · Buscador moderno em Português")
        .css_classes(["pesquisa-footer-text"])
        .opacity(0.85)
        .build();

    let hint_label = Label::builder()
        .label("Dica: pressione Enter para pesquisar")
        .css_classes(["pesquisa-footer-text", "caption"])
        .opacity(0.65)
        .build();

    footer_box.append(&footer_label);
    footer_box.append(&hint_label);

    main_box.append(&hero);
    main_box.append(&footer_box);

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

/// Constrói a página interna "brunya://settings".
pub fn build_settings_view<F: Fn(Language) + 'static>(
    settings_mgr: Arc<SettingsManager>,
    on_language_changed: F,
) -> gtk4::Widget {
    let page = PreferencesPage::new();
    let on_lang_rc = Rc::new(on_language_changed);

    let general_group = PreferencesGroup::builder()
        .title(&t!("settings.general"))
        .build();

    let lang_row = ActionRow::builder()
        .title(&t!("settings.language"))
        .build();

    let lang_box = GtkBox::new(Orientation::Horizontal, 8);
    let pt_btn = Button::builder()
        .label("Português (Brasil)")
        .css_classes(["pill"])
        .build();
    let en_btn = Button::builder()
        .label("English")
        .css_classes(["pill"])
        .build();

    let lang_cb_pt = on_lang_rc.clone();
    let s_mgr_pt = settings_mgr.clone();
    pt_btn.connect_clicked(move |_| {
        I18nManager::global().set_language(Language::PtBr);
        s_mgr_pt.update(|s| s.language = "pt-BR".to_string()).ok();
        lang_cb_pt(Language::PtBr);
    });

    let lang_cb_en = on_lang_rc.clone();
    let s_mgr_en = settings_mgr.clone();
    en_btn.connect_clicked(move |_| {
        I18nManager::global().set_language(Language::En);
        s_mgr_en.update(|s| s.language = "en".to_string()).ok();
        lang_cb_en(Language::En);
    });

    lang_box.append(&pt_btn);
    lang_box.append(&en_btn);
    lang_row.add_suffix(&lang_box);
    general_group.add(&lang_row);

    let theme_row = ActionRow::builder()
        .title(&t!("settings.theme"))
        .build();
    let theme_box = GtkBox::new(Orientation::Horizontal, 8);

    let system_theme_btn = Button::builder().label(&t!("settings.theme_system")).build();
    let light_theme_btn = Button::builder().label(&t!("settings.theme_light")).build();
    let dark_theme_btn = Button::builder().label(&t!("settings.theme_dark")).build();

    let s_mgr_t1 = settings_mgr.clone();
    system_theme_btn.connect_clicked(move |_| {
        libadwaita::StyleManager::default().set_color_scheme(libadwaita::ColorScheme::Default);
        s_mgr_t1.update(|s| s.theme = ThemePreference::System).ok();
    });

    let s_mgr_t2 = settings_mgr.clone();
    light_theme_btn.connect_clicked(move |_| {
        libadwaita::StyleManager::default().set_color_scheme(libadwaita::ColorScheme::ForceLight);
        s_mgr_t2.update(|s| s.theme = ThemePreference::Light).ok();
    });

    let s_mgr_t3 = settings_mgr.clone();
    dark_theme_btn.connect_clicked(move |_| {
        libadwaita::StyleManager::default().set_color_scheme(libadwaita::ColorScheme::ForceDark);
        s_mgr_t3.update(|s| s.theme = ThemePreference::Dark).ok();
    });

    theme_box.append(&system_theme_btn);
    theme_box.append(&light_theme_btn);
    theme_box.append(&dark_theme_btn);
    theme_row.add_suffix(&theme_box);
    general_group.add(&theme_row);

    page.add(&general_group);

    let perf_group = PreferencesGroup::builder()
        .title(&t!("settings.tab_suspension"))
        .description(&t!("settings.tab_suspension_desc"))
        .build();

    let susp_switch = SwitchRow::builder()
        .title(&t!("settings.tab_suspension"))
        .active(settings_mgr.get().tab_suspension_enabled)
        .build();

    let s_mgr_susp = settings_mgr.clone();
    susp_switch.connect_active_notify(move |sw| {
        let val = sw.is_active();
        s_mgr_susp.update(|s| s.tab_suspension_enabled = val).ok();
    });

    perf_group.add(&susp_switch);
    page.add(&perf_group);

    let priv_group = PreferencesGroup::builder()
        .title(&t!("settings.privacy_and_security"))
        .build();

    let cookie_row = SwitchRow::builder()
        .title(&t!("privacy.cookies_blocked"))
        .active(settings_mgr.get().block_third_party_cookies)
        .build();

    let s_mgr_ck = settings_mgr.clone();
    cookie_row.connect_active_notify(move |sw| {
        let val = sw.is_active();
        s_mgr_ck.update(|s| s.block_third_party_cookies = val).ok();
    });

    // Do Not Track
    let dnt_row = SwitchRow::builder()
        .title("Não rastrear (Do Not Track)")
        .subtitle("Envia cabeçalho 'DNT: 1' para todos os sites solicitando não rastreamento")
        .active(settings_mgr.get().do_not_track)
        .build();

    let s_mgr_dnt = settings_mgr.clone();
    dnt_row.connect_active_notify(move |sw| {
        let val = sw.is_active();
        s_mgr_dnt.update(|s| s.do_not_track = val).ok();
    });

    // DNS Seguro (DoH)
    let dns_row = ActionRow::builder()
        .title("DNS Seguro (DNS-over-HTTPS)")
        .subtitle("Criptografa consultas de domínio protegendo contra espionagem do provedor local")
        .build();

    let dns_box = GtkBox::new(Orientation::Horizontal, 6);
    let quad9_btn = Button::builder().label("Quad9 (Privacidade)").css_classes(["pill"]).build();
    let cf_btn = Button::builder().label("Cloudflare").css_classes(["pill"]).build();
    let sys_dns_btn = Button::builder().label("Sistema").css_classes(["pill"]).build();

    let s_mgr_q9 = settings_mgr.clone();
    quad9_btn.connect_clicked(move |_| {
        s_mgr_q9.update(|s| s.secure_dns_mode = brunya_settings::SecureDnsMode::Quad9).ok();
    });

    let s_mgr_cf = settings_mgr.clone();
    cf_btn.connect_clicked(move |_| {
        s_mgr_cf.update(|s| s.secure_dns_mode = brunya_settings::SecureDnsMode::Cloudflare).ok();
    });

    let s_mgr_sys = settings_mgr.clone();
    sys_dns_btn.connect_clicked(move |_| {
        s_mgr_sys.update(|s| s.secure_dns_mode = brunya_settings::SecureDnsMode::System).ok();
    });

    dns_box.append(&quad9_btn);
    dns_box.append(&cf_btn);
    dns_box.append(&sys_dns_btn);
    dns_row.add_suffix(&dns_box);

    priv_group.add(&cookie_row);
    priv_group.add(&dnt_row);
    priv_group.add(&dns_row);
    page.add(&priv_group);

    page.set_vexpand(true);
    page.set_hexpand(true);
    page.upcast()
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
        .placeholder_text("Pesquisar no histórico...")
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
            .label("Nenhum histórico recente registrado.")
            .css_classes(["dim-label"])
            .margin_top(32)
            .build();
        container.append(&empty_label);
    } else {
        for item in history_items {
            let row = ActionRow::builder()
                .title(&item.title)
                .subtitle(&format!("{} • {} visitas • {}", item.url, item.visit_count, item.last_visit.format("%d/%m/%Y %H:%M")))
                .activatable(true)
                .build();

            let target_url = item.url.clone();
            let nav_cb = on_nav_rc.clone();
            row.connect_activated(move |_| {
                nav_cb(&target_url);
            });

            row_widgets.borrow_mut().push((row.clone(), item.title.to_lowercase(), item.url.to_lowercase()));
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
            .label("Nenhum favorito salvo ainda. Pressione Ctrl+D para salvar a página atual.")
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
                .tooltip_text("Excluir favorito")
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
