use bear_core::types::TabId;
use bear_tabs::TabDescriptor;
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

        let container = ScrolledWindow::builder()
            .vscrollbar_policy(gtk4::PolicyType::Never)
            .hscrollbar_policy(gtk4::PolicyType::Automatic)
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

        for tab in tabs {
            let is_active = Some(tab.id) == active_id;

            let tab_item = GtkBox::new(Orientation::Horizontal, 6);
            tab_item.set_valign(Align::Center);
            tab_item.set_margin_start(2);
            tab_item.set_margin_end(2);

            let mut classes = vec!["tab-button", "flat"];
            if is_active {
                classes.push("active-tab");
            }
            if tab.is_suspended {
                classes.push("dim-label");
            }

            let tab_btn = Button::builder()
                .css_classes(classes)
                .build();

            // Ícone da aba
            let icon_name = if tab.is_pinned {
                "view-pin-symbolic"
            } else if tab.is_suspended {
                "system-run-symbolic" // Indicador de suspensão de memória
            } else if tab.url.starts_with("bear://") {
                "security-high-symbolic"
            } else {
                "text-html-symbolic"
            };

            let icon = Image::from_icon_name(icon_name);
            let display_title = if tab.title.is_empty() {
                "Nova Aba"
            } else {
                &tab.title
            };

            let label = Label::builder()
                .label(display_title)
                .max_width_chars(22)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .build();

            let close_btn = Button::builder()
                .icon_name("window-close-symbolic")
                .css_classes(["flat", "circular"])
                .tooltip_text("Fechar aba")
                .build();

            let tid = tab.id;
            let close_cb = on_close.clone();
            close_btn.connect_clicked(move |_| {
                close_cb(tid);
            });

            let switch_cb = on_switch.clone();
            tab_btn.connect_clicked(move |_| {
                switch_cb(tid);
            });

            // Gesto de Clique Direito (Menu de Contexto de Aba)
            let right_click = GestureClick::new();
            right_click.set_button(3); // Botão direito do mouse

            let dup_cb = on_duplicate.clone();
            let pin_cb = on_pin.clone();
            let close_context_cb = on_close.clone();
            let tab_btn_anchor = tab_btn.clone();

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
                popover.set_parent(&tab_btn_anchor);
                popover.popup();
            });

            tab_btn.add_controller(right_click);

            tab_item.append(&icon);
            tab_item.append(&label);
            tab_item.append(&close_btn);
            tab_btn.set_child(Some(&tab_item));

            self.tab_box.append(&tab_btn);
            self.tab_widgets.insert(tab.id, (tab_item, label, icon));
        }
    }
}
