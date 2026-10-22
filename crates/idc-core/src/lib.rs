//! Internet Download Center indirme motoru.

pub mod engine;
pub mod ext;
pub mod filename;
pub mod http;
pub mod limiter;
pub mod media;
pub mod model;
pub mod paths;
pub mod settings;
pub mod store;
pub mod system;
pub mod tools;
pub mod util;

/// Bir indirme için en çok bağlantı sayısı.
pub const MAX_CONNECTIONS: u8 = 16;
/// Varsayılan bağlantı sayısı.
pub const DEFAULT_CONNECTIONS: u8 = 8;

pub use engine::{Engine, EngineEvent};
pub use model::*;
pub use settings::Settings;
