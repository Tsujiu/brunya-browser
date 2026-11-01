use bear_core::types::TabId;
use bear_tabs::TabDescriptor;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Image, Label, Orientation, ScrolledWindow};
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

    pub fn render_tabs<FSwitch, FClose>(
        &mut self,
        tabs: &[TabDescriptor],
        active_id: Option<TabId>,
        on_switch: FSwitch,
        on_close: FClose,
    ) where
        FSwitch: Fn(TabId) + Clone + 'static,
        FClose: Fn(TabId) + Clone + 'static,
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
            let icon_name = if tab.is_suspended {
                "system-run-symbolic" // Indicador de suspensão
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
                .max_width_chars(20)
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

            tab_item.append(&icon);
            tab_item.append(&label);
            tab_item.append(&close_btn);
            tab_btn.set_child(Some(&tab_item));

            self.tab_box.append(&tab_btn);
            self.tab_widgets.insert(tab.id, (tab_item, label, icon));
        }
    }
}
