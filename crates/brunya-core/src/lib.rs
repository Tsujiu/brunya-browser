//! Núcleo e abstrações fundamentais do Brunya Browser.
//!
//! Fornece tipos de dados compartilhados, identificadores tipados,
//! controle de estado de navegação e traits do motor de navegação.

pub mod error;
pub mod traits;
pub mod types;

pub use error::{BrunyaError, Result};
pub use traits::*;
pub use types::*;
