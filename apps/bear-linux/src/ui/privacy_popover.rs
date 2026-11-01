use bear_adblock::AdblockEngine;
use bear_i18n::t;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Label, Orientation, Popover, Switch};
use std::sync::Arc;

pub struct PrivacyShieldPopover {
    pub popover: Popover,
}

impl PrivacyShieldPopover {
    pub fn new(adblock_engine: Arc<AdblockEngine>, current_site: String) -> Self {
        let popover = Popover::new();

        let content = GtkBox::new(Orientation::Vertical, 12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(16);
        content.set_margin_end(16);

        let title = Label::builder()
            .label(&t!("privacy.shield_title"))
            .css_classes(["title-4", "heading"])
            .halign(Align::Start)
            .build();
        content.append(&title);

        let site_label = Label::builder()
            .label(&format!("Site: {}", current_site))
            .css_classes(["caption", "dim-label"])
            .halign(Align::Start)
            .build();
        content.append(&site_label);

        // Contador de bloqueios
        let count = adblock_engine.total_blocked();
        let count_label = Label::builder()
            .label(&t!("privacy.blocked_trackers_count", count => count))
            .css_classes(["bold"])
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
                // Ativar proteção para o site (remover da lista de permitidos)
                // Na lógica simples do adblock:
            } else {
                ab_clone.allow_site(&site_clone);
            }
        });

        toggle_box.append(&toggle_label);
        toggle_box.append(&toggle);
        content.append(&toggle_box);

        popover.set_child(Some(&content));

        Self { popover }
    }
}
