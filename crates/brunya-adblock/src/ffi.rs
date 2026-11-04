//! Interface C-ABI estável (FFI) para integração com Chromium e stacks nativas.

use crate::modules::profiles::ShieldProfile;
use crate::AdblockEngine;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::catch_unwind;
use std::sync::OnceLock;

static GLOBAL_ENGINE: OnceLock<AdblockEngine> = OnceLock::new();

fn get_engine() -> &'static AdblockEngine {
    GLOBAL_ENGINE.get_or_init(AdblockEngine::new)
}

#[repr(C)]
pub struct CStatistics {
    pub total_blocked: u64,
    pub ads_blocked: u64,
    pub trackers_blocked: u64,
    pub threats_detected: u64,
    pub annoyances_removed: u64,
    pub urls_cleaned: u64,
}

/// Inicializa o BruNya Shield nativo.
#[no_mangle]
pub extern "C" fn shield_initialize() -> bool {
    catch_unwind(|| {
        let _ = get_engine();
        true
    })
    .unwrap_or(false)
}

/// Avalia se uma requisição de rede deve ser bloqueada pelo Shield.
///
/// # Safety
///
/// `url` e `source_domain` são ponteiros C. `url` nulo faz a função
/// devolver falso. `source_domain` pode ser nulo. Os ponteiros não nulos
/// precisam apontar para texto UTF-8 terminado em nul que permaneça válido
/// durante a chamada.
#[no_mangle]
pub unsafe extern "C" fn shield_check_request(
    url: *const c_char,
    source_domain: *const c_char,
) -> bool {
    catch_unwind(|| {
        if url.is_null() {
            return false;
        }
        let c_url = unsafe { CStr::from_ptr(url) };
        let url_str = match c_url.to_str() {
            Ok(s) => s,
            Err(_) => return false,
        };

        let src_domain_opt = if source_domain.is_null() {
            None
        } else {
            unsafe { CStr::from_ptr(source_domain) }.to_str().ok()
        };

        let engine = get_engine();
        engine.should_block(brunya_core::types::TabId(0), url_str, src_domain_opt, None)
    })
    .unwrap_or(false)
}

/// Retorna a folha de estilo CSS de filtragem cosmética padrão.
#[no_mangle]
pub extern "C" fn shield_get_cosmetic_rules() -> *const c_char {
    catch_unwind(|| {
        let css = crate::modules::CosmeticFilter::default_cosmetic_stylesheet();
        match CString::new(css) {
            Ok(c) => c.into_raw() as *const c_char,
            Err(_) => std::ptr::null(),
        }
    })
    .unwrap_or(std::ptr::null())
}

/// Libera uma string alocada por `shield_get_cosmetic_rules`.
///
/// # Safety
///
/// `s` é nulo ou foi obtido de `shield_get_cosmetic_rules` e ainda não foi
/// liberado. Outro ponteiro não pode ser passado.
#[no_mangle]
pub unsafe extern "C" fn shield_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = catch_unwind(|| unsafe {
            let _ = CString::from_raw(s);
        });
    }
}

/// Altera o perfil de proteção (0: Essencial, 1: Equilibrado, 2: Rigoroso, 3: Personalizado).
#[no_mangle]
pub extern "C" fn shield_set_profile(profile_code: i32) -> bool {
    catch_unwind(|| {
        let profile = match profile_code {
            0 => ShieldProfile::Essential,
            1 => ShieldProfile::Balanced,
            2 => ShieldProfile::Strict,
            3 => ShieldProfile::Custom,
            _ => return false,
        };
        get_engine().set_profile(profile);
        true
    })
    .unwrap_or(false)
}

/// Adiciona uma exceção para o domínio informado.
///
/// # Safety
///
/// `domain` é nulo ou aponta para texto UTF-8 terminado em nul, válido
/// durante a chamada.
#[no_mangle]
pub unsafe extern "C" fn shield_add_site_exception(domain: *const c_char) -> bool {
    catch_unwind(|| {
        if domain.is_null() {
            return false;
        }
        let c_dom = unsafe { CStr::from_ptr(domain) };
        if let Ok(dom_str) = c_dom.to_str() {
            get_engine().allow_site(dom_str);
            true
        } else {
            false
        }
    })
    .unwrap_or(false)
}

/// Consulta as estatísticas consolidadas de proteção.
#[no_mangle]
pub extern "C" fn shield_get_statistics() -> CStatistics {
    catch_unwind(|| {
        let stats = get_engine().stats().get_statistics();
        CStatistics {
            total_blocked: stats.total_blocked,
            ads_blocked: stats.ads_blocked,
            trackers_blocked: stats.trackers_blocked,
            threats_detected: stats.threats_detected,
            annoyances_removed: stats.annoyances_removed,
            urls_cleaned: stats.urls_cleaned,
        }
    })
    .unwrap_or(CStatistics {
        total_blocked: 0,
        ads_blocked: 0,
        trackers_blocked: 0,
        threats_detected: 0,
        annoyances_removed: 0,
        urls_cleaned: 0,
    })
}

/// Recarrega as regras dos motores de filtro.
#[no_mangle]
pub extern "C" fn shield_reload_filter_engine() -> bool {
    catch_unwind(|| {
        get_engine().reload_rules();
        true
    })
    .unwrap_or(false)
}

/// Encerramento seguro do BruNya Shield.
#[no_mangle]
pub extern "C" fn shield_shutdown() -> bool {
    true
}
