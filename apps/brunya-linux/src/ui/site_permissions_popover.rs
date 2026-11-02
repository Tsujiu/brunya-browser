use brunya_i18n::t;
use brunya_privacy::PermissionType;
use brunya_storage::{PermissionState, StorageManager};
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation, Popover};
use std::sync::Arc;

pub struct SitePermissionsPopover {
    pub popover: Popover,
}

impl SitePermissionsPopover {
    pub fn new(
        storage_mgr: Arc<StorageManager>,
        origin: String,
        is_secure: bool,
    ) -> Self {
        let popover = Popover::new();

        let content = GtkBox::new(Orientation::Vertical, 12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(16);
        content.set_margin_end(16);
        content.set_width_request(280);

        // Cabeçalho de Segurança
        let status_text = if origin.starts_with("brunya://") {
            "Página Interna Segura".to_string()
        } else if is_secure {
            "Conexão Segura (HTTPS)".to_string()
        } else {
            t!("common.not_secure")
        };

        let status_label = Label::builder()
            .label(status_text)
            .css_classes(["bold", if is_secure { "success" } else { "warning" }])
            .halign(Align::Start)
            .build();
        content.append(&status_label);

        let origin_label = Label::builder()
            .label(&origin)
            .css_classes(["caption", "dim-label"])
            .halign(Align::Start)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();
        content.append(&origin_label);

        // Permissões do Site
        let perms_title = Label::builder()
            .label(&t!("privacy.site_permissions"))
            .css_classes(["heading", "caption"])
            .halign(Align::Start)
            .margin_top(8)
            .build();
        content.append(&perms_title);

        let perms = [
            (PermissionType::Camera, &t!("privacy.camera")),
            (PermissionType::Microphone, &t!("privacy.microphone")),
            (PermissionType::Geolocation, &t!("privacy.location")),
            (PermissionType::Notifications, &t!("privacy.notifications")),
        ];

        for (perm_type, label_text) in perms {
            let row = GtkBox::new(Orientation::Horizontal, 8);
            let lbl = Label::builder()
                .label(label_text)
                .hexpand(true)
                .halign(Align::Start)
                .build();

            let current_state = storage_mgr
                .get_site_permission(&origin, perm_type.as_str())
                .unwrap_or(PermissionState::Ask);

            let state_btn = Button::builder()
                .label(match current_state {
                    PermissionState::Allow => "Permitir",
                    PermissionState::Block => "Bloquear",
                    PermissionState::Ask => "Perguntar",
                })
                .css_classes(["pill", "flat"])
                .build();

            let st_clone = storage_mgr.clone();
            let orig_clone = origin.clone();
            let perm_str = perm_type.as_str().to_string();

            state_btn.connect_clicked(move |btn| {
                let next_state = match btn.label().as_deref() {
                    Some("Perguntar") => PermissionState::Allow,
                    Some("Permitir") => PermissionState::Block,
                    _ => PermissionState::Ask,
                };
                st_clone.set_site_permission(&orig_clone, &perm_str, next_state).ok();
                btn.set_label(match next_state {
                    PermissionState::Allow => "Permitir",
                    PermissionState::Block => "Bloquear",
                    PermissionState::Ask => "Perguntar",
                });
            });

            row.append(&lbl);
            row.append(&state_btn);
            content.append(&row);
        }

        popover.set_child(Some(&content));
        Self { popover }
    }
}
