//! Gerenciamento de runtime de extensões.
//!
//! Gerencia service workers (simulados via JavaScriptCore/WebKit),
//! ciclos de vida, mensagens entre contextos e eventos.

use crate::errors::{ExtensionError, Result};
use crate::security::SecurityManager;
use crate::types::{
    ExtensionEvent, ExtensionId, ExtensionInfo, ExtensionMessage, RuntimeSessionId,
};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::info;

/// Estado do Service Worker de uma extensão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceWorkerState {
    /// Nunca iniciado.
    Idle,
    /// Inicializando.
    Starting,
    /// Em execução.
    Running,
    /// Suspenso por inatividade.
    Suspended,
    /// Com erro.
    Error(String),
    /// Encerrado.
    Stopped,
}

/// Contexto de execução de uma extensão.
pub struct ExtensionContext {
    pub info: ExtensionInfo,
    pub session_id: RuntimeSessionId,
    pub sw_state: RwLock<ServiceWorkerState>,
    pub last_activity: RwLock<Instant>,
    pub event_listeners: RwLock<Vec<String>>,
    pub content_scripts_active: RwLock<bool>,
}

/// Runtime de extensões — gerencia ciclo de vida e execução.
pub struct ExtensionRuntime {
    _security: Arc<SecurityManager>,
    contexts: RwLock<HashMap<String, Arc<ExtensionContext>>>,
    event_queue: RwLock<Vec<(ExtensionId, serde_json::Value)>>,
}

impl ExtensionRuntime {
    pub fn new(security: Arc<SecurityManager>) -> Self {
        Self {
            _security: security,
            contexts: RwLock::new(HashMap::new()),
            event_queue: RwLock::new(Vec::new()),
        }
    }

    /// Inicializa o runtime para uma extensão recém-carregada.
    pub fn initialize_context(&self, info: &ExtensionInfo) -> Arc<ExtensionContext> {
        let ctx = Arc::new(ExtensionContext {
            info: info.clone(),
            session_id: RuntimeSessionId::new(),
            sw_state: RwLock::new(ServiceWorkerState::Idle),
            last_activity: RwLock::new(Instant::now()),
            event_listeners: RwLock::new(Vec::new()),
            content_scripts_active: RwLock::new(!info.content_scripts.is_empty()),
        });

        self.contexts.write().insert(info.id.0.clone(), ctx.clone());
        info!("Runtime inicializado para extensão: {}", info.name);
        ctx
    }

    /// Obtém o contexto de uma extensão.
    pub fn get_context(&self, ext_id: &ExtensionId) -> Option<Arc<ExtensionContext>> {
        self.contexts.read().get(&ext_id.0).cloned()
    }

    /// Remove o contexto de runtime quando a extensão é desinstalada.
    pub fn remove_context(&self, ext_id: &ExtensionId) {
        self.contexts.write().remove(&ext_id.0);
    }

    /// Inicia (ou "acorda") o Service Worker de uma extensão.
    pub fn start_service_worker(&self, ext_id: &ExtensionId) -> Result<()> {
        let contexts = self.contexts.read();
        let ctx = contexts.get(&ext_id.0).ok_or_else(|| {
            ExtensionError::NotFound(format!("Contexto não encontrado: {}", ext_id))
        })?;

        let mut state = ctx.sw_state.write();
        match *state {
            ServiceWorkerState::Running => {
                // Já está rodando, apenas registrar atividade
                *ctx.last_activity.write() = Instant::now();
                return Ok(());
            }
            ServiceWorkerState::Starting => {
                return Err(ExtensionError::ServiceWorkerError(
                    "Service Worker já está iniciando".to_string(),
                ));
            }
            ServiceWorkerState::Error(ref e) => {
                return Err(ExtensionError::ServiceWorkerError(format!(
                    "Service Worker em estado de erro: {}",
                    e
                )));
            }
            _ => {} // Idle, Suspended, Stopped — podemos iniciar
        }

        *state = ServiceWorkerState::Starting;
        drop(state);

        // Em uma implementação completa, isso executaria o service_worker.js
        // em um ambiente JavaScript isolado (JavaScriptCore via WebKit, ou
        // quickjs/deno_core para execução headless).
        //
        // No polyfill atual, registramos o SW como "Running" imediatamente.
        *ctx.sw_state.write() = ServiceWorkerState::Running;
        *ctx.last_activity.write() = Instant::now();

        info!(
            "Service Worker iniciado para extensão: {} ({})",
            ctx.info.name, ext_id
        );

        // Processar eventos enfileirados
        self.dispatch_queued_events(ext_id);

        Ok(())
    }

    /// Suspende o Service Worker por inatividade (Manifest V3).
    pub fn suspend_service_worker(&self, ext_id: &ExtensionId) -> Result<()> {
        let contexts = self.contexts.read();
        if let Some(ctx) = contexts.get(&ext_id.0) {
            let mut state = ctx.sw_state.write();
            if *state == ServiceWorkerState::Running {
                *state = ServiceWorkerState::Suspended;
                info!("Service Worker suspenso por inatividade: {}", ctx.info.name);
            }
        }
        Ok(())
    }

    /// Enfileira um evento para ser processado pelo Service Worker.
    pub fn dispatch_event(&self, ext_id: &ExtensionId, event: ExtensionEvent) {
        let event_json = serde_json::to_value(&event).unwrap_or_default();

        let contexts = self.contexts.read();
        if let Some(ctx) = contexts.get(&ext_id.0) {
            let state = ctx.sw_state.read();
            if *state == ServiceWorkerState::Running || *state == ServiceWorkerState::Starting {
                // SW ativo — processar imediatamente
                drop(state);
                info!(
                    "Evento despachado para {}: {:?}",
                    ctx.info.name,
                    std::mem::discriminant(&event)
                );
                *ctx.last_activity.write() = Instant::now();

                // Em uma implementação real, isso chamaria os listeners registrados
                // no contexto JavaScript do Service Worker
            } else {
                drop(state);
                // SW inativo — enfileirar e acordar
                self.queue_event(ext_id, event_json);
                if *ctx.sw_state.read() != ServiceWorkerState::Starting {
                    let _ = self.start_service_worker(ext_id);
                }
            }
        }
    }

    /// Enfileira um evento para processamento posterior.
    fn queue_event(&self, ext_id: &ExtensionId, data: serde_json::Value) {
        let mut queue = self.event_queue.write();
        queue.push((ext_id.clone(), data));
    }

    /// Despacha eventos enfileirados para uma extensão.
    fn dispatch_queued_events(&self, ext_id: &ExtensionId) {
        let mut queue = self.event_queue.write();
        queue.retain(|(id, _)| id != ext_id);
    }

    /// Registra um listener de evento para uma extensão.
    pub fn add_event_listener(&self, ext_id: &ExtensionId, event: &str) {
        if let Some(ctx) = self.contexts.read().get(&ext_id.0) {
            ctx.event_listeners.write().push(event.to_string());
        }
    }

    /// Remove um listener de evento.
    pub fn remove_event_listener(&self, ext_id: &ExtensionId, event: &str) {
        if let Some(ctx) = self.contexts.read().get(&ext_id.0) {
            ctx.event_listeners.write().retain(|e| e != event);
        }
    }

    /// Envia uma mensagem de runtime entre extensões ou da página para extensão.
    pub fn send_message(&self, target: &ExtensionId, message: ExtensionMessage) -> Result<()> {
        let contexts = self.contexts.read();
        if contexts.contains_key(&target.0) {
            self.dispatch_event(target, ExtensionEvent::RuntimeOnMessage { message });
            Ok(())
        } else {
            Err(ExtensionError::NotFound(format!(
                "Extensão destino não encontrada: {}",
                target
            )))
        }
    }

    /// Retorna o estado do Service Worker de uma extensão.
    pub fn sw_state(&self, ext_id: &ExtensionId) -> ServiceWorkerState {
        self.contexts
            .read()
            .get(&ext_id.0)
            .map(|ctx| ctx.sw_state.read().clone())
            .unwrap_or(ServiceWorkerState::Stopped)
    }

    /// Suspende automaticamente SWs inativos (chamado periodicamente).
    pub fn suspend_idle_workers(&self, idle_timeout: Duration) -> Vec<ExtensionId> {
        let mut suspended = Vec::new();
        let contexts = self.contexts.read();
        let now = Instant::now();

        for (id, ctx) in contexts.iter() {
            let state = ctx.sw_state.read();
            if *state == ServiceWorkerState::Running {
                let idle = now - *ctx.last_activity.read();
                if idle > idle_timeout {
                    drop(state);
                    *ctx.sw_state.write() = ServiceWorkerState::Suspended;
                    suspended.push(ExtensionId(id.clone()));
                    info!(
                        "SW suspenso automaticamente: {} (ocioso por {:?})",
                        id, idle
                    );
                }
            }
        }

        suspended
    }

    /// Verifica se content scripts estão ativos para uma extensão.
    pub fn has_active_content_scripts(&self, ext_id: &ExtensionId) -> bool {
        self.contexts
            .read()
            .get(&ext_id.0)
            .map(|ctx| *ctx.content_scripts_active.read())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BackgroundType;

    fn create_test_ext() -> ExtensionInfo {
        ExtensionInfo {
            id: ExtensionId("runtime-test".to_string()),
            name: "Runtime Test".to_string(),
            version: "1.0.0".to_string(),
            background_type: Some(BackgroundType::ServiceWorker),
            ..Default::default()
        }
    }

    #[test]
    fn test_service_worker_lifecycle() {
        let security = Arc::new(SecurityManager::new());
        let runtime = ExtensionRuntime::new(security);

        let info = create_test_ext();
        let ctx = runtime.initialize_context(&info);

        assert_eq!(*ctx.sw_state.read(), ServiceWorkerState::Idle);

        runtime.start_service_worker(&info.id).unwrap();
        assert_eq!(*ctx.sw_state.read(), ServiceWorkerState::Running);

        runtime.suspend_service_worker(&info.id).unwrap();
        assert_eq!(*ctx.sw_state.read(), ServiceWorkerState::Suspended);
    }

    #[test]
    fn test_event_dispatching() {
        let security = Arc::new(SecurityManager::new());
        let runtime = ExtensionRuntime::new(security);

        let info = create_test_ext();
        runtime.initialize_context(&info);
        runtime.start_service_worker(&info.id).unwrap();

        runtime.dispatch_event(&info.id, ExtensionEvent::RuntimeOnStartup);

        let sw_state = runtime.sw_state(&info.id);
        assert_eq!(sw_state, ServiceWorkerState::Running);
    }
}
