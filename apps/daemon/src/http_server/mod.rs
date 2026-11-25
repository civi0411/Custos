pub mod handler;
pub mod security;
pub mod server;
pub mod sse;

pub use handler::{find_header_end, handle_http_client};
pub use security::{is_allowed_origin, is_authorized_ipc_client};
pub use server::{start_http_server, MAX_BODY_SIZE, READ_TIMEOUT};
pub use sse::{handle_sse_events, write_live};

#[cfg(test)]
mod tests;
