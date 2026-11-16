//! Incremental provider framing. Text is decoded only after complete UTF-8 frames.

pub mod legacy_decoder;
pub mod stream_observer;
pub mod turn_decoder;
pub mod types;

pub use legacy_decoder::unary_completion;
#[cfg(test)]
pub(crate) use legacy_decoder::Decoder;
pub use stream_observer::{stream_response_observed, stream_turn_response_observed};
#[cfg(test)]
pub use stream_observer::{stream_response, stream_response_with_idle_timeout};
pub use turn_decoder::ModelTurnStreamDecoder;
pub use types::Wire;


#[cfg(test)]
mod tests;
