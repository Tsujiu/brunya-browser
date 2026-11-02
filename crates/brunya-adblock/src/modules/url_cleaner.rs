//! Módulo 08 — URL Cleaner (Limpeza de Parâmetros de Rastreamento)

use url::Url;

pub struct UrlCleaner;

impl UrlCleaner {
    pub const TRACKING_PARAMS: &'static [&'static str] = &[
        "utm_source",
        "utm_medium",
        "utm_campaign",
        "utm_term",
        "utm_content",
        "utm_id",
        "fbclid",
        "gclid",
        "gbraid",
        "wbraid",
        "msclkid",
        "mc_eid",
        "dclid",
        "igshid",
        "twclid",
        "yclid",
        "s_kwcid",
        "_hsenc",
        "_hsmi",
        "mkt_tok",
        "sc_src",
        "sc_lid",
        "sc_uid",
    ];

    /// Limpa parâmetros de rastreamento conhecidos de uma URL preservando parâmetros legítimos.
    /// Retorna a URL limpa e a quantidade de parâmetros removidos.
    pub fn clean_url(raw_url: &str) -> (String, u32) {
        let mut parsed = match Url::parse(raw_url) {
            Ok(u) => u,
            Err(_) => return (raw_url.to_string(), 0),
        };

        // Somente URLs HTTP/HTTPS possuem parâmetros de rastreamento
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return (raw_url.to_string(), 0);
        }

        let mut cleaned_pairs = Vec::new();
        let mut stripped_count = 0;

        for (key, val) in parsed.query_pairs() {
            let key_lower = key.to_lowercase();
            // Parâmetros essenciais que NUNCA devem ser removidos
            if key_lower == "token"
                || key_lower == "code"
                || key_lower == "state"
                || key_lower == "id"
                || key_lower == "key"
                || key_lower == "auth"
                || key_lower == "session"
            {
                cleaned_pairs.push((key.into_owned(), val.into_owned()));
                continue;
            }

            if Self::TRACKING_PARAMS.contains(&key_lower.as_str()) {
                stripped_count += 1;
            } else {
                cleaned_pairs.push((key.into_owned(), val.into_owned()));
            }
        }

        if stripped_count > 0 {
            parsed.set_query(None);
            if !cleaned_pairs.is_empty() {
                let mut serializer = url::form_urlencoded::Serializer::new(String::new());
                for (k, v) in cleaned_pairs {
                    serializer.append_pair(&k, &v);
                }
                parsed.set_query(Some(&serializer.finish()));
            }
            (parsed.to_string(), stripped_count)
        } else {
            (raw_url.to_string(), 0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_cleaner_strips_tracking() {
        let url = "https://example.com/shop?utm_source=newsletter&utm_campaign=summer&id=123&fbclid=AQb_12345";
        let (cleaned, count) = UrlCleaner::clean_url(url);
        assert_eq!(count, 3);
        assert!(cleaned.contains("id=123"));
        assert!(!cleaned.contains("utm_source"));
        assert!(!cleaned.contains("fbclid"));
    }

    #[test]
    fn test_url_cleaner_preserves_auth() {
        let url = "https://oauth.example.com/callback?code=sec_123&state=xyz&token=abc";
        let (cleaned, count) = UrlCleaner::clean_url(url);
        assert_eq!(count, 0);
        assert_eq!(cleaned, url);
    }
}
