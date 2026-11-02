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
    pub address_entry: Entry,
    pub bookmark_btn: Button,
    pub shield_btn: Button,
    pub shield_icon: Image,
    pub shield_label: Label,
    pub shield_white_tex: Option<gdk4::Texture>,
    pub shield_black_tex: Option<gdk4::Texture>,
    pub theme_toggle_btn: Button,
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
            .css_classes(["flat", "circular", "nav-btn"])
            .sensitive(false)
            .build();

        let forward_btn = Button::builder()
            .icon_name("go-next-symbolic")
            .tooltip_text(&t!("common.forward"))
            .css_classes(["flat", "circular", "nav-btn"])
            .sensitive(false)
            .build();

        let reload_btn = Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text(&t!("common.reload"))
            .css_classes(["flat", "circular", "nav-btn"])
            .build();

        let stop_btn = Button::builder()
            .icon_name("process-stop-symbolic")
            .tooltip_text(&t!("common.stop"))
            .css_classes(["flat", "circular", "nav-btn"])
            .visible(false)
            .build();

        nav_box.append(&back_btn);
        nav_box.append(&forward_btn);
        nav_box.append(&reload_btn);
        nav_box.append(&stop_btn);
        container.pack_start(&nav_box);

        // 2. Barra de Endereço Central (com Favorito e Escudo Único Oficial à direita)
        let address_box = GtkBox::new(Orientation::Horizontal, 4);
        address_box.set_hexpand(true);
        address_box.set_margin_start(16);
        address_box.set_margin_end(16);
        address_box.add_css_class("address-container");

        let address_entry = Entry::builder()
            .placeholder_text(&t!("common.address_placeholder"))
            .hexpand(true)
            .css_classes(["address-entry"])
            .build();

        let bookmark_btn = Button::builder()
            .icon_name("non-starred-symbolic")
            .tooltip_text(&t!("common.add_bookmark"))
            .css_classes(["flat", "circular", "omnibox-btn"])
            .build();

        // Carregar as duas versões do escudo oficial: branco para modo escuro, preto para modo claro
        let white_bytes = glib::Bytes::from_static(include_bytes!("../../../../assets/icons/brunya-shield-white.svg"));
        let shield_white_tex = gdk4::Texture::from_bytes(&white_bytes).ok();

        let black_bytes = glib::Bytes::from_static(include_bytes!("../../../../assets/icons/brunya-shield.svg"));
        let shield_black_tex = gdk4::Texture::from_bytes(&black_bytes).ok();

        let shield_btn = Button::builder()
            .css_classes(["flat", "circular", "omnibox-btn"])
            .tooltip_text(&t!("privacy.shield_title"))
            .build();

        let initial_dark = libadwaita::StyleManager::default().is_dark();
        let initial_tex = if initial_dark {
            shield_white_tex.as_ref()
        } else {
            shield_black_tex.as_ref()
        };

        let shield_icon = if let Some(tex) = initial_tex {
            let img = Image::from_paintable(Some(tex));
            img.set_pixel_size(13);
            img
        } else {
            Image::from_icon_name("security-high-symbolic")
        };

        let shield_label = Label::builder().label("0").build();

        // Apenas o ícone puro do escudo: nada em volta, nenhum contador
        shield_btn.set_child(Some(&shield_icon));

        // Barra de navegação com entrada de texto, favorito e somente o ícone do escudo
        address_box.append(&address_entry);
        address_box.append(&bookmark_btn);
        address_box.append(&shield_btn);
        container.set_title_widget(Some(&address_box));

        // 3. Botões da Direita: Downloads, Alternador de Tema Sol/Lua, Nova Aba e Menu Principal
        let right_box = GtkBox::new(Orientation::Horizontal, 4);

        let downloads_btn = Button::builder()
            .icon_name("folder-download-symbolic")
            .tooltip_text(&t!("common.downloads"))
            .css_classes(["flat", "circular", "nav-btn"])
            .build();

        let theme_toggle_btn = Button::builder()
            .icon_name(if initial_dark { "display-brightness-symbolic" } else { "weather-clear-night-symbolic" })
            .tooltip_text(if initial_dark { "Mudar para tema claro" } else { "Mudar para tema escuro" })
            .css_classes(["flat", "circular", "nav-btn"])
            .build();

        let new_tab_btn = Button::builder()
            .icon_name("tab-new-symbolic")
            .tooltip_text(&t!("common.new_tab"))
            .css_classes(["flat", "circular", "nav-btn"])
            .build();

        let menu_btn = MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Menu do Brunya")
            .css_classes(["flat", "circular", "nav-btn"])
            .build();

        right_box.append(&downloads_btn);
        right_box.append(&theme_toggle_btn);
        right_box.append(&new_tab_btn);
        right_box.append(&menu_btn);
        container.pack_end(&right_box);

        Self {
            container,
            back_btn,
            forward_btn,
            reload_btn,
            stop_btn,
            address_entry,
            bookmark_btn,
            shield_btn,
            shield_icon,
            shield_label,
            shield_white_tex,
            shield_black_tex,
            theme_toggle_btn,
            downloads_btn,
            new_tab_btn,
            menu_btn,
        }
    }

    pub fn update_theme_icons(&self, is_dark: bool) {
        if is_dark {
            if let Some(ref tex) = self.shield_white_tex {
                self.shield_icon.set_paintable(Some(tex));
            }
            self.theme_toggle_btn.set_icon_name("display-brightness-symbolic");
            self.theme_toggle_btn.set_tooltip_text(Some("Mudar para tema claro"));
        } else {
            if let Some(ref tex) = self.shield_black_tex {
                self.shield_icon.set_paintable(Some(tex));
            }
            self.theme_toggle_btn.set_icon_name("weather-clear-night-symbolic");
            self.theme_toggle_btn.set_tooltip_text(Some("Mudar para tema escuro"));
        }
    }

    pub fn set_security_level(&self, _level: SecurityLevel) {
        // Mantido apenas o escudo oficial à direita
    }

    pub fn set_blocked_count(&self, count: u64) {
        self.shield_label.set_label(&count.to_string());
    }

    pub fn set_loading(&self, is_loading: bool) {
        self.reload_btn.set_visible(!is_loading);
        self.stop_btn.set_visible(is_loading);
    }
}
