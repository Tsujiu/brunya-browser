use bear_i18n::{t, I18nManager, Language};
use bear_settings::{SettingsManager, ThemePreference};
use bear_storage::StorageManager;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Entry, Label, ListBox, Orientation,
    ScrolledWindow, SelectionMode,
};
use libadwaita::prelude::*;
use libadwaita::{ActionRow, PreferencesGroup, PreferencesPage, SwitchRow};
use std::rc::Rc;
use std::sync::Arc;

/// Constrói a página interna "bear://newtab" (Nova Aba com atalhos e busca limpa).
pub fn build_newtab_view<F: Fn(&str) + 'static>(on_navigate: F) -> gtk4::Widget {
    let container = GtkBox::new(Orientation::Vertical, 24);
    container.set_valign(Align::Center);
    container.set_halign(Align::Center);
    container.set_margin_top(48);
    container.set_margin_bottom(48);
    container.set_margin_start(24);
    container.set_margin_end(24);

    let title = Label::builder()
        .label("BEAR Browser")
        .css_classes(["title-1", "heading"])
        .build();

    let subtitle = Label::builder()
        .label(&t!("common.app_tagline"))
        .css_classes(["title-4", "dim-label"])
        .build();

    container.append(&title);
    container.append(&subtitle);

    let search_box = GtkBox::new(Orientation::Horizontal, 8);
    search_box.set_halign(Align::Center);

    let search_entry = Entry::builder()
        .placeholder_text(&t!("common.address_placeholder"))
        .width_request(450)
        .build();

    let search_btn = Button::builder()
        .icon_name("system-search-symbolic")
        .css_classes(["suggested-action"])
        .build();

    let on_nav_clone = Rc::new(on_navigate);
    let nav_click = on_nav_clone.clone();
    let entry_clone = search_entry.clone();
    search_btn.connect_clicked(move |_| {
        let text = entry_clone.text();
        if !text.is_empty() {
            nav_click(text.as_str());
        }
    });

    let nav_enter = on_nav_clone.clone();
    search_entry.connect_activate(move |entry| {
        let text = entry.text();
        if !text.is_empty() {
            nav_enter(text.as_str());
        }
    });

    search_box.append(&search_entry);
    search_box.append(&search_btn);
    container.append(&search_box);

    let shortcuts_box = GtkBox::new(Orientation::Horizontal, 16);
    shortcuts_box.set_halign(Align::Center);
    shortcuts_box.set_margin_top(16);

    let shortcuts = [
        ("Linux Mint", "https://linuxmint.com"),
        ("Ubuntu", "https://ubuntu.com"),
        ("Kernel.org", "https://kernel.org"),
        ("DuckDuckGo", "https://duckduckgo.com"),
        ("Wikipedia", "https://pt.wikipedia.org"),
    ];

    for (name, url) in shortcuts {
        let btn = Button::builder()
            .label(name)
            .css_classes(["flat", "pill"])
            .build();
        let target = url.to_string();
        let nav_shortcut = on_nav_clone.clone();
        btn.connect_clicked(move |_| {
            nav_shortcut(&target);
        });
        shortcuts_box.append(&btn);
    }

    container.append(&shortcuts_box);

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .child(&container)
        .build();

    scroll.upcast()
}

/// Constrói a página interna "bear://settings".
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

    priv_group.add(&cookie_row);
    page.add(&priv_group);

    page.upcast()
}

/// Constrói a página interna "bear://history".
pub fn build_history_view<F: Fn(&str) + 'static>(
    storage_mgr: Arc<StorageManager>,
    on_navigate: F,
) -> gtk4::Widget {
    let container = GtkBox::new(Orientation::Vertical, 16);
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

    let clear_btn = Button::builder()
        .label(&t!("privacy.clear_browsing_data"))
        .css_classes(["destructive-action"])
        .build();

    header_box.append(&title);
    header_box.append(&clear_btn);
    container.append(&header_box);

    let list_box = ListBox::builder()
        .selection_mode(SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    let history_items = storage_mgr.get_recent_history(100).unwrap_or_default();

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
                .subtitle(&format!("{} • {} visitas", item.url, item.visit_count))
                .activatable(true)
                .build();

            let target_url = item.url.clone();
            let nav_cb = on_nav_rc.clone();
            row.connect_activated(move |_| {
                nav_cb(&target_url);
            });

            list_box.append(&row);
        }
        container.append(&list_box);
    }

    let st_clear = storage_mgr.clone();
    clear_btn.connect_clicked(move |_| {
        st_clear.clear_history().ok();
    });

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .child(&container)
        .build();

    scroll.upcast()
}

/// Constrói a página interna "bear://about".
pub fn build_about_view() -> gtk4::Widget {
    let container = GtkBox::new(Orientation::Vertical, 16);
    container.set_valign(Align::Center);
    container.set_halign(Align::Center);
    container.set_margin_top(48);
    container.set_margin_bottom(48);

    let logo = Label::builder()
        .label("🐻")
        .css_classes(["display-1"])
        .build();

    let title = Label::builder()
        .label("BEAR Browser")
        .css_classes(["title-1", "heading"])
        .build();

    let version = Label::builder()
        .label(&t!("settings.version", version => "0.1.0-alpha"))
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
        .child(&container)
        .build();

    scroll.upcast()
}
