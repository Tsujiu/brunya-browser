use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Entry, Label, Orientation};
use std::rc::Rc;

pub struct FindBar {
    pub container: GtkBox,
    pub entry: Entry,
    pub count_label: Label,
    pub prev_btn: Button,
    pub next_btn: Button,
    pub close_btn: Button,
}

impl FindBar {
    pub fn new() -> Rc<Self> {
        let container = GtkBox::new(Orientation::Horizontal, 6);
        container.set_margin_start(16);
        container.set_margin_end(16);
        container.set_margin_top(4);
        container.set_margin_bottom(4);
        container.add_css_class("card");
        container.set_visible(false);

        let entry = Entry::builder()
            .placeholder_text("Localizar na página...")
            .width_request(240)
            .build();

        let count_label = Label::builder()
            .label("")
            .css_classes(["caption", "dim-label"])
            .margin_start(4)
            .margin_end(4)
            .build();

        let prev_btn = Button::builder()
            .icon_name("go-up-symbolic")
            .tooltip_text("Anterior")
            .css_classes(["flat", "circular"])
            .build();

        let next_btn = Button::builder()
            .icon_name("go-down-symbolic")
            .tooltip_text("Próxima")
            .css_classes(["flat", "circular"])
            .build();

        let close_btn = Button::builder()
            .icon_name("window-close-symbolic")
            .tooltip_text("Fechar")
            .css_classes(["flat", "circular"])
            .build();

        container.append(&entry);
        container.append(&count_label);
        container.append(&prev_btn);
        container.append(&next_btn);
        container.append(&close_btn);

        let bar = Rc::new(Self {
            container,
            entry,
            count_label,
            prev_btn,
            next_btn,
            close_btn,
        });

        let this_close = bar.clone();
        bar.close_btn.connect_clicked(move |_| {
            this_close.hide();
        });

        bar
    }

    pub fn show(&self) {
        self.container.set_visible(true);
        self.entry.grab_focus();
        self.entry.select_region(0, -1);
    }

    pub fn hide(&self) {
        self.container.set_visible(false);
        self.entry.set_text("");
        self.count_label.set_label("");
    }

    pub fn toggle(&self) {
        if self.container.is_visible() {
            self.hide();
        } else {
            self.show();
        }
    }
}
