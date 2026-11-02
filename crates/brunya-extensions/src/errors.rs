//! Tipos de erro do sistema de extensões.

use thiserror::Error;

/// Converte erro de storage para erro de extensão.
impl From<brunya_storage::StorageError> for ExtensionError {
    fn from(e: brunya_storage::StorageError) -> Self {
        ExtensionError::StorageError(e.to_string())
    }
}

#[derive(Error, Debug)]
pub enum ExtensionError {
    #[error("Extensão não encontrada: {0}")]
    NotFound(String),

    #[error("Extensão já instalada: {0}")]
    AlreadyInstalled(String),

    #[error("Manifest.json inválido: {0}")]
    InvalidManifest(String),

    #[error("Manifest não encontrado no diretório: {0}")]
    ManifestNotFound(String),

    #[error("Versão do Manifest não suportada: {0}")]
    UnsupportedManifestVersion(String),

    #[error("Arquivo de extensão inválido ou corrompido: {0}")]
    CorruptPackage(String),

    #[error("Assinatura inválida para a extensão: {0}")]
    InvalidSignature(String),

    #[error("Permissão negada: {0}")]
    PermissionDenied(String),

    #[error("Permissões insuficientes: necessário {needed}, possui {actual}")]
    InsufficientPermissions { needed: String, actual: String },

    #[error("Permissão {0} não foi concedida pelo usuário")]
    PermissionNotGranted(String),

    #[error("Erro de runtime da extensão: {0}")]
    RuntimeError(String),

    #[error("Service Worker falhou: {0}")]
    ServiceWorkerError(String),

    #[error("Erro de armazenamento: {0}")]
    StorageError(String),

    #[error("Erro de I/O: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Erro de serialização JSON: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("URL inválida: {0}")]
    InvalidUrl(String),

    #[error("Falha na atualização: {0}")]
    UpdateError(String),

    #[error("Falha ao baixar extensão: {0}")]
    Download(String),

    #[error("Extensão incompatível com esta versão do navegador: {0}")]
    Incompatible(String),

    #[error("Recurso não implementado: {0}")]
    NotImplemented(String),

    #[error("Erro interno do sistema de extensões: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, ExtensionError>;