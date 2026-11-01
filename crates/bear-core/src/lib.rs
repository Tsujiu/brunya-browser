//! Núcleo e abstrações fundamentais do BEAR Browser.
//!
//! Fornece tipos de dados compartilhados, identificadores tipados,
//! controle de estado de navegação e traits do motor de navegação.

pub mod error;
pub mod types;
pub mod traits;

pub use error::{BearError, Result};
pub use types::*;
pub use traits::*;
