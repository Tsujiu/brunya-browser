use brunya_core::types::TabId;
use brunya_tabs::TabDescriptor;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, GestureClick, Image, Label, Orientation,
    Popover, ScrolledWindow,
};
use std::collections::HashMap;

pub struct BrowserTabBar {
    pub container: ScrolledWindow,
    tab_box: GtkBox,
    tab_widgets: HashMap<TabId, (GtkBox, Label, Image)>,
}

impl BrowserTabBar {
    pub fn new() -> Self {
        let tab_box = GtkBox::new(Orientation::Horizontal, 4);
        tab_box.set_margin_start(4);
        tab_box.set_margin_end(4);
        tab_box.set_margin_top(2);
        tab_box.set_margin_bottom(2);
        tab_box.set_hexpand(true);

        let container = ScrolledWindow::builder()
            .vscrollbar_policy(gtk4::PolicyType::Never)
            .hscrollbar_policy(gtk4::PolicyType::Automatic)
            .hexpand(true)
            .child(&tab_box)
            .css_classes(["tab-bar"])
            .build();

        Self {
            container,
            tab_box,
            tab_widgets: HashMap::new(),
        }
    }

    pub fn render_tabs<FSwitch, FClose, FDup, FPin>(
        &mut self,
        tabs: &[TabDescriptor],
        active_id: Option<TabId>,
        favicons: &HashMap<TabId, Option<gdk4::Texture>>,
        on_switch: FSwitch,
        on_close: FClose,
        on_duplicate: FDup,
        on_pin: FPin,
    ) where
        FSwitch: Fn(TabId) + Clone + 'static,
        FClose: Fn(TabId) + Clone + 'static,
        FDup: Fn(TabId) + Clone + 'static,
        FPin: Fn(TabId) + Clone + 'static,
    {
        // Limpar widgets antigos
        while let Some(child) = self.tab_box.first_child() {
            self.tab_box.remove(&child);
        }
        self.tab_widgets.clear();

        let num_tabs = tabs.len();
        // Largura responsiva e dinâmica: quanto mais abas, menores elas ficam
        let target_width = if num_tabs <= 1 {
            240
        } else if num_tabs <= 3 {
            190
        } else if num_tabs <= 5 {
            145
        } else if num_tabs <= 8 {
            105
        } else if num_tabs <= 12 {
            75
        } else {
            48
        };

        for tab in tabs {
            let is_active = Some(tab.id) == active_id;

            let tab_pill = GtkBox::new(Orientation::Horizontal, 4);
            tab_pill.set_valign(Align::Center);
            tab_pill.set_hexpand(true);
            tab_pill.set_width_request(target_width);
            tab_pill.add_css_class("tab-pill");
            if is_active {
                tab_pill.add_css_class("active");
            }
            if tab.is_suspended {
                tab_pill.add_css_class("suspended");
            }

            // Ícone da aba: puxa o favicon real do site caso disponível
            let icon: Image = if let Some(Some(tex)) = favicons.get(&tab.id) {
                let img = Image::from_paintable(Some(tex));
                img.set_pixel_size(16);
                img
            } else if tab.is_pinned {
                let img = Image::from_icon_name("view-pin-symbolic");
                img.set_pixel_size(16);
                img
            } else if tab.is_suspended {
                let img = Image::from_icon_name("system-run-symbolic");
                img.set_pixel_size(16);
                img
            } else if tab.url.starts_with("brunya://") {
                let img = Image::from_icon_name("web-browser-symbolic");
                img.set_pixel_size(16);
                img
            } else {
                let img = Image::from_icon_name("text-html-symbolic");
                img.set_pixel_size(16);
                img
            };

            let display_title = if tab.title.is_empty() {
                "Nova Aba"
            } else {
                &tab.title
            };

            let show_label = target_width >= 70;
            let max_chars = if target_width >= 180 { 18 } else if target_width >= 120 { 11 } else { 6 };

            let label = Label::builder()
                .label(display_title)
                .max_width_chars(max_chars)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .hexpand(true)
                .halign(Align::Start)
                .visible(show_label)
                .build();

            // Botão de Seleção da Aba (Ícone + Título com hexpand para empurrar o X para o final)
            let select_btn = Button::builder()
                .css_classes(["flat", "tab-select-btn"])
                .hexpand(true)
                .build();

            let select_content = GtkBox::new(Orientation::Horizontal, 6);
            select_content.set_hexpand(true);
            select_content.append(&icon);
            select_content.append(&label);
            select_btn.set_child(Some(&select_content));

            let tid = tab.id;
            let switch_cb = on_switch.clone();
            select_btn.connect_clicked(move |_| {
                switch_cb(tid);
            });

            // Botão X de Fechamento fixado no final (extrema direita) da aba
            let close_btn = Button::builder()
                .icon_name("window-close-symbolic")
                .css_classes(["flat", "circular", "tab-close-btn"])
                .tooltip_text("Fechar aba")
                .halign(Align::End)
                .build();

            let close_cb = on_close.clone();
            close_btn.connect_clicked(move |_| {
                close_cb(tid);
            });

            // Gesto de Clique Direito no Tab Pill para Menu de Contexto
            let right_click = GestureClick::new();
            right_click.set_button(3); // Botão direito do mouse

            let dup_cb = on_duplicate.clone();
            let pin_cb = on_pin.clone();
            let close_context_cb = on_close.clone();
            let anchor = tab_pill.clone();

            right_click.connect_pressed(move |gesture, _n_press, _x, _y| {
                gesture.set_state(gtk4::EventSequenceState::Claimed);
                let popover = Popover::new();
                let menu_box = GtkBox::new(Orientation::Vertical, 4);
                menu_box.set_margin_top(4);
                menu_box.set_margin_bottom(4);
                menu_box.set_margin_start(4);
                menu_box.set_margin_end(4);

                let dup_btn = Button::builder().label("Duplicar Aba").css_classes(["flat"]).build();
                let d_cb = dup_cb.clone();
                let pop_c1 = popover.clone();
                dup_btn.connect_clicked(move |_| {
                    pop_c1.popdown();
                    d_cb(tid);
                });

                let pin_btn = Button::builder().label("Fixar / Desafixar Aba").css_classes(["flat"]).build();
                let p_cb = pin_cb.clone();
                let pop_c2 = popover.clone();
                pin_btn.connect_clicked(move |_| {
                    pop_c2.popdown();
                    p_cb(tid);
                });

                let close_action_btn = Button::builder().label("Fechar Aba").css_classes(["flat", "destructive-action"]).build();
                let c_cb = close_context_cb.clone();
                let pop_c3 = popover.clone();
                close_action_btn.connect_clicked(move |_| {
                    pop_c3.popdown();
                    c_cb(tid);
                });

                menu_box.append(&dup_btn);
                menu_box.append(&pin_btn);
                menu_box.append(&close_action_btn);
                popover.set_child(Some(&menu_box));
                popover.set_parent(&anchor);
                popover.popup();
            });

            tab_pill.add_controller(right_click);
            tab_pill.append(&select_btn);
            tab_pill.append(&close_btn);

            self.tab_box.append(&tab_pill);
            self.tab_widgets.insert(tab.id, (tab_pill, label, icon));
        }
    }
}
