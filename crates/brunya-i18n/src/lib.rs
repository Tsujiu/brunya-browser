//! Sistema centralizado de internacionalização do Brunya Browser.
//!
//! Suporta pt-BR (referência padrão) e en (oficial secundário),
//! interpolação segura de variáveis, detecção do locale do sistema
//! e testes de paridade de chaves.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    #[default]
    PtBr,
    En,
}

impl Language {
    pub fn code(&self) -> &'static str {
        match self {
            Language::PtBr => "pt-BR",
            Language::En => "en",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Language::PtBr => "Português (Brasil)",
            Language::En => "English",
        }
    }

    pub fn from_locale_str(locale: &str) -> Self {
        let normalized = locale.to_lowercase().replace('_', "-");
        if normalized.starts_with("pt") {
            Language::PtBr
        } else if normalized.starts_with("en") {
            Language::En
        } else {
            Language::PtBr // Padrão de referência
        }
    }

    pub fn detect_system() -> Self {
        let env_vars = ["LC_ALL", "LC_MESSAGES", "LANG"];
        for var in env_vars {
            if let Ok(val) = std::env::var(var) {
                if !val.is_empty() {
                    return Self::from_locale_str(&val);
                }
            }
        }
        Language::PtBr
    }
}

pub struct I18nManager {
    current_language: RwLock<Language>,
    catalogues: HashMap<Language, HashMap<String, String>>,
}

static INSTANCE: OnceLock<I18nManager> = OnceLock::new();

impl I18nManager {
    pub fn global() -> &'static I18nManager {
        INSTANCE.get_or_init(|| {
            let mut mgr = I18nManager {
                current_language: RwLock::new(Language::detect_system()),
                catalogues: HashMap::new(),
            };
            mgr.load_embedded();
            mgr
        })
    }

    pub fn set_language(&self, lang: Language) {
        *self.current_language.write() = lang;
    }

    pub fn current_language(&self) -> Language {
        *self.current_language.read()
    }

    fn load_embedded(&mut self) {
        let mut pt_br_map = HashMap::new();
        Self::merge_json(
            &mut pt_br_map,
            "common",
            include_str!("../../../locales/pt-BR/common.json"),
        );
        Self::merge_json(
            &mut pt_br_map,
            "settings",
            include_str!("../../../locales/pt-BR/settings.json"),
        );
        Self::merge_json(
            &mut pt_br_map,
            "privacy",
            include_str!("../../../locales/pt-BR/privacy.json"),
        );
        Self::merge_json(
            &mut pt_br_map,
            "errors",
            include_str!("../../../locales/pt-BR/errors.json"),
        );
        Self::merge_json(
            &mut pt_br_map,
            "downloads",
            include_str!("../../../locales/pt-BR/downloads.json"),
        );
        Self::merge_json(
            &mut pt_br_map,
            "extensions",
            include_str!("../../../locales/pt-BR/extensions.json"),
        );
        self.catalogues.insert(Language::PtBr, pt_br_map);

        let mut en_map = HashMap::new();
        Self::merge_json(
            &mut en_map,
            "common",
            include_str!("../../../locales/en/common.json"),
        );
        Self::merge_json(
            &mut en_map,
            "settings",
            include_str!("../../../locales/en/settings.json"),
        );
        Self::merge_json(
            &mut en_map,
            "privacy",
            include_str!("../../../locales/en/privacy.json"),
        );
        Self::merge_json(
            &mut en_map,
            "errors",
            include_str!("../../../locales/en/errors.json"),
        );
        Self::merge_json(
            &mut en_map,
            "downloads",
            include_str!("../../../locales/en/downloads.json"),
        );
        Self::merge_json(
            &mut en_map,
            "extensions",
            include_str!("../../../locales/en/extensions.json"),
        );
        self.catalogues.insert(Language::En, en_map);
    }

    fn merge_json(map: &mut HashMap<String, String>, prefix: &str, raw_json: &str) {
        if let Ok(entries) = serde_json::from_str::<HashMap<String, String>>(raw_json) {
            for (k, v) in entries {
                map.insert(format!("{}.{}", prefix, k), v);
            }
        }
    }

    pub fn translate(&self, key: &str, vars: &[(&str, &str)]) -> String {
        let current = self.current_language();

        // 1. Tentar idioma atual
        let template = self
            .catalogues
            .get(&current)
            .and_then(|cat| cat.get(key))
            // 2. Fallback obrigatório para pt-BR
            .or_else(|| {
                self.catalogues
                    .get(&Language::PtBr)
                    .and_then(|cat| cat.get(key))
            });

        let mut result = match template {
            Some(t) => t.clone(),
            None => format!("[MISSING: {}]", key),
        };

        // Interpolação de variáveis
        for (k, v) in vars {
            let placeholder = format!("{{{}}}", k);
            result = result.replace(&placeholder, v);
        }

        result
    }

    pub fn all_keys_for(&self, lang: Language) -> Vec<String> {
        self.catalogues
            .get(&lang)
            .map(|c| c.keys().cloned().collect())
            .unwrap_or_default()
    }
}

/// Macro conveniente para tradução com interpolação opcional
#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::I18nManager::global().translate($key, &[])
    };
    ($key:expr, $($k:ident => $v:expr),+ $(,)?) => {
        $crate::I18nManager::global().translate($key, &[
            $((stringify!($k), &$v.to_string())),+
        ])
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i18n_translation_and_fallback() {
        let i18n = I18nManager::global();
        i18n.set_language(Language::PtBr);

        assert_eq!(t!("common.app_name"), "Brunya Browser");
        assert_eq!(
            t!("privacy.blocked_trackers_count", count => 15),
            "15 rastreadores e anúncios bloqueados"
        );

        i18n.set_language(Language::En);
        assert_eq!(t!("common.new_tab"), "New Tab");
        assert_eq!(
            t!("privacy.blocked_trackers_count", count => 42),
            "42 trackers and ads blocked"
        );
    }

    #[test]
    fn test_catalogues_parity() {
        let i18n = I18nManager::global();
        let pt_keys: std::collections::HashSet<_> =
            i18n.all_keys_for(Language::PtBr).into_iter().collect();
        let en_keys: std::collections::HashSet<_> =
            i18n.all_keys_for(Language::En).into_iter().collect();

        assert!(!pt_keys.is_empty(), "Catálogo pt-BR não pode estar vazio");
        assert!(!en_keys.is_empty(), "Catálogo en não pode estar vazio");

        let missing_in_en: Vec<_> = pt_keys.difference(&en_keys).collect();
        let missing_in_pt: Vec<_> = en_keys.difference(&pt_keys).collect();

        assert!(
            missing_in_en.is_empty(),
            "Chaves presentes em pt-BR mas ausentes em en: {:?}",
            missing_in_en
        );
        assert!(
            missing_in_pt.is_empty(),
            "Chaves presentes em en mas ausentes em pt-BR: {:?}",
            missing_in_pt
        );
    }
}
