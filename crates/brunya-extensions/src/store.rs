//! Download de extensões nas lojas Chrome, Edge, Opera e Firefox.
//!
//! O pacote vem do endpoint oficial de atualização de cada loja (CRX ou XPI).
//! O Brunya não republica o arquivo.

use crate::errors::{ExtensionError, Result};
use crate::loader::{inspect_package, PackagePreview, sanitize_extension_id};
use std::io::Read;

const MAX_PACKAGE_BYTES: u64 = 64 * 1024 * 1024;
const CHROME_VERSION: &str = "143.0.7499.41";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.7499.41 Safari/537.36";

/// Loja reconhecida a partir do texto colado pelo usuário.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreRequest {
    Chrome { id: String },
    Edge { id: String },
    Opera { slug: String },
    Firefox { slug: String },
}

impl StoreRequest {
    pub fn store_name(&self) -> &'static str {
        match self {
            StoreRequest::Chrome { .. } => "Chrome Web Store",
            StoreRequest::Edge { .. } => "Edge Add-ons",
            StoreRequest::Opera { .. } => "Opera Addons",
            StoreRequest::Firefox { .. } => "Firefox Add-ons",
        }
    }

    pub fn explicit_id(&self) -> Option<&str> {
        match self {
            StoreRequest::Chrome { id } | StoreRequest::Edge { id } => Some(id),
            StoreRequest::Opera { .. } | StoreRequest::Firefox { .. } => None,
        }
    }
}

/// Pacote baixado e já inspecionado, pronto para o diálogo de permissões.
#[derive(Debug, Clone)]
pub struct DownloadedExtension {
    pub bytes: Vec<u8>,
    pub preview: PackagePreview,
    pub store_name: String,
}

/// Interpreta uma URL de loja ou um id de extensão.
pub fn parse_store_input(input: &str) -> Result<StoreRequest> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ExtensionError::InvalidUrl(
            "Informe o link da loja ou o id da extensão".to_string(),
        ));
    }

    let lower = input.to_ascii_lowercase();
    if lower.contains("chromewebstore.google.com") || lower.contains("chrome.google.com/webstore") {
        let id = find_store_id(input).ok_or_else(|| {
            ExtensionError::InvalidUrl("Não encontrei o id da extensão nesse link do Chrome".to_string())
        })?;
        return Ok(StoreRequest::Chrome { id });
    }
    if lower.contains("microsoftedge.microsoft.com") || lower.contains("edge.microsoft.com") {
        let id = find_store_id(input).ok_or_else(|| {
            ExtensionError::InvalidUrl("Não encontrei o id da extensão nesse link do Edge".to_string())
        })?;
        return Ok(StoreRequest::Edge { id });
    }
    if lower.contains("addons.opera.com") {
        let slug = last_path_segment(input).ok_or_else(|| {
            ExtensionError::InvalidUrl("Não encontrei o nome da extensão nesse link do Opera".to_string())
        })?;
        return Ok(StoreRequest::Opera { slug });
    }
    if lower.contains("addons.mozilla.org") {
        let slug = segment_after(input, "addon").ok_or_else(|| {
            ExtensionError::InvalidUrl("Não encontrei o nome da extensão nesse link do Firefox".to_string())
        })?;
        return Ok(StoreRequest::Firefox { slug });
    }
    if let Some(id) = as_store_id(input) {
        return Ok(StoreRequest::Chrome { id });
    }
    Err(ExtensionError::InvalidUrl(
        "Use um link da Chrome Web Store, Edge Add-ons, Opera Addons, Firefox Add-ons, ou o id de 32 letras".to_string(),
    ))
}

/// Baixa o pacote e lê o manifesto para a confirmação.
pub fn download_store_package(request: &StoreRequest) -> Result<DownloadedExtension> {
    let bytes = match request {
        StoreRequest::Chrome { id } => http_get(&chrome_crx_url(id))?,
        StoreRequest::Edge { id } => http_get(&edge_crx_url(id))?,
        StoreRequest::Opera { slug } => download_opera(slug)?,
        StoreRequest::Firefox { slug } => download_firefox(slug)?,
    };
    if !(bytes.starts_with(b"PK") || bytes.starts_with(b"Cr24")) {
        return Err(ExtensionError::Download(
            "A loja não devolveu um pacote de extensão".to_string(),
        ));
    }
    let explicit = request.explicit_id();
    let mut preview = inspect_package(&bytes, explicit)?;
    if let Some(id) = explicit {
        preview.id = sanitize_extension_id(id)?;
    }
    Ok(DownloadedExtension {
        bytes,
        preview,
        store_name: request.store_name().to_string(),
    })
}

fn chrome_crx_url(id: &str) -> String {
    format!(
        "https://clients2.google.com/service/update2/crx?response=redirect&os=linux&arch=x64&os_arch=x86_64&nacl_arch=x86-64&prod=chromiumcrx&prodchannel=stable&prodversion={CHROME_VERSION}&acceptformat=crx2,crx3&x=id%3D{id}%26installsource%3Dondemand%26uc"
    )
}

fn edge_crx_url(id: &str) -> String {
    format!(
        "https://edge.microsoft.com/extensionwebstorebase/v1/crx?response=redirect&prod=chromiumcrx&prodchannel=stable&prodversion={CHROME_VERSION}&acceptformat=crx3&x=id%3D{id}%26installsource%3Dondemand%26uc"
    )
}

fn download_opera(slug: &str) -> Result<Vec<u8>> {
    let url = format!("https://addons.opera.com/extensions/download/{slug}/");
    let bytes = http_get(&url)?;
    if bytes.starts_with(b"PK") || bytes.starts_with(b"Cr24") {
        return Ok(bytes);
    }
    let html = String::from_utf8_lossy(&bytes);
    if let Some(found) = regex::Regex::new(r#"https?://[^"'\\\s>]+\.(?:crx|nex|zip|xpi)"#)
        .ok()
        .and_then(|re| re.find(&html).map(|m| m.as_str().to_string()))
    {
        return http_get(&found);
    }
    Err(ExtensionError::Download(
        "A loja do Opera não entregou o pacote dessa extensão".to_string(),
    ))
}

fn download_firefox(slug: &str) -> Result<Vec<u8>> {
    let api = format!("https://addons.mozilla.org/api/v5/addons/addon/{slug}/");
    let body = http_get(&api)?;
    let json: serde_json::Value = serde_json::from_slice(&body).map_err(|err| {
        ExtensionError::Download(format!("Resposta inválida do Firefox Add-ons: {err}"))
    })?;
    let file_url = json
        .pointer("/current_version/file/url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ExtensionError::Download(
                "O Firefox Add-ons não informou o arquivo dessa extensão".to_string(),
            )
        })?;
    http_get(file_url)
}

fn http_get(url: &str) -> Result<Vec<u8>> {
    let mut response = ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "*/*")
        .call()
        .map_err(|err| ExtensionError::Download(err.to_string()))?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(ExtensionError::Download(format!(
            "A loja respondeu HTTP {status}"
        )));
    }
    let mut reader = response.body_mut().as_reader().take(MAX_PACKAGE_BYTES + 1);
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    if buf.len() as u64 > MAX_PACKAGE_BYTES {
        return Err(ExtensionError::Download(
            "O pacote passa de 64 MB".to_string(),
        ));
    }
    if buf.is_empty() {
        return Err(ExtensionError::Download(
            "A loja devolveu uma resposta vazia".to_string(),
        ));
    }
    Ok(buf)
}

fn as_store_id(input: &str) -> Option<String> {
    let id = input.trim().to_ascii_lowercase();
    if id.len() == 32 && id.chars().all(|c| ('a'..='p').contains(&c)) {
        Some(id)
    } else {
        None
    }
}

fn find_store_id(input: &str) -> Option<String> {
    path_segments(input).into_iter().rev().find_map(|seg| as_store_id(&seg))
}

fn last_path_segment(input: &str) -> Option<String> {
    path_segments(input).into_iter().rev().find(|seg| !seg.is_empty())
}

fn segment_after(input: &str, marker: &str) -> Option<String> {
    let segments = path_segments(input);
    let pos = segments.iter().position(|seg| seg.eq_ignore_ascii_case(marker))?;
    let slug = segments.get(pos + 1)?;
    if slug.is_empty() { None } else { Some(slug.clone()) }
}

fn path_segments(input: &str) -> Vec<String> {
    let no_query = input.split(['?', '#']).next().unwrap_or(input);
    no_query
        .split('/')
        .filter(|seg| !seg.is_empty() && !seg.contains("://") && !seg.contains('.'))
        .map(|seg| seg.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chrome_edge_opera_firefox_and_raw_id() {
        let chrome = parse_store_input(
            "https://chromewebstore.google.com/detail/dark-reader/eimadpbcbfnmbkopoojfekhnkhdbieeh",
        ).unwrap();
        assert_eq!(
            chrome,
            StoreRequest::Chrome { id: "eimadpbcbfnmbkopoojfekhnkhdbieeh".into() }
        );

        let edge = parse_store_input(
            "https://microsoftedge.microsoft.com/addons/detail/dark-reader/ifoakfbpdcdoeenechcleahebpkhcibi",
        ).unwrap();
        assert!(matches!(edge, StoreRequest::Edge { .. }));

        let opera = parse_store_input(
            "https://addons.opera.com/en/extensions/details/dark-reader/",
        ).unwrap();
        assert_eq!(opera, StoreRequest::Opera { slug: "dark-reader".into() });

        let firefox = parse_store_input(
            "https://addons.mozilla.org/pt-BR/firefox/addon/darkreader/",
        ).unwrap();
        assert_eq!(firefox, StoreRequest::Firefox { slug: "darkreader".into() });

        let raw = parse_store_input("eimadpbcbfnmbkopoojfekhnkhdbieeh").unwrap();
        assert_eq!(
            raw,
            StoreRequest::Chrome { id: "eimadpbcbfnmbkopoojfekhnkhdbieeh".into() }
        );
    }

    #[test]
    fn store_catalog_without_id_is_not_an_install() {
        assert!(parse_store_input("https://chromewebstore.google.com/category/extensions").is_err());
        assert!(parse_store_input("https://chromewebstore.google.com/").is_err());
    }

    #[test]
    #[ignore = "baixa um CRX real da Chrome Web Store"]
    fn downloads_dark_reader_from_chrome_store() {
        let request = parse_store_input(
            "https://chromewebstore.google.com/detail/dark-reader/eimadpbcbfnmbkopoojfekhnkhdbieeh",
        )
        .unwrap();
        let package = download_store_package(&request).expect("download");
        assert!(package.bytes.starts_with(b"Cr24") || package.bytes.starts_with(b"PK"));
        assert!(package.preview.name.to_lowercase().contains("dark"));
        assert_eq!(package.preview.id, "eimadpbcbfnmbkopoojfekhnkhdbieeh");
    }
}
