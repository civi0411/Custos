//! Protocol Translation Layer
//!
//! Provides bidirectional conversions between canonical Custos `ModelTurnRequest`/`ModelTurnEvent`
//! and upstream provider protocols (OpenAI Chat Completions, Anthropic Messages).

pub mod anthropic;
pub mod bedrock;
pub mod gemini;
pub mod openai;

pub use anthropic::*;
pub use bedrock::*;
pub use gemini::*;
pub use openai::*;
