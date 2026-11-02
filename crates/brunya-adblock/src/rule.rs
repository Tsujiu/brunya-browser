use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Script,
    Image,
    Stylesheet,
    XmlHttpRequest,
    Subdocument,
    Media,
    Font,
    Other,
}

impl ResourceType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "script" => Some(ResourceType::Script),
            "image" => Some(ResourceType::Image),
            "stylesheet" | "css" => Some(ResourceType::Stylesheet),
            "xmlhttprequest" | "xhr" | "fetch" => Some(ResourceType::XmlHttpRequest),
            "subdocument" | "iframe" => Some(ResourceType::Subdocument),
            "media" => Some(ResourceType::Media),
            "font" => Some(ResourceType::Font),
            "other" => Some(ResourceType::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleKind {
    Network,
    Whitelist,
    Cosmetic { selector: String },
}

#[derive(Debug, Clone)]
pub struct ParsedRule {
    pub raw: String,
    pub kind: RuleKind,
    pub pattern: String,
    pub domain_anchor: bool,   // Começa com ||
    pub start_anchor: bool,    // Começa com |
    pub end_anchor: bool,      // Termina com |
    pub third_party: Option<bool>,
    pub resource_types: Option<HashSet<ResourceType>>,
    pub domains: Option<HashSet<String>>,
    pub excluded_domains: Option<HashSet<String>>,
}

impl ParsedRule {
    pub fn parse(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('!') || trimmed.starts_with('#') {
            return None; // Comentários
        }

        // 1. Regra Cosmética (Element Hiding): domain.com##.ad-class ou ##.ad-banner
        if let Some((domains_part, selector_part)) = trimmed.split_once("##") {
            let mut domains = HashSet::new();
            for d in domains_part.split(',') {
                let d_trim = d.trim();
                if !d_trim.is_empty() {
                    domains.insert(d_trim.to_lowercase());
                }
            }
            return Some(ParsedRule {
                raw: trimmed.to_string(),
                kind: RuleKind::Cosmetic {
                    selector: selector_part.trim().to_string(),
                },
                pattern: String::new(),
                domain_anchor: false,
                start_anchor: false,
                end_anchor: false,
                third_party: None,
                resource_types: None,
                domains: if domains.is_empty() { None } else { Some(domains) },
                excluded_domains: None,
            });
        }

        // 2. Regra de Exceção / Whitelist: @@||example.com...
        let (is_whitelist, rest) = if let Some(stripped) = trimmed.strip_prefix("@@") {
            (true, stripped)
        } else {
            (false, trimmed)
        };

        // 3. Separar opções ($script,domain=example.com)
        let (rule_body, options_part) = match rest.split_once('$') {
            Some((b, o)) => (b, Some(o)),
            None => (rest, None),
        };

        // Analisar âncoras no corpo da regra
        let (domain_anchor, body_after_anchor) = if let Some(stripped) = rule_body.strip_prefix("||") {
            (true, stripped)
        } else {
            (false, rule_body)
        };

        let (start_anchor, body_after_start) = if !domain_anchor && body_after_anchor.starts_with('|') {
            (true, &body_after_anchor[1..])
        } else {
            (false, body_after_anchor)
        };

        let (end_anchor, pattern) = if body_after_start.ends_with('|') && !body_after_start.ends_with("||") {
            (true, &body_after_start[..body_after_start.len() - 1])
        } else {
            (false, body_after_start)
        };

        let mut third_party = None;
        let mut resource_types = HashSet::new();
        let mut domains = HashSet::new();
        let mut excluded_domains = HashSet::new();

        if let Some(opts) = options_part {
            for opt in opts.split(',') {
                let opt_trim = opt.trim();
                if opt_trim == "third-party" || opt_trim == "3p" {
                    third_party = Some(true);
                } else if opt_trim == "~third-party" || opt_trim == "~3p" || opt_trim == "1p" {
                    third_party = Some(false);
                } else if let Some(domain_list) = opt_trim.strip_prefix("domain=") {
                    for d in domain_list.split('|') {
                        let d_clean = d.trim().to_lowercase();
                        if let Some(negated) = d_clean.strip_prefix('~') {
                            excluded_domains.insert(negated.to_string());
                        } else if !d_clean.is_empty() {
                            domains.insert(d_clean);
                        }
                    }
                } else if let Some(rt) = ResourceType::from_str(opt_trim) {
                    resource_types.insert(rt);
                }
            }
        }

        Some(ParsedRule {
            raw: trimmed.to_string(),
            kind: if is_whitelist { RuleKind::Whitelist } else { RuleKind::Network },
            pattern: pattern.to_lowercase(),
            domain_anchor,
            start_anchor,
            end_anchor,
            third_party,
            resource_types: if resource_types.is_empty() { None } else { Some(resource_types) },
            domains: if domains.is_empty() { None } else { Some(domains) },
            excluded_domains: if excluded_domains.is_empty() { None } else { Some(excluded_domains) },
        })
    }

    pub fn matches(
        &self,
        url_lower: &str,
        url_domain: &str,
        source_domain: Option<&str>,
        resource_type: Option<ResourceType>,
    ) -> bool {
        // Verificar tipo de recurso
        if let Some(ref required_types) = self.resource_types {
            if let Some(rt) = resource_type {
                if !required_types.contains(&rt) {
                    return false;
                }
            }
        }

        // Verificar terceiro partido
        if let Some(is_3p_required) = self.third_party {
            if let Some(src) = source_domain {
                let is_third_party = !Self::are_same_domain(src, url_domain);
                if is_third_party != is_3p_required {
                    return false;
                }
            }
        }

        // Verificar restrições de domínio de origem
        if let Some(src) = source_domain {
            if let Some(ref excluded) = self.excluded_domains {
                if excluded.contains(src) {
                    return false;
                }
            }
            if let Some(ref allowed) = self.domains {
                if !allowed.contains(src) {
                    return false;
                }
            }
        }

        // Comparar padrão na URL
        if self.domain_anchor {
            // ||example.com^ casa com https://example.com/ ou https://sub.example.com/
            let clean_pattern = self.pattern.trim_end_matches('^');
            if url_domain == clean_pattern || url_domain.ends_with(&format!(".{}", clean_pattern)) {
                return true;
            }
            if url_lower.contains(clean_pattern) {
                return true;
            }
            return false;
        }

        if self.start_anchor {
            if !url_lower.starts_with(&self.pattern) {
                return false;
            }
        }

        if self.end_anchor {
            if !url_lower.ends_with(&self.pattern) {
                return false;
            }
        }

        let clean_pattern = self.pattern.trim_matches('^');
        url_lower.contains(clean_pattern)
    }

    fn are_same_domain(d1: &str, d2: &str) -> bool {
        let b1 = Self::base_domain(d1);
        let b2 = Self::base_domain(d2);
        b1 == b2
    }

    pub fn base_domain(domain: &str) -> &str {
        let parts: Vec<&str> = domain.split('.').collect();
        if parts.len() <= 2 {
            domain
        } else {
            let end_idx = domain.rfind('.').unwrap_or(0);
            if let Some(second_dot) = domain[..end_idx].rfind('.') {
                &domain[second_dot + 1..]
            } else {
                domain
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_domain_anchor_rule() {
        let rule = ParsedRule::parse("||doubleclick.net^$third-party,script").unwrap();
        assert_eq!(rule.kind, RuleKind::Network);
        assert!(rule.domain_anchor);
        assert_eq!(rule.pattern, "doubleclick.net^");
        assert_eq!(rule.third_party, Some(true));
        assert!(rule.resource_types.as_ref().unwrap().contains(&ResourceType::Script));

        // Testar match
        assert!(rule.matches(
            "https://ad.doubleclick.net/pixel.js",
            "ad.doubleclick.net",
            Some("example.com"),
            Some(ResourceType::Script),
        ));

        // Não deve dar match se for do mesmo domínio
        assert!(!rule.matches(
            "https://ad.doubleclick.net/pixel.js",
            "ad.doubleclick.net",
            Some("doubleclick.net"),
            Some(ResourceType::Script),
        ));
    }

    #[test]
    fn test_parse_whitelist_exception() {
        let rule = ParsedRule::parse("@@||linuxmint.com^").unwrap();
        assert_eq!(rule.kind, RuleKind::Whitelist);
        assert!(rule.domain_anchor);
        assert!(rule.matches("https://linuxmint.com/news", "linuxmint.com", None, None));
    }

    #[test]
    fn test_parse_cosmetic_rule() {
        let rule = ParsedRule::parse("kernel.org##.ad-header").unwrap();
        match rule.kind {
            RuleKind::Cosmetic { selector } => assert_eq!(selector, ".ad-header"),
            _ => panic!("Esperado RuleKind::Cosmetic"),
        }
    }
}
