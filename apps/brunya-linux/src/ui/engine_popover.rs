use brunya_settings::search_engines::{SearchEngineDef, ALL_SEARCH_ENGINES};
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Entry, Image, Label, ListBox, Orientation, Popover,
    ScrolledWindow, SelectionMode,
};
use libadwaita::prelude::*;
use libadwaita::ActionRow;
use std::cell::RefCell;
use std::rc::Rc;

pub fn get_engine_icon(engine_id: &str) -> gtk4::Widget {
    // 1. Verificar caminhos locais no sistema ou desenvolvimento
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [
        format!("{}/.local/share/brunya/icons/engines/{}.png", home, engine_id),
        format!("/usr/share/brunya-browser/icons/engines/{}.png", engine_id),
        format!("assets/icons/engines/{}.png", engine_id),
    ];

    for path in &candidates {
        let p = std::path::Path::new(path);
        if p.exists() {
            let file = gio::File::for_path(p);
            if let Ok(texture) = gdk4::Texture::from_file(&file) {
                let img = Image::from_paintable(Some(&texture));
                img.set_pixel_size(18);
                return img.upcast();
            }
        }
    }

    // Fallback elegante para texto com glifo
    let def = brunya_settings::search_engines::get_engine_by_id(engine_id);
    let glyph = def.map(|d| d.glyph).unwrap_or("🔍");
    let lbl = Label::builder().label(glyph).css_classes(["bold", "caption"]).build();
    lbl.upcast()
}

pub struct SearchEnginePopover {
    pub popover: Popover,
}

impl SearchEnginePopover {
    pub fn new<F: Fn(SearchEngineDef) + 'static>(on_select: F) -> Self {
        let popover = Popover::new();

        let content = GtkBox::new(Orientation::Vertical, 8);
        content.set_margin_top(10);
        content.set_margin_bottom(10);
        content.set_margin_start(10);
        content.set_margin_end(10);
        content.set_width_request(340);

        let title_label = Label::builder()
            .label("Motores de Busca")
            .css_classes(["heading", "caption"])
            .halign(Align::Start)
            .margin_bottom(4)
            .build();
        content.append(&title_label);

        // Campo de busca em tempo real para os 66 motores
        let search_filter = Entry::builder()
            .placeholder_text("Filtrar motores de busca...")
            .build();
        content.append(&search_filter);

        let list_box = ListBox::builder()
            .selection_mode(SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();

        let on_select_rc = Rc::new(on_select);
        let popover_weak = popover.clone();

        let row_entries = Rc::new(RefCell::new(Vec::<(ActionRow, SearchEngineDef)>::new()));

        for engine in ALL_SEARCH_ENGINES {
            let row = ActionRow::builder()
                .title(engine.name)
                .subtitle(engine.category)
                .activatable(true)
                .build();

            // Puxa o real ícone do motor de busca
            let icon_widget = get_engine_icon(engine.id);
            row.add_prefix(&icon_widget);

            let sel_cb = on_select_rc.clone();
            let pop_clone = popover_weak.clone();
            let eng = *engine;
            row.connect_activated(move |_| {
                sel_cb(eng);
                pop_clone.popdown();
            });

            row_entries.borrow_mut().push((row.clone(), *engine));
            list_box.append(&row);
        }

        // Filtragem instantânea ao digitar
        let entries_filter = row_entries;
        search_filter.connect_changed(move |entry| {
            let q = entry.text().to_lowercase();
            for (row, eng) in entries_filter.borrow().iter() {
                let matches = q.is_empty()
                    || eng.name.to_lowercase().contains(&q)
                    || eng.category.to_lowercase().contains(&q);
                row.set_visible(matches);
            }
        });

        let scroll = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .max_content_height(280)
            .propagate_natural_height(true)
            .child(&list_box)
            .build();

        content.append(&scroll);
        popover.set_child(Some(&content));

        Self { popover }
    }
}
