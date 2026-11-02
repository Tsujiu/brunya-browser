use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Label, Orientation, ScrolledWindow};

/// Constrói a página informativa "brunya://private" (Modo Privado com limites explicados).
pub fn build_private_info_view() -> gtk4::Widget {
    let container = GtkBox::new(Orientation::Vertical, 16);
    container.set_valign(Align::Center);
    container.set_halign(Align::Center);
    container.set_vexpand(true);
    container.set_hexpand(true);
    container.set_margin_top(48);
    container.set_margin_bottom(48);
    container.set_margin_start(32);
    container.set_margin_end(32);

    let icon = Label::builder()
        .label("🕶️")
        .css_classes(["display-1"])
        .build();

    let title = Label::builder()
        .label("Você está em uma Janela Privada")
        .css_classes(["title-1", "heading"])
        .build();

    let subtitle = Label::builder()
        .label("O Brunya Browser não salvará seu histórico, cookies ou dados de formulários.")
        .css_classes(["title-4", "dim-label"])
        .build();

    let card = GtkBox::new(Orientation::Vertical, 12);
    card.set_margin_top(24);
    card.add_css_class("card");
    card.set_width_request(550);

    let what_is_saved_title = Label::builder()
        .label("O que o Modo Privado faz:")
        .css_classes(["heading", "bold"])
        .halign(Align::Start)
        .build();

    let what_is_saved_desc = Label::builder()
        .label("• As páginas visitadas não serão salvas no seu histórico local.\n• Cookies e dados de sites temporários serão apagados assim que você fechar todas as janelas privadas.\n• O Escudo de Privacidade continua ativo bloqueando anúncios e rastreadores.")
        .wrap(true)
        .halign(Align::Start)
        .build();

    let limits_title = Label::builder()
        .label("O que o Modo Privado NÃO faz:")
        .css_classes(["heading", "bold", "warning"])
        .halign(Align::Start)
        .build();

    let limits_desc = Label::builder()
        .label("• Não torna sua navegação anônima na internet.\n• Seu provedor de internet (ISP), empregador ou administradores de rede ainda podem identificar os sites visitados.\n• Downloads realizados permanecerão salvos na pasta Downloads do seu computador.")
        .wrap(true)
        .halign(Align::Start)
        .build();

    card.append(&what_is_saved_title);
    card.append(&what_is_saved_desc);
    card.append(&limits_title);
    card.append(&limits_desc);

    container.append(&icon);
    container.append(&title);
    container.append(&subtitle);
    container.append(&card);

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .child(&container)
        .build();

    scroll.upcast()
}
