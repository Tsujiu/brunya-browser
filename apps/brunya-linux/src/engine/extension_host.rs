//! Ponte entre as extensões instaladas e o WebKit.
//!
//! Serve os arquivos da extensão em `brunya-extension://`, injeta scripts de
//! conteúdo, abre popups e responde ao polyfill `chrome.*`.

use brunya_extensions::inject::{
    background_page_relative, best_icon_file, content_script_matches, content_script_source,
    extension_page_url, generated_background_html, has_background, normalized_run_at,
    polyfill_source, url_matches, world_name,
};
use brunya_extensions::storage::StorageArea;
use brunya_extensions::types::{ExtensionId, ExtensionInfo, ExtensionState};
use brunya_extensions::ExtensionSystem;
use gio::prelude::*;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::{Arc, Once};
use tracing::warn;
use webkit6::prelude::*;
use webkit6::{LoadEvent, UserContentInjectedFrames, UserScript, UserScriptInjectionTime, WebView};

#[derive(Clone)]
pub struct ExtensionTab {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub active: bool,
    pub index: u32,
}

struct ReplyTarget {
    view: WebView,
    world: Option<String>,
    js_id: i64,
}

pub struct ExtensionBridge {
    system: Arc<ExtensionSystem>,
    host: gtk4::Box,
    backgrounds: RefCell<HashMap<String, WebView>>,
    pending_install: RefCell<HashSet<String>>,
    replies: RefCell<HashMap<String, ReplyTarget>>,
    injected: RefCell<HashMap<String, HashSet<String>>>,
    view_tabs: RefCell<HashMap<String, u64>>,
    tabs: RefCell<Option<Rc<dyn Fn() -> Vec<ExtensionTab>>>>,
    open_url: RefCell<Option<Rc<dyn Fn(String)>>>,
    navigate: RefCell<Option<Rc<dyn Fn(u64, String)>>>,
}

impl ExtensionBridge {
    pub fn new(system: Arc<ExtensionSystem>, host: gtk4::Box) -> Rc<Self> {
        register_extension_scheme(system.clone());
        Rc::new(Self {
            system,
            host,
            backgrounds: RefCell::new(HashMap::new()),
            pending_install: RefCell::new(HashSet::new()),
            replies: RefCell::new(HashMap::new()),
            injected: RefCell::new(HashMap::new()),
            view_tabs: RefCell::new(HashMap::new()),
            tabs: RefCell::new(None),
            open_url: RefCell::new(None),
            navigate: RefCell::new(None),
        })
    }

    pub fn set_browser_controls(
        &self,
        tabs: Rc<dyn Fn() -> Vec<ExtensionTab>>,
        open_url: Rc<dyn Fn(String)>,
        navigate: Rc<dyn Fn(u64, String)>,
    ) {
        *self.tabs.borrow_mut() = Some(tabs);
        *self.open_url.borrow_mut() = Some(open_url);
        *self.navigate.borrow_mut() = Some(navigate);
    }

    pub fn note_installed(self: &Rc<Self>, ext_id: &str) {
        self.pending_install.borrow_mut().insert(ext_id.to_string());
        if let Some(view) = self.backgrounds.borrow_mut().remove(ext_id) {
            self.host.remove(&view);
        }
    }

    pub fn sync_backgrounds(self: &Rc<Self>) {
        let active = self.enabled_extensions();
        let wanted: HashSet<String> = active
            .iter()
            .filter(|ext| has_background(ext))
            .map(|ext| ext.id.0.clone())
            .collect();
        let stale: Vec<String> = self
            .backgrounds
            .borrow()
            .keys()
            .filter(|id| !wanted.contains(*id))
            .cloned()
            .collect();
        for id in stale {
            if let Some(view) = self.backgrounds.borrow_mut().remove(&id) {
                self.host.remove(&view);
            }
        }
        for ext in active {
            if has_background(&ext) && !self.backgrounds.borrow().contains_key(&ext.id.0) {
                self.spawn_background(&ext);
            }
        }
    }

    pub fn attach_content_view(self: &Rc<Self>, view: &WebView, tab_id: u64) {
        self.view_tabs.borrow_mut().insert(ptr_key(view), tab_id);
        self.connect_messages(view);
    }

    pub fn reset_injection(&self, view: &WebView) {
        self.injected.borrow_mut().remove(&ptr_key(view));
    }

    pub fn inject_content(self: &Rc<Self>, view: &WebView, url: &str, run_at: &str) {
        if !url.starts_with("http://")
            && !url.starts_with("https://")
            && !url.starts_with("file://")
        {
            return;
        }
        let view_key = ptr_key(view);
        for ext in self.enabled_extensions() {
            let world = world_name(&ext.id.0);
            self.ensure_world(view, &world);
            for (index, spec) in ext.content_scripts.iter().enumerate() {
                if normalized_run_at(&spec.run_at) != run_at || !content_script_matches(spec, url) {
                    continue;
                }
                let stamp = format!("{}:{index}:{run_at}", ext.id.0);
                let mut injected = self.injected.borrow_mut();
                let seen = injected.entry(view_key.clone()).or_default();
                if !seen.insert(stamp) {
                    continue;
                }
                drop(injected);
                let source = content_script_source(&ext, spec);
                let world_owned = world.clone();
                let world_log = world_owned.clone();
                view.evaluate_javascript(
                    &source,
                    Some(world_owned.as_str()),
                    None,
                    None::<&gio::Cancellable>,
                    move |result| {
                        if let Err(err) = result {
                            warn!("Script de conteúdo falhou em {world_log}: {err}");
                        }
                    },
                );
            }
        }
    }

    pub fn open_popup(self: &Rc<Self>, parent: &impl IsA<gtk4::Widget>, ext_id: &str) {
        let Some(info) = self.extension(ext_id) else {
            return;
        };
        let Some(popup) = info.popup_path.clone() else {
            return;
        };
        let popover = gtk4::Popover::new();
        popover.set_parent(parent);
        popover.set_autohide(true);
        let view = WebView::builder()
            .width_request(380)
            .height_request(520)
            .build();
        if let Some(tab_id) = self.active_tab().map(|tab| tab.id) {
            self.view_tabs.borrow_mut().insert(ptr_key(&view), tab_id);
        }
        self.prepare_extension_view(&view, &info);
        view.load_uri(&extension_page_url(&info.id.0, &popup));
        popover.set_child(Some(&view));
        popover.popup();
        popover.connect_closed(|popover| {
            popover.unparent();
        });
    }

    pub fn fire_action_clicked(&self, ext_id: &str, tab: &ExtensionTab) {
        let Some(view) = self.backgrounds.borrow().get(ext_id).cloned() else {
            return;
        };
        let tab_js = tab_value(tab).to_string();
        let script = format!(
            "if (globalThis.__brunyaActionClick) globalThis.__brunyaActionClick({tab_js});"
        );
        view.evaluate_javascript(&script, None, None, None::<&gio::Cancellable>, |_| {});
    }

    fn spawn_background(self: &Rc<Self>, info: &ExtensionInfo) {
        let view = WebView::new();
        view.set_size_request(1, 1);
        self.host.append(&view);
        self.prepare_extension_view(&view, info);
        let url = if let Some(page) = background_page_relative(info) {
            extension_page_url(&info.id.0, &page)
        } else {
            extension_page_url(&info.id.0, "_brunya/background.html")
        };
        let just_installed = self.pending_install.borrow_mut().remove(&info.id.0);
        let fired = Rc::new(RefCell::new(false));
        view.connect_load_changed(move |view, event| {
            if event != LoadEvent::Finished || *fired.borrow() {
                return;
            }
            *fired.borrow_mut() = true;
            let call = if just_installed {
                "__brunyaFireInstalled"
            } else {
                "__brunyaFireStartup"
            };
            let script = format!("if (globalThis.{call}) globalThis.{call}();");
            view.evaluate_javascript(&script, None, None, None::<&gio::Cancellable>, |_| {});
        });
        view.load_uri(&url);
        self.backgrounds
            .borrow_mut()
            .insert(info.id.0.clone(), view);
    }

    fn prepare_extension_view(self: &Rc<Self>, view: &WebView, info: &ExtensionInfo) {
        if let Some(manager) = view.user_content_manager() {
            let script = UserScript::new(
                &polyfill_source(info, "page"),
                UserContentInjectedFrames::AllFrames,
                UserScriptInjectionTime::Start,
                &[],
                &[],
            );
            manager.add_script(&script);
            manager.register_script_message_handler("brunyaExt", None);
        }
        self.connect_messages(view);
    }

    fn connect_messages(self: &Rc<Self>, view: &WebView) {
        let Some(manager) = view.user_content_manager() else {
            return;
        };
        let bridge = self.clone();
        let source = view.clone();
        manager.connect_script_message_received(Some("brunyaExt"), move |_manager, value| {
            bridge.on_message(&source, value);
        });
    }

    fn ensure_world(&self, view: &WebView, world: &str) {
        if let Some(manager) = view.user_content_manager() {
            manager.register_script_message_handler("brunyaExt", Some(world));
        }
    }

    fn on_message(&self, view: &WebView, value: &webkit6::javascriptcore::Value) {
        let raw = message_text(value);
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(&raw) else {
            warn!("Mensagem de extensão ilegível");
            return;
        };
        let op = msg.get("op").and_then(|v| v.as_str()).unwrap_or("");
        if op == "reply" {
            let reply_to = msg.get("replyTo").and_then(|v| v.as_str()).unwrap_or("");
            let payload = msg
                .get("payload")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            self.finish_reply(reply_to, payload, None);
            return;
        }

        let ext_id = msg
            .get("extId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let js_id = msg.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let context = msg
            .get("context")
            .and_then(|v| v.as_str())
            .unwrap_or("page");
        let world = (context == "content").then(|| world_name(&ext_id));
        let extension_id = ExtensionId::new(ext_id.clone());

        if op == "runtime.sendMessage" {
            self.route_message(view, &ext_id, js_id, &msg, world);
            return;
        }

        let result = match op {
            "storage.get" => Ok(self.storage_get(&extension_id, &msg)),
            "storage.set" => self
                .storage_set(&extension_id, &msg)
                .map(|_| serde_json::Value::Null),
            "storage.remove" => self
                .storage_remove(&extension_id, &msg)
                .map(|_| serde_json::Value::Null),
            "storage.clear" => self
                .storage_clear(&extension_id, &msg)
                .map(|_| serde_json::Value::Null),
            "tabs.query" => Ok(self.tabs_query(msg.get("query"))),
            "tabs.current" => Ok(self
                .tab_for_view(view)
                .map(|tab| tab_value(&tab))
                .unwrap_or(serde_json::Value::Null)),
            "tabs.create" => self.tabs_create(msg.get("props")),
            "tabs.update" => self.tabs_update(&msg),
            other => Err(format!("Operação de extensão não suportada: {other}")),
        };
        match result {
            Ok(payload) => self.answer(view, world.as_deref(), js_id, payload, None),
            Err(err) => self.answer(
                view,
                world.as_deref(),
                js_id,
                serde_json::Value::Null,
                Some(err),
            ),
        }
    }

    fn route_message(
        &self,
        view: &WebView,
        ext_id: &str,
        js_id: i64,
        msg: &serde_json::Value,
        world: Option<String>,
    ) {
        let token = msg
            .get("pageToken")
            .and_then(|v| v.as_str())
            .unwrap_or("page");
        let reply_to = format!("{token}:{js_id}");
        let background = self.backgrounds.borrow().get(ext_id).cloned();
        let from_background = background
            .as_ref()
            .map(|bg| ptr_key(bg) == ptr_key(view))
            .unwrap_or(false);
        if from_background || background.is_none() {
            self.answer(view, world.as_deref(), js_id, serde_json::Value::Null, None);
            return;
        }
        self.replies.borrow_mut().insert(
            reply_to.clone(),
            ReplyTarget {
                view: view.clone(),
                world,
                js_id,
            },
        );
        let message = msg
            .get("message")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let sender = serde_json::json!({
            "id": ext_id,
            "url": view.uri().map(|u| u.to_string()).unwrap_or_default(),
            "tab": self.tab_for_view(view).map(|tab| tab_value(&tab)),
        });
        let reply_js = serde_json::to_string(&reply_to).unwrap_or_else(|_| "\"\"".to_string());
        let script = format!(
            "if (globalThis.__brunyaDeliver) globalThis.__brunyaDeliver({message}, {sender}, {reply_js});"
        );
        if let Some(background) = background {
            background.evaluate_javascript(&script, None, None, None::<&gio::Cancellable>, |_| {});
        }
    }

    fn finish_reply(&self, reply_to: &str, payload: serde_json::Value, err: Option<String>) {
        let Some(target) = self.replies.borrow_mut().remove(reply_to) else {
            return;
        };
        self.answer(
            &target.view,
            target.world.as_deref(),
            target.js_id,
            payload,
            err,
        );
    }

    fn answer(
        &self,
        view: &WebView,
        world: Option<&str>,
        js_id: i64,
        payload: serde_json::Value,
        err: Option<String>,
    ) {
        let payload = serde_json::to_string(&payload).unwrap_or_else(|_| "null".to_string());
        let err = match err {
            Some(err) => serde_json::to_string(&err).unwrap_or_else(|_| "\"erro\"".to_string()),
            None => "null".to_string(),
        };
        let script = format!(
            "if (globalThis.__brunyaResolve) globalThis.__brunyaResolve({js_id}, {payload}, {err});"
        );
        view.evaluate_javascript(&script, world, None, None::<&gio::Cancellable>, |_| {});
    }

    fn storage_get(&self, ext_id: &ExtensionId, msg: &serde_json::Value) -> serde_json::Value {
        let area = storage_area(msg);
        let all = self.system.storage.get_all(ext_id, area);
        match msg.get("keys") {
            None | Some(serde_json::Value::Null) => map_to_json(all),
            Some(serde_json::Value::String(key)) => {
                let mut out = serde_json::Map::new();
                if let Some(value) = all.get(key) {
                    out.insert(key.clone(), value.clone());
                }
                serde_json::Value::Object(out)
            }
            Some(serde_json::Value::Array(keys)) => {
                let mut out = serde_json::Map::new();
                for key in keys {
                    if let Some(key) = key.as_str() {
                        if let Some(value) = all.get(key) {
                            out.insert(key.to_string(), value.clone());
                        }
                    }
                }
                serde_json::Value::Object(out)
            }
            Some(serde_json::Value::Object(defaults)) => {
                let mut out = serde_json::Map::new();
                for (key, default) in defaults {
                    out.insert(
                        key.clone(),
                        all.get(key).cloned().unwrap_or_else(|| default.clone()),
                    );
                }
                serde_json::Value::Object(out)
            }
            _ => serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    fn storage_set(&self, ext_id: &ExtensionId, msg: &serde_json::Value) -> Result<(), String> {
        let area = storage_area(msg);
        let Some(items) = msg.get("items").and_then(|v| v.as_object()) else {
            return Ok(());
        };
        for (key, value) in items {
            self.system
                .storage
                .set(ext_id, area, key.clone(), value.clone())
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }

    fn storage_remove(&self, ext_id: &ExtensionId, msg: &serde_json::Value) -> Result<(), String> {
        let area = storage_area(msg);
        let mut keys = Vec::new();
        match msg.get("keys") {
            Some(serde_json::Value::String(key)) => keys.push(key.clone()),
            Some(serde_json::Value::Array(list)) => {
                keys.extend(list.iter().filter_map(|v| v.as_str().map(str::to_string)));
            }
            _ => {}
        }
        for key in keys {
            self.system
                .storage
                .remove(ext_id, area, &key)
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }

    fn storage_clear(&self, ext_id: &ExtensionId, msg: &serde_json::Value) -> Result<(), String> {
        self.system
            .storage
            .clear(ext_id, storage_area(msg))
            .map_err(|err| err.to_string())
    }

    fn tabs_query(&self, query: Option<&serde_json::Value>) -> serde_json::Value {
        let tabs = self.all_tabs();
        let active_only = query
            .and_then(|q| q.get("active"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let url_pattern = query.and_then(|q| q.get("url")).and_then(|v| v.as_str());
        let matched: Vec<serde_json::Value> = tabs
            .into_iter()
            .filter(|tab| !active_only || tab.active)
            .filter(|tab| {
                url_pattern
                    .map(|pattern| url_matches(pattern, &tab.url))
                    .unwrap_or(true)
            })
            .map(|tab| tab_value(&tab))
            .collect();
        serde_json::Value::Array(matched)
    }

    fn tabs_create(&self, props: Option<&serde_json::Value>) -> Result<serde_json::Value, String> {
        let url = props
            .and_then(|p| p.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("brunya://newtab")
            .to_string();
        if let Some(open) = self.open_url.borrow().clone() {
            open(url.clone());
        }
        Ok(serde_json::json!({ "id": 0, "url": url, "active": true, "windowId": 1 }))
    }

    fn tabs_update(&self, msg: &serde_json::Value) -> Result<serde_json::Value, String> {
        let url = msg
            .pointer("/props/url")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let tab_id = msg.get("tabId").and_then(|v| v.as_u64());
        if let Some(url) = url {
            if let Some(navigate) = self.navigate.borrow().clone() {
                let id = tab_id.unwrap_or_else(|| self.active_tab().map(|tab| tab.id).unwrap_or(0));
                navigate(id, url.clone());
            }
            return Ok(serde_json::json!({ "id": tab_id.unwrap_or(0), "url": url }));
        }
        Ok(serde_json::Value::Null)
    }

    fn all_tabs(&self) -> Vec<ExtensionTab> {
        self.tabs
            .borrow()
            .as_ref()
            .map(|tabs| tabs())
            .unwrap_or_default()
    }

    fn active_tab(&self) -> Option<ExtensionTab> {
        self.all_tabs().into_iter().find(|tab| tab.active)
    }

    fn tab_for_view(&self, view: &WebView) -> Option<ExtensionTab> {
        let id = *self.view_tabs.borrow().get(&ptr_key(view))?;
        self.all_tabs().into_iter().find(|tab| tab.id == id)
    }

    fn extension(&self, ext_id: &str) -> Option<ExtensionInfo> {
        self.system
            .manager
            .get_extension(&ExtensionId::new(ext_id.to_string()))
    }

    fn enabled_extensions(&self) -> Vec<ExtensionInfo> {
        self.system.manager.list_active_extensions()
    }
}

pub fn extension_image(info: &ExtensionInfo, px: i32) -> gtk4::Image {
    if let Some(path) = best_icon_file(info) {
        let image = gtk4::Image::from_file(path);
        image.set_pixel_size(px);
        return image;
    }
    let image = if let Some(texture) = toolbar_icon(false) {
        gtk4::Image::from_paintable(Some(&texture))
    } else {
        gtk4::Image::from_icon_name("application-x-addon")
    };
    image.set_pixel_size(px);
    image
}

pub fn toolbar_icon(is_dark: bool) -> Option<gdk4::Texture> {
    let bytes = if is_dark {
        glib::Bytes::from_static(include_bytes!(
            "../../../../assets/icons/brunya-extensions-white.svg"
        ))
    } else {
        glib::Bytes::from_static(include_bytes!(
            "../../../../assets/icons/brunya-extensions.svg"
        ))
    };
    gdk4::Texture::from_bytes(&bytes).ok()
}

fn storage_area(msg: &serde_json::Value) -> StorageArea {
    msg.get("area")
        .and_then(|v| v.as_str())
        .and_then(StorageArea::from_str)
        .unwrap_or(StorageArea::Local)
}

fn map_to_json(map: HashMap<String, serde_json::Value>) -> serde_json::Value {
    serde_json::Value::Object(map.into_iter().collect())
}

fn tab_value(tab: &ExtensionTab) -> serde_json::Value {
    serde_json::json!({
        "id": tab.id,
        "windowId": 1,
        "index": tab.index,
        "active": tab.active,
        "url": tab.url,
        "title": tab.title,
        "status": "complete",
        "highlighted": tab.active,
    })
}

fn message_text(value: &webkit6::javascriptcore::Value) -> String {
    if let Some(json) = value.to_json(0) {
        let text = json.to_string();
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(inner) = parsed.as_str() {
                return inner.to_string();
            }
            return text;
        }
    }
    value.to_str().to_string()
}

fn ptr_key(view: &WebView) -> String {
    let ptr = <WebView as glib::translate::ToGlibPtr<'_, *mut webkit6::ffi::WebKitWebView>>::to_glib_none(
        view,
    )
    .0;
    format!("{ptr:p}")
}

fn register_extension_scheme(system: Arc<ExtensionSystem>) {
    static ONCE: Once = Once::new();
    ONCE.call_once(move || {
        let Some(context) = webkit6::WebContext::default() else {
            return;
        };
        if let Some(security) = context.security_manager() {
            security.register_uri_scheme_as_secure("brunya-extension");
            security.register_uri_scheme_as_cors_enabled("brunya-extension");
            security.register_uri_scheme_as_local("brunya-extension");
        }
        context.register_uri_scheme("brunya-extension", move |request| {
            serve_extension_uri(&system, request)
        });
    });
}

fn serve_extension_uri(system: &ExtensionSystem, request: &webkit6::URISchemeRequest) {
    let uri = request.uri().map(|u| u.to_string()).unwrap_or_default();
    let Ok(url) = url::Url::parse(&uri) else {
        finish_error(request, "URL de extensão inválida");
        return;
    };
    let Some(ext_id) = url.host_str() else {
        finish_error(request, "Extensão não indicada na URL");
        return;
    };
    let Some(info) = system
        .manager
        .get_extension(&ExtensionId::new(ext_id.to_string()))
    else {
        finish_error(request, "Extensão não instalada");
        return;
    };
    if !matches!(
        info.state,
        ExtensionState::Active | ExtensionState::DeveloperMode
    ) {
        finish_error(request, "Extensão desativada");
        return;
    }
    let path = url.path().trim_start_matches('/');
    if path.is_empty() || path.contains("..") {
        finish_error(request, "Caminho recusado");
        return;
    }
    if path == "_brunya/polyfill.js" {
        finish_bytes(
            request,
            polyfill_source(&info, "page").into_bytes(),
            "text/javascript",
        );
        return;
    }
    if path == "_brunya/background.html" {
        if let Some(html) = generated_background_html(&info) {
            finish_bytes(request, html.into_bytes(), "text/html");
            return;
        }
        finish_error(request, "Esta extensão não tem página de fundo");
        return;
    }
    match brunya_extensions::paths::read_extension_file(&info.directory, path) {
        Ok(bytes) => finish_bytes(request, bytes, mime_for(path)),
        Err(_) => finish_error(request, "Arquivo da extensão não encontrado"),
    }
}

fn finish_bytes(request: &webkit6::URISchemeRequest, bytes: Vec<u8>, mime: &str) {
    let len = bytes.len() as i64;
    let bytes = glib::Bytes::from_owned(bytes);
    let stream = gio::MemoryInputStream::from_bytes(&bytes);
    request.finish(&stream, len, Some(mime));
}

fn finish_error(request: &webkit6::URISchemeRequest, message: &str) {
    let mut error = glib::Error::new(gio::IOErrorEnum::NotFound, message);
    request.finish_error(&mut error);
}

fn mime_for(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") | Some("htm") => "text/html",
        Some("js") | Some("mjs") => "text/javascript",
        Some("css") => "text/css",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("wasm") => "application/wasm",
        Some("woff2") => "font/woff2",
        Some("txt") => "text/plain",
        _ => "application/octet-stream",
    }
}
