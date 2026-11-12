//! Cloud Model Providers, HTTP Transport, Wire Codecs, and Realtime Clients.

#[cfg(all(
    feature = "live-websocket",
    not(any(feature = "rustls-tls", feature = "native-tls"))
))]
compile_error!("feature `live-websocket` requires either `rustls-tls` or `native-tls`");

pub mod anthropic;
pub mod azure_foundry;
pub mod codecs;
pub mod databricks;
pub mod declarative;
pub mod fake;
pub mod google;
pub mod google_codeassist;
pub use google_codeassist as antigravity;
pub mod ollama;
pub mod openai;
pub mod openrouter;
pub mod realtime;
pub mod snowflake;
pub mod transport;

pub use declarative::declarative_providers::*;
