//! Gerenciamento de abas e política inteligente de suspensão de memória do BEAR Browser.

use bear_core::types::TabId;
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabDescriptor {
    pub id: TabId,
    pub title: String,
    pub url: String,
    pub favicon_url: Option<String>,
    pub is_pinned: bool,
    pub is_playing_audio: bool,
    pub is_suspended: bool,
    pub created_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
}

impl TabDescriptor {
    pub fn new(id: TabId, url: &str, title: &str) -> Self {
        let now = Utc::now();
        Self {
            id,
            title: title.to_string(),
            url: url.to_string(),
            favicon_url: None,
            is_pinned: false,
            is_playing_audio: false,
            is_suspended: false,
            created_at: now,
            last_active_at: now,
        }
    }

    pub fn can_be_suspended(&self) -> bool {
        !self.is_pinned && !self.is_playing_audio && !self.is_suspended
    }
}

pub struct TabManager {
    tabs: RwLock<Vec<TabDescriptor>>,
    active_tab_id: RwLock<Option<TabId>>,
    closed_history: RwLock<VecDeque<TabDescriptor>>,
    max_closed_history: usize,
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            tabs: RwLock::new(Vec::new()),
            active_tab_id: RwLock::new(None),
            closed_history: RwLock::new(VecDeque::new()),
            max_closed_history: 25,
        }
    }

    pub fn open_tab(&self, url: &str, title: &str) -> TabDescriptor {
        let tab = TabDescriptor::new(TabId::new(), url, title);
        let mut tabs = self.tabs.write();
        tabs.push(tab.clone());
        *self.active_tab_id.write() = Some(tab.id);
        tab
    }

    pub fn close_tab(&self, tab_id: TabId) -> Option<TabDescriptor> {
        let mut tabs = self.tabs.write();
        if let Some(pos) = tabs.iter().position(|t| t.id == tab_id) {
            let removed = tabs.remove(pos);
            
            // Guardar para restauração (Ctrl+Shift+T)
            let mut closed = self.closed_history.write();
            if closed.len() >= self.max_closed_history {
                closed.pop_front();
            }
            closed.push_back(removed.clone());

            // Atualizar aba ativa se a fechada era a ativa
            let mut active = self.active_tab_id.write();
            if *active == Some(tab_id) {
                if !tabs.is_empty() {
                    let next_pos = if pos >= tabs.len() { tabs.len() - 1 } else { pos };
                    *active = Some(tabs[next_pos].id);
                    tabs[next_pos].last_active_at = Utc::now();
                } else {
                    *active = None;
                }
            }

            Some(removed)
        } else {
            None
        }
    }

    pub fn set_active_tab(&self, tab_id: TabId) -> bool {
        let mut tabs = self.tabs.write();
        if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.last_active_at = Utc::now();
            if tab.is_suspended {
                info!("Reativando aba suspensa: {} ({})", tab.title, tab.url);
                tab.is_suspended = false;
            }
            *self.active_tab_id.write() = Some(tab_id);
            true
        } else {
            false
        }
    }

    pub fn active_tab(&self) -> Option<TabDescriptor> {
        let active_id = (*self.active_tab_id.read())?;
        self.tabs.read().iter().find(|t| t.id == active_id).cloned()
    }

    pub fn list_tabs(&self) -> Vec<TabDescriptor> {
        self.tabs.read().clone()
    }

    pub fn restore_last_closed_tab(&self) -> Option<TabDescriptor> {
        let last_closed = self.closed_history.write().pop_back()?;
        let restored = self.open_tab(&last_closed.url, &last_closed.title);
        Some(restored)
    }

    pub fn update_tab_metadata(&self, tab_id: TabId, url: &str, title: &str, favicon: Option<String>) {
        let mut tabs = self.tabs.write();
        if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.url = url.to_string();
            if !title.is_empty() {
                tab.title = title.to_string();
            }
            if favicon.is_some() {
                tab.favicon_url = favicon;
            }
        }
    }

    pub fn toggle_pin_tab(&self, tab_id: TabId) -> bool {
        let mut tabs = self.tabs.write();
        if let Some(tab) = tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.is_pinned = !tab.is_pinned;
            tab.is_pinned
        } else {
            false
        }
    }

    /// Executa a política inteligente de suspensão de abas inativas para economia de memória RAM.
    /// Retorna os IDs das abas que foram suspensas nesta rodada.
    pub fn suspend_inactive_tabs(&self, idle_timeout: Duration) -> Vec<TabId> {
        let active_id = *self.active_tab_id.read();
        let now = Utc::now();
        let mut tabs = self.tabs.write();
        let mut suspended = Vec::new();

        for tab in tabs.iter_mut() {
            if Some(tab.id) == active_id {
                continue; // Nunca suspender a aba atualmente visível
            }

            if tab.can_be_suspended() && (now - tab.last_active_at) > idle_timeout {
                tab.is_suspended = true;
                suspended.push(tab.id);
                info!("Aba {} colocada em suspensão inteligente de memória", tab.id);
            }
        }

        suspended
    }
}

impl Default for TabManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_lifecycle() {
        let mgr = TabManager::new();

        let t1 = mgr.open_tab("https://linuxmint.com", "Linux Mint");
        let t2 = mgr.open_tab("https://kernel.org", "Kernel");

        assert_eq!(mgr.list_tabs().len(), 2);
        assert_eq!(mgr.active_tab().unwrap().id, t2.id);

        mgr.close_tab(t2.id);
        assert_eq!(mgr.list_tabs().len(), 1);
        assert_eq!(mgr.active_tab().unwrap().id, t1.id);

        let restored = mgr.restore_last_closed_tab().unwrap();
        assert_eq!(restored.url, "https://kernel.org");
        assert_eq!(mgr.list_tabs().len(), 2);
    }

    #[test]
    fn test_tab_suspension() {
        let mgr = TabManager::new();
        let t1 = mgr.open_tab("https://docs.rs", "Docs");
        let _t2 = mgr.open_tab("https://rust-lang.org", "Rust");

        // Simular que t1 está inativo há mais de 10 minutos
        {
            let mut tabs = mgr.tabs.write();
            tabs[0].last_active_at = Utc::now() - Duration::minutes(15);
        }

        let suspended = mgr.suspend_inactive_tabs(Duration::minutes(10));
        assert_eq!(suspended.len(), 1);
        assert_eq!(suspended[0], t1.id);

        // Ativar t1 remove suspensão
        mgr.set_active_tab(t1.id);
        let tab = mgr.active_tab().unwrap();
        assert!(!tab.is_suspended);
    }
}
