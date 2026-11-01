use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use url::Url;

/// Identificador único para cada aba no navegador.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TabId(pub u64);

static NEXT_TAB_ID: AtomicU64 = AtomicU64::new(1);

impl TabId {
    pub fn new() -> Self {
        Self(NEXT_TAB_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for TabId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TabId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tab-{}", self.0)
    }
}

/// Nível de segurança da conexão TLS/HTTP da página atual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SecurityLevel {
    #[default]
    Unknown,
    Secure,          // HTTPS válido
    SecureWithHints, // HTTPS com avisos menores (ex: formulário misto)
    Insecure,        // HTTP sem criptografia
    BrokenTls,       // Certificado inválido ou expirado
    Internal,        // bear:// páginas internas
}

impl SecurityLevel {
    pub fn icon_name(&self) -> &'static str {
        match self {
            SecurityLevel::Secure => "channel-secure-symbolic",
            SecurityLevel::SecureWithHints => "channel-insecure-symbolic",
            SecurityLevel::Insecure => "channel-insecure-symbolic",
            SecurityLevel::BrokenTls => "dialog-warning-symbolic",
            SecurityLevel::Internal => "security-high-symbolic",
            SecurityLevel::Unknown => "network-wireless-acquiring-symbolic",
        }
    }

    pub fn is_secure(&self) -> bool {
        matches!(self, SecurityLevel::Secure | SecurityLevel::Internal)
    }
}

/// Estado reativo da navegação em uma aba.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavigationState {
    pub url: String,
    pub title: String,
    pub favicon_url: Option<String>,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub is_loading: bool,
    pub progress: f64,
    pub security_level: SecurityLevel,
    pub status_code: Option<i32>,
    pub updated_at: DateTime<Utc>,
}

impl Default for NavigationState {
    fn default() -> Self {
        Self {
            url: "bear://newtab".to_string(),
            title: "Nova Aba".to_string(),
            favicon_url: None,
            can_go_back: false,
            can_go_forward: false,
            is_loading: false,
            progress: 0.0,
            security_level: SecurityLevel::Internal,
            status_code: None,
            updated_at: Utc::now(),
        }
    }
}

/// Helper para normalização e resolução de entrada na barra de endereço.
pub struct UrlHelper;

impl UrlHelper {
    /// Converte texto digitado pelo usuário (ex: "kernel.org" ou "notícias de tecnologia")
    /// em uma URL canônica válida ou URL de pesquisa sem rastreamento.
    pub fn parse_input(input: &str, search_engine_template: &str) -> String {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return "bear://newtab".to_string();
        }

        // Esquemas internos do BEAR
        if trimmed.starts_with("bear://") || trimmed.starts_with("about:") {
            return trimmed.to_string();
        }

        // Esquema explícito
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") || trimmed.starts_with("file://") {
            if let Ok(url) = Url::parse(trimmed) {
                return url.to_string();
            }
        }

        // Caso pareça um domínio (ex: "example.com", "localhost", "192.168.1.1:8080")
        if Self::looks_like_domain(trimmed) {
            let candidate = format!("https://{}", trimmed);
            if let Ok(url) = Url::parse(&candidate) {
                return url.to_string();
            }
        }

        // Caso contrário, é uma pesquisa no mecanismo padrão
        let encoded_query = url::form_urlencoded::byte_serialize(trimmed.as_bytes()).collect::<String>();
        if search_engine_template.contains("%s") {
            search_engine_template.replace("%s", &encoded_query)
        } else {
            format!("https://duckduckgo.com/?q={}", encoded_query)
        }
    }

    fn looks_like_domain(text: &str) -> bool {
        if text.contains(' ') {
            return false;
        }

        if text.starts_with("localhost") {
            return true;
        }

        if let Some((domain, _)) = text.split_once('/') {
            return Self::looks_like_domain(domain);
        }

        if let Some((domain, _)) = text.split_once(':') {
            return Self::looks_like_domain(domain);
        }

        if let Some(dot_idx) = text.rfind('.') {
            let tld = &text[dot_idx + 1..];
            return !tld.is_empty() && tld.chars().all(|c| c.is_ascii_alphanumeric());
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_id_uniqueness() {
        let t1 = TabId::new();
        let t2 = TabId::new();
        assert_ne!(t1, t2);
        assert!(t2.0 > t1.0);
    }

    #[test]
    fn test_url_helper_domain_vs_search() {
        let engine = "https://duckduckgo.com/?q=%s";

        assert_eq!(
            UrlHelper::parse_input("example.com", engine),
            "https://example.com/"
        );
        assert_eq!(
            UrlHelper::parse_input("https://mintlinux.org", engine),
            "https://mintlinux.org/"
        );
        assert_eq!(
            UrlHelper::parse_input("rust lang tutorial", engine),
            "https://duckduckgo.com/?q=rust+lang+tutorial"
        );
        assert_eq!(
            UrlHelper::parse_input("bear://settings", engine),
            "bear://settings"
        );
    }
}
