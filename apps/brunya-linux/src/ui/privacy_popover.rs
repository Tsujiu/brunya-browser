use brunya_adblock::{AdblockEngine, ShieldProfile};
use brunya_core::types::TabId;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Image, Label, ListBox, Orientation, Popover,
    ScrolledWindow, SelectionMode, Switch,
};
use libadwaita::prelude::*;
use libadwaita::ActionRow;
use std::sync::Arc;

pub struct PrivacyShieldPopover {
    pub popover: Popover,
}

impl PrivacyShieldPopover {
    pub fn new(adblock_engine: Arc<AdblockEngine>, tab_id: TabId, current_site: String) -> Self {
        let popover = Popover::new();

        let content = GtkBox::new(Orientation::Vertical, 10);
        content.set_margin_top(14);
        content.set_margin_bottom(14);
        content.set_margin_start(16);
        content.set_margin_end(16);
        content.set_width_request(380);

        // 1. Cabeçalho com o Brasão Oficial do BruNya Shield
        let header_box = GtkBox::new(Orientation::Horizontal, 10);
        header_box.set_valign(Align::Center);

        let shield_bytes = glib::Bytes::from_static(include_bytes!("../../../../assets/icons/brunya-shield-white.svg"));
        let shield_tex = gdk4::Texture::from_bytes(&shield_bytes).ok();
        let shield_img = if let Some(ref tex) = shield_tex {
            let img = Image::from_paintable(Some(tex));
            img.set_pixel_size(24);
            img
        } else {
            Image::from_icon_name("security-high-symbolic")
        };

        let title_labels = GtkBox::new(Orientation::Vertical, 2);
        let title = Label::builder()
            .label("BruNya Shield")
            .css_classes(["title-3", "heading", "bold"])
            .halign(Align::Start)
            .build();
        let subtitle = Label::builder()
            .label("Proteção para você. Liberdade para explorar.")
            .css_classes(["caption", "dim-label"])
            .halign(Align::Start)
            .build();

        title_labels.append(&title);
        title_labels.append(&subtitle);
        header_box.append(&shield_img);
        header_box.append(&title_labels);
        content.append(&header_box);

        // 2. Proteção no Site Atual (Toggle por site)
        let site_card = GtkBox::new(Orientation::Horizontal, 10);
        site_card.set_margin_top(4);
        site_card.add_css_class("card");
        site_card.set_margin_bottom(4);

        let site_name = if current_site.is_empty() {
            "Páginas Internas".to_string()
        } else {
            current_site.clone()
        };

        let site_info = GtkBox::new(Orientation::Vertical, 2);
        site_info.set_hexpand(true);
        let site_title = Label::builder()
            .label(&site_name)
            .css_classes(["bold", "caption"])
            .halign(Align::Start)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();
        let site_status_lbl = Label::builder()
            .label("Proteção ativa neste site")
            .css_classes(["caption", "success"])
            .halign(Align::Start)
            .build();
        site_info.append(&site_title);
        site_info.append(&site_status_lbl);

        let is_allowed = adblock_engine.is_site_allowed(&current_site);
        let site_switch = Switch::builder()
            .active(!is_allowed)
            .valign(Align::Center)
            .build();

        let ab_toggle = adblock_engine.clone();
        let site_toggle = current_site.clone();
        let status_lbl_c = site_status_lbl.clone();
        site_switch.connect_active_notify(move |sw| {
            if sw.is_active() {
                ab_toggle.remove_site_exception(&site_toggle);
                status_lbl_c.set_label("Proteção ativa neste site");
                status_lbl_c.remove_css_class("warning");
                status_lbl_c.add_css_class("success");
            } else {
                ab_toggle.allow_site(&site_toggle);
                status_lbl_c.set_label("Proteção desativada neste site");
                status_lbl_c.remove_css_class("success");
                status_lbl_c.add_css_class("warning");
            }
        });

        site_card.append(&site_info);
        site_card.append(&site_switch);
        content.append(&site_card);

        // 3. Perfis de Proteção: Essencial, Equilibrado, Rigoroso, Personalizado
        let profile_lbl = Label::builder()
            .label("PERFIL DE PROTEÇÃO")
            .css_classes(["heading", "caption", "dim-label"])
            .halign(Align::Start)
            .margin_top(4)
            .build();
        content.append(&profile_lbl);

        let profile_box = GtkBox::new(Orientation::Horizontal, 4);
        let current_profile = adblock_engine.profile();

        let p_ess = Button::builder()
            .label("Essencial")
            .css_classes(["pill", if current_profile == ShieldProfile::Essential { "suggested-action" } else { "flat" }])
            .build();
        let p_bal = Button::builder()
            .label("Equilibrado")
            .css_classes(["pill", if current_profile == ShieldProfile::Balanced { "suggested-action" } else { "flat" }])
            .build();
        let p_str = Button::builder()
            .label("Rigoroso")
            .css_classes(["pill", if current_profile == ShieldProfile::Strict { "suggested-action" } else { "flat" }])
            .build();

        let ab_p1 = adblock_engine.clone();
        let b_e = p_ess.clone();
        let b_b = p_bal.clone();
        let b_s = p_str.clone();
        p_ess.connect_clicked(move |_| {
            ab_p1.set_profile(ShieldProfile::Essential);
            b_e.add_css_class("suggested-action");
            b_e.remove_css_class("flat");
            b_b.remove_css_class("suggested-action");
            b_b.add_css_class("flat");
            b_s.remove_css_class("suggested-action");
            b_s.add_css_class("flat");
        });

        let ab_p2 = adblock_engine.clone();
        let b2_e = p_ess.clone();
        let b2_b = p_bal.clone();
        let b2_s = p_str.clone();
        p_bal.connect_clicked(move |_| {
            ab_p2.set_profile(ShieldProfile::Balanced);
            b2_b.add_css_class("suggested-action");
            b2_b.remove_css_class("flat");
            b2_e.remove_css_class("suggested-action");
            b2_e.add_css_class("flat");
            b2_s.remove_css_class("suggested-action");
            b2_s.add_css_class("flat");
        });

        let ab_p3 = adblock_engine.clone();
        let b3_e = p_ess.clone();
        let b3_b = p_bal.clone();
        let b3_s = p_str.clone();
        p_str.connect_clicked(move |_| {
            ab_p3.set_profile(ShieldProfile::Strict);
            b3_s.add_css_class("suggested-action");
            b3_s.remove_css_class("flat");
            b3_e.remove_css_class("suggested-action");
            b3_e.add_css_class("flat");
            b3_b.remove_css_class("suggested-action");
            b3_b.add_css_class("flat");
        });

        profile_box.append(&p_ess);
        profile_box.append(&p_bal);
        profile_box.append(&p_str);
        content.append(&profile_box);

        // 4. Estatísticas Categorizadas Reais (Separadas por módulo)
        let stats = adblock_engine.stats().get_statistics();

        let stats_grid = GtkBox::new(Orientation::Horizontal, 8);
        stats_grid.set_margin_top(6);

        // Card Anúncios
        let c_ads = GtkBox::new(Orientation::Vertical, 2);
        c_ads.add_css_class("card");
        c_ads.set_hexpand(true);
        let c_ads_val = Label::builder().label(&stats.ads_blocked.to_string()).css_classes(["title-3", "bold", "accent"]).build();
        let c_ads_lbl = Label::builder().label("Anúncios").css_classes(["caption", "dim-label"]).build();
        c_ads.append(&c_ads_val);
        c_ads.append(&c_ads_lbl);

        // Card Rastreadores
        let c_tr = GtkBox::new(Orientation::Vertical, 2);
        c_tr.add_css_class("card");
        c_tr.set_hexpand(true);
        let c_tr_val = Label::builder().label(&stats.trackers_blocked.to_string()).css_classes(["title-3", "bold", "success"]).build();
        let c_tr_lbl = Label::builder().label("Rastreadores").css_classes(["caption", "dim-label"]).build();
        c_tr.append(&c_tr_val);
        c_tr.append(&c_tr_lbl);

        // Card Ameaças
        let c_th = GtkBox::new(Orientation::Vertical, 2);
        c_th.add_css_class("card");
        c_th.set_hexpand(true);
        let c_th_val = Label::builder().label(&stats.threats_detected.to_string()).css_classes(["title-3", "bold", "warning"]).build();
        let c_th_lbl = Label::builder().label("Ameaças").css_classes(["caption", "dim-label"]).build();
        c_th.append(&c_th_val);
        c_th.append(&c_th_lbl);

        // Card URLs Limpas
        let c_url = GtkBox::new(Orientation::Vertical, 2);
        c_url.add_css_class("card");
        c_url.set_hexpand(true);
        let c_url_val = Label::builder().label(&stats.urls_cleaned.to_string()).css_classes(["title-3", "bold"]).build();
        let c_url_lbl = Label::builder().label("URLs Limpas").css_classes(["caption", "dim-label"]).build();
        c_url.append(&c_url_val);
        c_url.append(&c_url_lbl);

        stats_grid.append(&c_ads);
        stats_grid.append(&c_tr);
        stats_grid.append(&c_th);
        stats_grid.append(&c_url);
        content.append(&stats_grid);

        // 5. Módulos Ativos do BruNya Shield
        let modules_lbl = Label::builder()
            .label("MÓDULOS DE PROTEÇÃO (10)")
            .css_classes(["heading", "caption", "dim-label"])
            .halign(Align::Start)
            .margin_top(8)
            .build();
        content.append(&modules_lbl);

        let modules_list = ListBox::builder()
            .selection_mode(SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();

        let cfg = adblock_engine.modules_config();

        let modules_defs = [
            ("AdBlock Nativo", "Bloqueio de publicidade e banners", cfg.adblock),
            ("Anti-Tracking", "Bloqueio de pixels, beacons e analytics", cfg.anti_tracking),
            ("Sites Maliciosos", "Proteção contra phishing e malware", cfg.malicious_sites),
            ("Anti-Popup", "Bloqueio de janelas e redirecionamentos", cfg.anti_popup),
            ("Anti-Annoyances", "Remoção de avisos de cookies e overlays", cfg.anti_annoyances),
            ("Anti-Fingerprinting", "Proteção de APIs de canvas e áudio", cfg.anti_fingerprinting),
            ("Anti-Crypto Mining", "Bloqueio de scripts de cryptojacking", cfg.anti_crypto_mining),
            ("URL Cleaner", "Remoção de parâmetros de rastreamento", cfg.url_cleaner),
            ("Cookies de Terceiros", "Bloqueio de cookies não particionados", cfg.third_party_cookies),
            ("Anúncios em Vídeo", "Bloqueio de publicidade em streams", cfg.video_ads),
        ];

        for (m_title, m_desc, is_active) in modules_defs {
            let row = ActionRow::builder()
                .title(m_title)
                .subtitle(m_desc)
                .build();
            let sw = Switch::builder()
                .active(is_active)
                .valign(Align::Center)
                .build();
            row.add_suffix(&sw);
            modules_list.append(&row);
        }

        let modules_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .max_content_height(160)
            .propagate_natural_height(true)
            .child(&modules_list)
            .build();

        content.append(&modules_scroll);

        // 6. Histórico Recente de Bloqueios nesta Aba
        let tab_entries = adblock_engine.stats().get_tab_entries(tab_id);
        if !tab_entries.is_empty() {
            let hist_lbl = Label::builder()
                .label("HISTÓRICO DESTA ABA")
                .css_classes(["heading", "caption", "dim-label"])
                .halign(Align::Start)
                .margin_top(6)
                .build();
            content.append(&hist_lbl);

            let hist_list = ListBox::builder()
                .selection_mode(SelectionMode::None)
                .css_classes(["boxed-list"])
                .build();

            for entry in tab_entries.iter().rev().take(4) {
                let row = ActionRow::builder()
                    .title(&entry.domain)
                    .subtitle(entry.category.display_name())
                    .build();
                hist_list.append(&row);
            }
            content.append(&hist_list);
        }

        popover.set_child(Some(&content));
        Self { popover }
    }
}
