//! Injeção de scripts de conteúdo e o polyfill `chrome.*` / `browser.*`.

use crate::manifest::Manifest;
use crate::paths::{read_extension_text, resolve_extension_file};
use crate::types::{ContentScriptSpec, ExtensionInfo};
use std::path::{Path, PathBuf};

const POLYFILL: &str = r#"(function(){
  if (globalThis.__brunyaReady) return;
  globalThis.__brunyaReady = true;
  const EXT_ID = __EXT_ID__;
  const CONTEXT = __CONTEXT__;
  const MESSAGES = __MESSAGES__;
  const MANIFEST = __MANIFEST__;
  const pageToken = Math.random().toString(36).slice(2) + Date.now().toString(36);
  const pending = new Map();
  let seq = 1;
  const onMessageListeners = [];
  const onInstalledListeners = [];
  const onStartupListeners = [];
  const onActionListeners = [];
  const api = {};
  api.runtime = { lastError: undefined };

  function hostCall(op, data) {
    const id = seq++;
    return new Promise(function(resolve, reject) {
      pending.set(id, { resolve: resolve, reject: reject });
      const body = Object.assign({ id: id, pageToken: pageToken, extId: EXT_ID, context: CONTEXT, op: op }, data || {});
      try {
        window.webkit.messageHandlers.brunyaExt.postMessage(JSON.stringify(body));
      } catch (err) {
        pending.delete(id);
        reject(err);
      }
    });
  }

  globalThis.__brunyaResolve = function(id, payload, err) {
    const item = pending.get(id);
    if (!item) return;
    pending.delete(id);
    if (err) {
      api.runtime.lastError = { message: String(err) };
      item.resolve(undefined);
    } else {
      api.runtime.lastError = undefined;
      item.resolve(payload);
    }
  };

  function withCallback(promise, cb) {
    return promise.then(function(value) {
      if (typeof cb === "function") cb(value);
      return value;
    }, function(err) {
      api.runtime.lastError = { message: String(err && err.message || err) };
      if (typeof cb === "function") cb();
    });
  }

  function listenerApi(list) {
    return {
      addListener: function(fn) { if (typeof fn === "function") list.push(fn); },
      removeListener: function(fn) { const i = list.indexOf(fn); if (i >= 0) list.splice(i, 1); },
      hasListener: function(fn) { return list.indexOf(fn) >= 0; }
    };
  }

  function storageArea(area) {
    return {
      get: function(keys, cb) { return withCallback(hostCall("storage.get", { area: area, keys: keys === undefined ? null : keys }), cb); },
      set: function(items, cb) { return withCallback(hostCall("storage.set", { area: area, items: items || {} }), cb); },
      remove: function(keys, cb) { return withCallback(hostCall("storage.remove", { area: area, keys: keys }), cb); },
      clear: function(cb) { return withCallback(hostCall("storage.clear", { area: area }), cb); }
    };
  }

  function getMessage(key, substitutions) {
    const entry = MESSAGES[key];
    if (entry == null) return "";
    let msg = String(entry);
    const subs = Array.isArray(substitutions) ? substitutions : (substitutions == null ? [] : [substitutions]);
    subs.forEach(function(value, index) { msg = msg.split("$" + (index + 1)).join(String(value)); });
    return msg;
  }

  api.runtime.id = EXT_ID;
  api.runtime.getURL = function(path) {
    const clean = String(path || "").replace(/^\/+/, "");
    return "brunya-extension://" + EXT_ID + "/" + clean;
  };
  api.runtime.getManifest = function() { return MANIFEST; };
  api.runtime.sendMessage = function(message, options, cb) {
    let callback = cb;
    let msg = message;
    if (typeof options === "function") callback = options;
    if (typeof message === "string" && options && typeof options === "object") msg = options;
    return withCallback(hostCall("runtime.sendMessage", { message: msg }), callback);
  };
  api.runtime.onMessage = listenerApi(onMessageListeners);
  api.runtime.onInstalled = listenerApi(onInstalledListeners);
  api.runtime.onStartup = listenerApi(onStartupListeners);
  api.runtime.connect = function() {
    return { name: "", postMessage: function(){}, disconnect: function(){}, onMessage: listenerApi([]), onDisconnect: listenerApi([]) };
  };
  api.runtime.onConnect = listenerApi([]);

  const action = {
    setBadgeText: function(_d, cb) { if (cb) cb(); return Promise.resolve(); },
    setBadgeBackgroundColor: function(_d, cb) { if (cb) cb(); return Promise.resolve(); },
    setTitle: function(_d, cb) { if (cb) cb(); return Promise.resolve(); },
    setIcon: function(_d, cb) { if (cb) cb(); return Promise.resolve(); },
    onClicked: listenerApi(onActionListeners)
  };
  api.action = action;
  api.browserAction = action;
  api.pageAction = action;
  api.storage = { local: storageArea("local"), sync: storageArea("sync"), session: storageArea("session"), onChanged: listenerApi([]) };
  api.i18n = { getMessage: getMessage };
  api.tabs = {
    query: function(query, cb) { return withCallback(hostCall("tabs.query", { query: query || {} }), cb); },
    create: function(props, cb) { return withCallback(hostCall("tabs.create", { props: props || {} }), cb); },
    update: function(tabId, props, cb) {
      if (typeof tabId === "object") { cb = props; props = tabId; tabId = null; }
      return withCallback(hostCall("tabs.update", { tabId: tabId, props: props || {} }), cb);
    },
    sendMessage: function(_tabId, message, opts, cb) {
      if (typeof opts === "function") cb = opts;
      return api.runtime.sendMessage(message, cb);
    },
    getCurrent: function(cb) { return withCallback(hostCall("tabs.current", {}), cb); },
    onUpdated: listenerApi([]),
    onActivated: listenerApi([]),
    onRemoved: listenerApi([])
  };
  api.alarms = { create: function(){}, clear: function(){ return Promise.resolve(true); }, onAlarm: listenerApi([]) };
  api.notifications = { create: function(id, opt, cb) { if (typeof id === "object") cb = opt; if (cb) cb("brunya"); return Promise.resolve("brunya"); } };
  api.contextMenus = { create: function(){}, removeAll: function(cb){ if (cb) cb(); return Promise.resolve(); }, onClicked: listenerApi([]) };
  api.permissions = {
    contains: function(_p, cb) { if (cb) cb(true); return Promise.resolve(true); },
    request: function(_p, cb) { if (cb) cb(true); return Promise.resolve(true); },
    remove: function(_p, cb) { if (cb) cb(true); return Promise.resolve(true); }
  };
  api.scripting = { executeScript: function(){ return Promise.resolve([]); }, insertCSS: function(){ return Promise.resolve(); } };
  api.webNavigation = { onCompleted: listenerApi([]), onBeforeNavigate: listenerApi([]) };
  api.declarativeNetRequest = { updateDynamicRules: function(){ return Promise.resolve(); }, getDynamicRules: function(cb){ if (cb) cb([]); return Promise.resolve([]); } };

  globalThis.chrome = api;
  globalThis.browser = api;

  globalThis.__brunyaDeliver = function(message, sender, replyTo) {
    let replied = false;
    let asyncReply = false;
    function sendResponse(payload) {
      if (replied) return;
      replied = true;
      try {
        window.webkit.messageHandlers.brunyaExt.postMessage(JSON.stringify({
          op: "reply", replyTo: replyTo, payload: payload === undefined ? null : payload
        }));
      } catch (err) {}
    }
    onMessageListeners.forEach(function(fn) {
      try {
        if (fn(message, sender, sendResponse) === true) asyncReply = true;
      } catch (err) { console.error(err); }
    });
    if (!asyncReply && !replied) sendResponse(null);
  };
  globalThis.__brunyaFireInstalled = function() {
    onInstalledListeners.forEach(function(fn) { try { fn({ reason: "install" }); } catch (err) {} });
  };
  globalThis.__brunyaFireStartup = function() {
    onStartupListeners.forEach(function(fn) { try { fn(); } catch (err) {} });
  };
  globalThis.__brunyaActionClick = function(tab) {
    onActionListeners.forEach(function(fn) { try { fn(tab); } catch (err) {} });
  };
})();
"#;

/// Nome do mundo isolado de uma extensão dentro do WebKit.
pub fn world_name(ext_id: &str) -> String {
    format!("brunya-{ext_id}")
}

pub fn extension_page_url(ext_id: &str, relative: &str) -> String {
    let relative = relative.trim().trim_start_matches('/');
    format!("brunya-extension://{ext_id}/{relative}")
}

pub fn normalized_run_at(value: &str) -> &'static str {
    match value {
        "document_start" => "document_start",
        "document_end" => "document_end",
        _ => "document_idle",
    }
}

pub fn url_matches_any(patterns: &[String], url: &str) -> bool {
    patterns.iter().any(|pattern| url_matches(pattern, url))
}

pub fn content_script_matches(spec: &ContentScriptSpec, url: &str) -> bool {
    if spec.matches.is_empty() || !url_matches_any(&spec.matches, url) {
        return false;
    }
    !url_matches_any(&spec.exclude_matches, url)
}

/// Padrão de correspondência do Chrome (`*://*.exemplo.com/*`, `<all_urls>`).
pub fn url_matches(pattern: &str, raw_url: &str) -> bool {
    if pattern == "<all_urls>" {
        return raw_url.starts_with("http://")
            || raw_url.starts_with("https://")
            || raw_url.starts_with("file://")
            || raw_url.starts_with("ftp://");
    }
    let Some((pat_scheme, rest)) = pattern.split_once("://") else {
        return false;
    };
    let (pat_host, pat_path) = match rest.split_once('/') {
        Some((host, path)) => (host, format!("/{path}")),
        None => (rest, "/*".to_string()),
    };
    let Ok(url) = url::Url::parse(raw_url) else {
        return false;
    };
    let scheme = url.scheme();
    let scheme_ok = if pat_scheme == "*" {
        scheme == "http" || scheme == "https"
    } else {
        pat_scheme.eq_ignore_ascii_case(scheme)
    };
    if !scheme_ok {
        return false;
    }
    let host = url.host_str().unwrap_or("");
    if !host_matches(pat_host, host) {
        return false;
    }
    let path = if url.path().is_empty() {
        "/"
    } else {
        url.path()
    };
    glob_match(&pat_path, path)
}

fn host_matches(pattern: &str, host: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        return host.eq_ignore_ascii_case(suffix)
            || host
                .to_ascii_lowercase()
                .ends_with(&format!(".{suffix}").to_ascii_lowercase());
    }
    pattern.eq_ignore_ascii_case(host)
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == text;
    }
    if !text.starts_with(parts[0]) {
        return false;
    }
    let mut rest = &text[parts[0].len()..];
    for (index, part) in parts.iter().enumerate().skip(1) {
        if index + 1 == parts.len() {
            return rest.ends_with(part);
        }
        if part.is_empty() {
            continue;
        }
        if let Some(pos) = rest.find(part) {
            rest = &rest[pos + part.len()..];
        } else {
            return false;
        }
    }
    true
}

pub fn polyfill_source(info: &ExtensionInfo, context: &str) -> String {
    POLYFILL
        .replace("__EXT_ID__", &json_string(&info.id.0))
        .replace("__CONTEXT__", &json_string(context))
        .replace("__MESSAGES__", &messages_json(&info.directory))
        .replace("__MANIFEST__", &manifest_stub(info))
}

pub fn content_script_source(info: &ExtensionInfo, spec: &ContentScriptSpec) -> String {
    let mut parts = vec![polyfill_source(info, "content")];
    for css in &spec.css {
        if let Ok(text) = read_extension_text(&info.directory, css) {
            let encoded = serde_json::to_string(&text).unwrap_or_else(|_| "\"\"".to_string());
            let id = json_string(&info.id.0);
            parts.push(format!(
                "(function(){{var css={encoded};var el=document.createElement('style');el.setAttribute('data-brunya-ext',{id});el.textContent=css;(document.head||document.documentElement).appendChild(el);}})();"
            ));
        }
    }
    for js in &spec.js {
        if let Ok(text) = read_extension_text(&info.directory, js) {
            parts.push(text);
        }
    }
    parts.join("\n")
}

/// HTML gerado para service worker ou scripts de fundo. `None` quando a extensão tem uma página própria.
pub fn generated_background_html(info: &ExtensionInfo) -> Option<String> {
    let (manifest, _) = Manifest::load_from_dir(&info.directory).ok()?;
    let bg = manifest.background.as_ref()?;
    if bg.page.is_some() {
        return None;
    }
    let mut scripts = Vec::new();
    if let Some(worker) = &bg.service_worker {
        scripts.push(worker.clone());
    }
    scripts.extend(bg.scripts.iter().cloned());
    if scripts.is_empty() {
        return None;
    }
    let mut html = format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><script src=\"{}\"></script>",
        extension_page_url(&info.id.0, "_brunya/polyfill.js")
    );
    for script in scripts {
        html.push_str(&format!(
            "<script src=\"{}\"></script>",
            extension_page_url(&info.id.0, &script)
        ));
    }
    html.push_str("</head><body></body></html>");
    Some(html)
}

pub fn background_page_relative(info: &ExtensionInfo) -> Option<String> {
    let (manifest, _) = Manifest::load_from_dir(&info.directory).ok()?;
    manifest.background.as_ref().and_then(|bg| bg.page.clone())
}

pub fn has_background(info: &ExtensionInfo) -> bool {
    background_page_relative(info).is_some() || generated_background_html(info).is_some()
}

/// Ícone da ação ou, na falta dele, o maior ícone do manifesto que exista no disco.
pub fn best_icon_file(info: &ExtensionInfo) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok((manifest, _)) = Manifest::load_from_dir(&info.directory) {
        for action in [
            manifest.action,
            manifest.browser_action,
            manifest.page_action,
        ]
        .into_iter()
        .flatten()
        {
            if let Some(icons) = action.default_icon {
                push_icons(
                    &mut candidates,
                    &info.directory,
                    icons
                        .into_iter()
                        .map(|(size, path)| (size.parse::<i32>().unwrap_or(32), path)),
                );
            }
        }
    }
    push_icons(
        &mut candidates,
        &info.directory,
        info.icons
            .iter()
            .map(|icon| (icon.size as i32, icon.path.clone())),
    );
    candidates.sort_by_key(|(size, _)| (*size - 32).abs());
    candidates.into_iter().next().map(|(_, path)| path)
}

fn push_icons(
    out: &mut Vec<(i32, PathBuf)>,
    base: &Path,
    icons: impl Iterator<Item = (i32, String)>,
) {
    for (size, relative) in icons {
        if let Ok(path) = resolve_extension_file(base, &relative) {
            if path.is_file() {
                out.push((size, path));
            }
        }
    }
}

fn messages_json(dir: &Path) -> String {
    let locale = Manifest::load_from_dir(dir)
        .ok()
        .and_then(|(manifest, _)| manifest.default_locale);
    let mut locales = Vec::new();
    if let Some(locale) = locale {
        locales.push(locale);
    }
    locales.extend([
        "pt_BR".to_string(),
        "pt-BR".to_string(),
        "pt".to_string(),
        "en".to_string(),
        "en_US".to_string(),
    ]);
    for locale in locales {
        let path = dir.join("_locales").join(locale).join("messages.json");
        let Ok(raw) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
            continue;
        };
        let Some(obj) = json.as_object() else {
            continue;
        };
        let mut flat = serde_json::Map::new();
        for (key, value) in obj {
            if let Some(message) = value.get("message").and_then(|m| m.as_str()) {
                flat.insert(key.clone(), serde_json::Value::String(message.to_string()));
            }
        }
        if !flat.is_empty() {
            return serde_json::Value::Object(flat).to_string();
        }
    }
    "{}".to_string()
}

fn manifest_stub(info: &ExtensionInfo) -> String {
    serde_json::json!({
        "manifest_version": info.manifest_version,
        "name": info.name,
        "version": info.version,
        "description": info.description,
    })
    .to_string()
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_patterns() {
        assert!(url_matches("*://*/*", "https://example.com/a"));
        assert!(url_matches(
            "https://*.google.com/*",
            "https://www.google.com/search"
        ));
        assert!(url_matches("https://*.google.com/*", "https://google.com/"));
        assert!(!url_matches(
            "https://*.google.com/*",
            "http://www.google.com/"
        ));
        assert!(url_matches("<all_urls>", "https://example.com/"));
        assert!(!url_matches("<all_urls>", "brunya://extensions"));
        assert!(url_matches(
            "*://example.com/foo*",
            "https://example.com/foobar"
        ));
    }
}
