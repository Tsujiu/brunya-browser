use crate::window::BearWindow;
use bear_adblock::AdblockEngine;
use bear_i18n::{t, I18nManager, Language};
use bear_settings::{SettingsManager, ThemePreference};
use bear_storage::StorageManager;
use bear_tabs::TabManager;
use chrono::Duration;
use gio::SimpleAction;
use gtk4::prelude::*;
use libadwaita::Application;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tracing::info;

pub struct BearApp {
    pub app: Application,
    tab_manager: Arc<TabManager>,
    adblock_engine: Arc<AdblockEngine>,
    storage_manager: Arc<StorageManager>,
    settings_manager: Arc<SettingsManager>,
    initial_urls: Vec<String>,
    main_window: RefCell<Option<Rc<BearWindow>>>,
}

impl BearApp {
    pub fn new(initial_urls: Vec<String>) -> Rc<Self> {
        let app = Application::builder()
            .application_id("org.bearbrowser.Bear")
            .build();

        // Inicializar armazenamento SQLite em ~/.local/share/bear/bear.db
        let data_dir = dirs_or_fallback();
        let db_path = data_dir.join("bear.db");
        let storage_manager = Arc::new(
            StorageManager::open(&db_path).unwrap_or_else(|_| StorageManager::open_in_memory().unwrap()),
        );

        // Inicializar configurações em ~/.config/bear/settings.json
        let config_path = config_dir_or_fallback().join("settings.json");
        let settings_manager = Arc::new(SettingsManager::load_or_default(&config_path));

        // Inicializar bloqueador de anúncios
        let adblock_engine = Arc::new(AdblockEngine::new());
        adblock_engine.set_mode(settings_manager.get().adblock_mode);

        // Gerenciador de abas
        let tab_manager = Arc::new(TabManager::new());

        // Aplicar preferências de idioma
        let lang = settings_manager.get().language;
        if lang != "system" {
            I18nManager::global().set_language(Language::from_locale_str(&lang));
        }

        let bear_app = Rc::new(Self {
            app,
            tab_manager,
            adblock_engine,
            storage_manager,
            settings_manager,
            initial_urls,
            main_window: RefCell::new(None),
        });

        bear_app.setup_signals();
        bear_app.setup_memory_saver_timer();
        bear_app
    }

    fn setup_signals(self: &Rc<Self>) {
        let sm = self.settings_manager.clone();

        self.app.connect_startup(move |_| {
            // Aplicar tema quando GTK/Adwaita estiver inicializado
            let theme = sm.get().theme;
            let style_mgr = libadwaita::StyleManager::default();
            match theme {
                ThemePreference::Light => style_mgr.set_color_scheme(libadwaita::ColorScheme::ForceLight),
                ThemePreference::Dark => style_mgr.set_color_scheme(libadwaita::ColorScheme::ForceDark),
                ThemePreference::System => style_mgr.set_color_scheme(libadwaita::ColorScheme::Default),
            }
        });

        let this_act = self.clone();
        self.app.connect_activate(move |app| {
            if this_act.main_window.borrow().is_none() {
                let win = BearWindow::new(
                    app,
                    this_act.tab_manager.clone(),
                    this_act.adblock_engine.clone(),
                    this_act.storage_manager.clone(),
                    this_act.settings_manager.clone(),
                );

                if !this_act.initial_urls.is_empty() {
                    for (i, url) in this_act.initial_urls.iter().enumerate() {
                        if i == 0 {
                            win.navigate_active_tab(url);
                        } else {
                            win.open_new_tab(url, url);
                        }
                    }
                }

                *this_act.main_window.borrow_mut() = Some(win.clone());
                win.present();
            } else if let Some(ref win) = *this_act.main_window.borrow() {
                win.present();
            }
        });

        // Configurar ações de menu
        let win_weak = Rc::downgrade(self);

        // Ação: Nova Aba
        let action_new_tab = SimpleAction::new("new_tab", None);
        let w1 = win_weak.clone();
        action_new_tab.connect_activate(move |_, _| {
            if let Some(app) = w1.upgrade() {
                if let Some(ref win) = *app.main_window.borrow() {
                    win.open_new_tab("bear://newtab", &t!("common.new_tab"));
                }
            }
        });
        self.app.add_action(&action_new_tab);

        // Ação: Histórico
        let action_hist = SimpleAction::new("open_history", None);
        let w2 = win_weak.clone();
        action_hist.connect_activate(move |_, _| {
            if let Some(app) = w2.upgrade() {
                if let Some(ref win) = *app.main_window.borrow() {
                    win.open_new_tab("bear://history", &t!("common.history"));
                }
            }
        });
        self.app.add_action(&action_hist);

        // Ação: Configurações
        let action_cfg = SimpleAction::new("open_settings", None);
        let w3 = win_weak.clone();
        action_cfg.connect_activate(move |_, _| {
            if let Some(app) = w3.upgrade() {
                if let Some(ref win) = *app.main_window.borrow() {
                    win.open_new_tab("bear://settings", &t!("common.settings"));
                }
            }
        });
        self.app.add_action(&action_cfg);

        // Ação: Sobre
        let action_about = SimpleAction::new("open_about", None);
        let w4 = win_weak.clone();
        action_about.connect_activate(move |_, _| {
            if let Some(app) = w4.upgrade() {
                if let Some(ref win) = *app.main_window.borrow() {
                    win.open_new_tab("bear://about", &t!("common.about"));
                }
            }
        });
        self.app.add_action(&action_about);
    }

    /// Timer de economia inteligente de memória: a cada 60s inspeciona abas inativas
    fn setup_memory_saver_timer(self: &Rc<Self>) {
        let tm = self.tab_manager.clone();
        let sm = self.settings_manager.clone();

        glib::timeout_add_seconds_local(60, move || {
            let cfg = sm.get();
            if cfg.tab_suspension_enabled && cfg.tab_suspension_timeout_minutes > 0 {
                let dur = Duration::minutes(cfg.tab_suspension_timeout_minutes as i64);
                let suspended = tm.suspend_inactive_tabs(dur);
                if !suspended.is_empty() {
                    info!("BEAR Economizador de Memória: {} abas suspensas", suspended.len());
                }
            }
            glib::ControlFlow::Continue
        });
    }

    pub fn run(&self) -> glib::ExitCode {
        self.app.run_with_args::<&str>(&[])
    }
}

fn dirs_or_fallback() -> std::path::PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        std::path::PathBuf::from(home).join(".local/share/bear")
    } else {
        std::path::PathBuf::from("/tmp/bear")
    }
}

fn config_dir_or_fallback() -> std::path::PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        std::path::PathBuf::from(home).join(".config/bear")
    } else {
        std::path::PathBuf::from("/tmp/bear/config")
    }
}
