use brunya_core::types::SecurityLevel;
use brunya_i18n::t;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Entry, Image, Label, MenuButton, Orientation};
use libadwaita::HeaderBar;

pub struct BrowserHeader {
    pub container: HeaderBar,
    pub back_btn: Button,
    pub forward_btn: Button,
    pub reload_btn: Button,
    pub stop_btn: Button,
    pub security_btn: Button,
    pub address_entry: Entry,
    pub bookmark_btn: Button,
    pub shield_btn: Button,
    pub shield_label: Label,
    pub downloads_btn: Button,
    pub new_tab_btn: Button,
    pub menu_btn: MenuButton,
}

impl BrowserHeader {
    pub fn new() -> Self {
        let container = HeaderBar::new();

        // 1. Botões de Navegação (Esquerda)
        let nav_box = GtkBox::new(Orientation::Horizontal, 4);

        let back_btn = Button::builder()
            .icon_name("go-previous-symbolic")
            .tooltip_text(&t!("common.back"))
            .sensitive(false)
            .build();

        let forward_btn = Button::builder()
            .icon_name("go-next-symbolic")
            .tooltip_text(&t!("common.forward"))
            .sensitive(false)
            .build();

        let reload_btn = Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text(&t!("common.reload"))
            .build();

        let stop_btn = Button::builder()
            .icon_name("process-stop-symbolic")
            .tooltip_text(&t!("common.stop"))
            .visible(false)
            .build();

        nav_box.append(&back_btn);
        nav_box.append(&forward_btn);
        nav_box.append(&reload_btn);
        nav_box.append(&stop_btn);
        container.pack_start(&nav_box);

        // 2. Barra de Endereço Central com Indicador de Segurança, Favorito e Escudo
        let address_box = GtkBox::new(Orientation::Horizontal, 0);
        address_box.set_hexpand(true);
        address_box.set_margin_start(16);
        address_box.set_margin_end(16);
        address_box.add_css_class("linked");

        let security_btn = Button::builder()
            .icon_name(SecurityLevel::Internal.icon_name())
            .tooltip_text("Verificar segurança da conexão")
            .css_classes(["flat"])
            .build();

        let address_entry = Entry::builder()
            .placeholder_text(&t!("common.address_placeholder"))
            .hexpand(true)
            .build();

        let bookmark_btn = Button::builder()
            .icon_name("non-starred-symbolic")
            .tooltip_text(&t!("common.add_bookmark"))
            .css_classes(["flat"])
            .build();

        // Botão do Escudo de Privacidade com Contador de Rastreadores Bloqueados
        let shield_btn = Button::builder()
            .css_classes(["flat"])
            .tooltip_text(&t!("privacy.shield_title"))
            .build();

        let shield_content = GtkBox::new(Orientation::Horizontal, 4);
        let shield_icon = Image::from_icon_name("security-high-symbolic");
        let shield_label = Label::builder()
            .label("0")
            .css_classes(["numeric", "caption"])
            .build();

        shield_content.append(&shield_icon);
        shield_content.append(&shield_label);
        shield_btn.set_child(Some(&shield_content));

        address_box.append(&security_btn);
        address_box.append(&address_entry);
        address_box.append(&bookmark_btn);
        address_box.append(&shield_btn);
        container.set_title_widget(Some(&address_box));

        // 3. Botões da Direita: Downloads, Nova Aba e Menu Principal
        let right_box = GtkBox::new(Orientation::Horizontal, 4);

        let downloads_btn = Button::builder()
            .icon_name("folder-download-symbolic")
            .tooltip_text(&t!("common.downloads"))
            .build();

        let new_tab_btn = Button::builder()
            .icon_name("tab-new-symbolic")
            .tooltip_text(&t!("common.new_tab"))
            .build();

        let menu_btn = MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Menu do Brunya")
            .build();

        right_box.append(&downloads_btn);
        right_box.append(&new_tab_btn);
        right_box.append(&menu_btn);
        container.pack_end(&right_box);

        Self {
            container,
            back_btn,
            forward_btn,
            reload_btn,
            stop_btn,
            security_btn,
            address_entry,
            bookmark_btn,
            shield_btn,
            shield_label,
            downloads_btn,
            new_tab_btn,
            menu_btn,
        }
    }

    pub fn set_security_level(&self, level: SecurityLevel) {
        self.security_btn.set_icon_name(level.icon_name());
    }

    pub fn set_blocked_count(&self, count: u64) {
        self.shield_label.set_label(&count.to_string());
    }

    pub fn set_loading(&self, is_loading: bool) {
        self.reload_btn.set_visible(!is_loading);
        self.stop_btn.set_visible(is_loading);
    }
}
