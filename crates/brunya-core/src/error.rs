use thiserror::Error;

#[derive(Error, Debug)]
pub enum BrunyaError {
    #[error("Erro de inicialização do motor web: {0}")]
    EngineInitFailed(String),

    #[error("Aba não encontrada: {0}")]
    TabNotFound(String),

    #[error("URL inválida: {0}")]
    InvalidUrl(String),

    #[error("Erro de armazenamento local: {0}")]
    StorageError(String),

    #[error("Erro de internacionalização: {0}")]
    I18nError(String),

    #[error("Erro de configuração: {0}")]
    SettingsError(String),

    #[error("Erro de rede / requisição bloqueada: {0}")]
    NetworkError(String),

    #[error("Erro de processo de subprocesso/sandbox: {0}")]
    ProcessError(String),

    #[error("Erro de I/O: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Erro interno desconhecido: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, BrunyaError>;
