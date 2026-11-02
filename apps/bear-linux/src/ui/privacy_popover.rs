use bear_adblock::{AdblockEngine, ProtectionMode};
use bear_core::types::TabId;
use bear_i18n::t;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Label, ListBox, Orientation, Popover, ScrolledWindow,
    SelectionMode, Switch,
};
use libadwaita::ActionRow;
use std::sync::Arc;

pub struct PrivacyShieldPopover {
    pub popover: Popover,
}

impl PrivacyShieldPopover {
    pub fn new(adblock_engine: Arc<AdblockEngine>, tab_id: TabId, current_site: String) -> Self {
        let popover = Popover::new();

        let content = GtkBox::new(Orientation::Vertical, 12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(16);
        content.set_margin_end(16);
        content.set_width_request(340);

        let title_box = GtkBox::new(Orientation::Horizontal, 8);
        let title = Label::builder()
            .label(&t!("privacy.shield_title"))
            .css_classes(["title-4", "heading"])
            .hexpand(true)
            .halign(Align::Start)
            .build();

        title_box.append(&title);
        content.append(&title_box);

        let site_label = Label::builder()
            .label(&format!("Site: {}", if current_site.is_empty() { "Navegação Interna" } else { &current_site }))
            .css_classes(["caption", "dim-label"])
            .halign(Align::Start)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();
        content.append(&site_label);

        // Contador de bloqueios
        let count = adblock_engine.total_blocked();
        let count_label = Label::builder()
            .label(&t!("privacy.blocked_trackers_count", count => count))
            .css_classes(["bold", "success"])
            .halign(Align::Start)
            .build();
        content.append(&count_label);

        // Toggle do Escudo
        let toggle_box = GtkBox::new(Orientation::Horizontal, 8);
        let toggle_label = Label::builder()
            .label(&t!("privacy.shield_status_enabled"))
            .hexpand(true)
            .halign(Align::Start)
            .build();

        let is_allowed = adblock_engine.is_site_allowed(&current_site);
        let toggle = Switch::builder()
            .active(!is_allowed)
            .build();

        let ab_clone = adblock_engine.clone();
        let site_clone = current_site.clone();
        toggle.connect_active_notify(move |sw| {
            if sw.is_active() {
                // Ativo
            } else {
                ab_clone.allow_site(&site_clone);
            }
        });

        toggle_box.append(&toggle_label);
        toggle_box.append(&toggle);
        content.append(&toggle_box);

        // Seletor de Modo de Proteção: Balanceado vs Rigoroso
        let mode_label = Label::builder()
            .label(&t!("privacy.protection_mode"))
            .css_classes(["heading", "caption"])
            .halign(Align::Start)
            .margin_top(6)
            .build();
        content.append(&mode_label);

        let mode_box = GtkBox::new(Orientation::Horizontal, 6);
        let current_mode = adblock_engine.mode();

        let bal_btn = Button::builder()
            .label("Balanceado")
            .css_classes(["pill", if current_mode == ProtectionMode::Balanced { "suggested-action" } else { "flat" }])
            .build();

        let strict_btn = Button::builder()
            .label("Rigoroso")
            .css_classes(["pill", if current_mode == ProtectionMode::Strict { "suggested-action" } else { "flat" }])
            .build();

        let ab_bal = adblock_engine.clone();
        let b_btn = bal_btn.clone();
        let s_btn = strict_btn.clone();
        bal_btn.connect_clicked(move |_| {
            ab_bal.set_mode(ProtectionMode::Balanced);
            b_btn.add_css_class("suggested-action");
            b_btn.remove_css_class("flat");
            s_btn.remove_css_class("suggested-action");
            s_btn.add_css_class("flat");
        });

        let ab_str = adblock_engine.clone();
        let b2_btn = bal_btn.clone();
        let s2_btn = strict_btn.clone();
        strict_btn.connect_clicked(move |_| {
            ab_str.set_mode(ProtectionMode::Strict);
            s2_btn.add_css_class("suggested-action");
            s2_btn.remove_css_class("flat");
            b2_btn.remove_css_class("suggested-action");
            b2_btn.add_css_class("flat");
        });

        mode_box.append(&bal_btn);
        mode_box.append(&strict_btn);
        content.append(&mode_box);

        // Lista de Elementos Bloqueados nesta Aba
        let list_title = Label::builder()
            .label("Rastreadores Bloqueados nesta Página")
            .css_classes(["heading", "caption"])
            .halign(Align::Start)
            .margin_top(8)
            .build();
        content.append(&list_title);

        let tab_entries = adblock_engine.stats().get_tab_entries(tab_id);

        if tab_entries.is_empty() {
            let clean_label = Label::builder()
                .label("Nenhum rastreador detectado nesta página.")
                .css_classes(["dim-label", "caption"])
                .halign(Align::Start)
                .margin_bottom(8)
                .build();
            content.append(&clean_label);
        } else {
            let list_box = ListBox::builder()
                .selection_mode(SelectionMode::None)
                .css_classes(["boxed-list"])
                .build();

            for entry in tab_entries.iter().take(5) {
                let row = ActionRow::builder()
                    .title(&entry.domain)
                    .subtitle(entry.category.display_name())
                    .build();
                list_box.append(&row);
            }

            let scroll = ScrolledWindow::builder()
                .hscrollbar_policy(gtk4::PolicyType::Never)
                .vscrollbar_policy(gtk4::PolicyType::Automatic)
                .max_content_height(140)
                .propagate_natural_height(true)
                .child(&list_box)
                .build();

            content.append(&scroll);
        }

        popover.set_child(Some(&content));
        Self { popover }
    }
}
