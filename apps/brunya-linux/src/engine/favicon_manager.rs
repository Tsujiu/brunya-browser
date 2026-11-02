use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct FaviconManager;

impl FaviconManager {
    /// Extrai o domínio limpo a partir de uma URL (ex: "https://fast.com/pt/" -> "fast.com")
    pub fn extract_domain(url: &str) -> Option<String> {
        if let Ok(parsed) = url::Url::parse(url) {
            let host = parsed.host_str()?;
            let clean_host = host.strip_prefix("www.").unwrap_or(host);
            Some(clean_host.to_lowercase())
        } else {
            None
        }
    }

    /// Tenta carregar o favicon a partir do cache local ou ícones empacotados
    pub fn load_cached_favicon(domain: &str) -> Option<gdk::Texture> {
        let home = std::env::var("HOME").unwrap_or_default();
        let candidates = [
            format!("{}/.local/share/brunya/icons/favicons/{}.png", home, domain),
            format!("{}/.local/share/brunya/icons/engines/{}.png", home, domain),
            format!("/usr/share/brunya-browser/icons/engines/{}.png", domain),
            format!("assets/icons/engines/{}.png", domain),
        ];

        for path in &candidates {
            let p = std::path::Path::new(path);
            if p.exists() {
                let file = gio::File::for_path(p);
                if let Ok(texture) = gdk::Texture::from_file(&file) {
                    return Some(texture);
                }
            }
        }

        None
    }

    /// Busca o favicon de forma assíncrona em background e notifica na thread principal GTK
    pub fn fetch_favicon_async<F>(domain: String, on_loaded: F)
    where
        F: Fn(gdk::Texture) + 'static,
    {
        let home = std::env::var("HOME").unwrap_or_default();
        let cache_dir = PathBuf::from(format!("{}/.local/share/brunya/icons/favicons", home));
        let cache_file = cache_dir.join(format!("{}.png", domain));

        if cache_file.exists() {
            let file = gio::File::for_path(&cache_file);
            if let Ok(texture) = gdk::Texture::from_file(&file) {
                on_loaded(texture);
                return;
            }
        }

        let (sender, receiver) = mpsc::channel::<PathBuf>();
        let domain_clone = domain.clone();
        let cache_dest = cache_file.clone();

        std::thread::spawn(move || {
            let _ = fs::create_dir_all(&cache_dir);
            let url = format!("https://www.google.com/s2/favicons?domain={}&sz=32", domain_clone);

            let res = ureq::get(&url)
                .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
                .call();

            if let Ok(mut resp) = res {
                use std::io::Read;
                let mut data = Vec::new();
                if resp.body_mut().as_reader().read_to_end(&mut data).is_ok() && data.len() > 100 {
                    if fs::write(&cache_dest, &data).is_ok() {
                        let _ = sender.send(cache_dest);
                    }
                }
            }
        });

        let start_time = Instant::now();
        glib::timeout_add_local(Duration::from_millis(60), move || {
            if let Ok(path) = receiver.try_recv() {
                let file = gio::File::for_path(&path);
                if let Ok(texture) = gdk::Texture::from_file(&file) {
                    on_loaded(texture);
                }
                glib::ControlFlow::Break
            } else if start_time.elapsed() > Duration::from_secs(4) {
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }
}
