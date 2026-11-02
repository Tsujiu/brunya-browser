use crate::error::Result;
use crate::types::{NavigationState, TabId};
use std::sync::Arc;

/// Callback de pintura para Off-Screen Rendering (OSR).
/// Chamado pelo motor de renderização quando novos frames de pixels são gerados.
pub trait RenderCallback: Send + Sync {
    fn on_paint(&self, tab_id: TabId, width: u32, height: u32, buffer: &[u8]);
}

/// Ação decidida pelo interceptador de requisições de rede.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestAction {
    Allow,
    Block { reason: String },
    Redirect { target_url: String },
}

/// Interceptador de rede para bloqueio de anúncios e privacidade.
pub trait NetworkInterceptor: Send + Sync {
    fn inspect_request(&self, tab_id: TabId, url: &str, resource_type: &str) -> RequestAction;
}

/// Interface abstrata de uma aba do navegador no motor web.
pub trait BrowserTab: Send + Sync {
    fn id(&self) -> TabId;
    fn navigate(&self, url: &str) -> Result<()>;
    fn go_back(&self) -> Result<()>;
    fn go_forward(&self) -> Result<()>;
    fn reload(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn resize(&self, width: u32, height: u32) -> Result<()>;
    fn send_mouse_move(&self, x: i32, y: i32, mouse_leave: bool) -> Result<()>;
    fn send_mouse_click(&self, x: i32, y: i32, button: MouseButton, is_up: bool, click_count: i32) -> Result<()>;
    fn send_scroll(&self, x: i32, y: i32, delta_x: f64, delta_y: f64) -> Result<()>;
    fn send_key_event(&self, key_type: KeyEventType, windows_key_code: i32, native_key_code: i32, is_system_key: bool) -> Result<()>;
    fn navigation_state(&self) -> NavigationState;
    fn execute_javascript(&self, script: &str) -> Result<()>;
    fn is_suspended(&self) -> bool;
    fn suspend(&self) -> Result<()>;
    fn resume(&self) -> Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventType {
    RawKeyDown,
    KeyDown,
    KeyUp,
    Char,
}

/// Interface do motor de navegação desacoplado.
pub trait BrowserEngine: Send + Sync {
    fn name(&self) -> &'static str;
    fn version(&self) -> String;
    fn initialize(&self) -> Result<()>;
    fn shutdown(&self) -> Result<()>;
    fn do_work(&self);
    fn create_tab(&self, initial_url: &str, render_callback: Arc<dyn RenderCallback>) -> Result<Arc<dyn BrowserTab>>;
    fn close_tab(&self, tab_id: TabId) -> Result<()>;
    fn set_network_interceptor(&self, interceptor: Arc<dyn NetworkInterceptor>);
}
