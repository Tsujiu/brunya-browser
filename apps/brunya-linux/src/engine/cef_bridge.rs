//! Ponte de integração direta entre Chromium Embedded Framework (CEF) e GTK4.

#[cfg(feature = "cef-engine")]
use cef::{api_hash, sys};
use tracing::info;

pub struct CefBridge {
    pub is_initialized: bool,
}

impl CefBridge {
    pub fn new() -> Self {
        Self { is_initialized: false }
    }

    pub fn initialize(&mut self) -> Result<(), String> {
        #[cfg(feature = "cef-engine")]
        unsafe {
            info!("Inicializando CEF Chromium Engine...");
            let hash_ptr = api_hash(sys::CEF_API_VERSION_LAST, 0);
            if !hash_ptr.is_null() {
                let cstr = std::ffi::CStr::from_ptr(hash_ptr);
                info!("CEF Engine carregado. API Hash: {:?}", cstr);
            }
            self.is_initialized = true;
        }

        #[cfg(not(feature = "cef-engine"))]
        {
            info!("CEF Engine não habilitado nesta compilação (usando motor nativo otimizado).");
        }

        Ok(())
    }

    pub fn pump_message_loop(&self) {
        #[cfg(feature = "cef-engine")]
        if self.is_initialized {
            // CEF message loop pump
        }
    }
}
