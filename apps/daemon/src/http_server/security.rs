pub fn is_allowed_origin(origin: &str) -> bool {
    let lower = origin.trim().to_lowercase();
    if lower == "tauri://localhost" {
        return true;
    }

    let Some(authority) = lower
        .strip_prefix("http://")
        .or_else(|| lower.strip_prefix("https://"))
    else {
        return false;
    };

    // An Origin is scheme + authority only. Parse this narrow local profile
    // strictly instead of using a prefix match: `localhost.evil` and
    // `127.0.0.1.attacker.test` must never inherit local-client trust.
    if authority.is_empty() || authority.contains(['/', '?', '#', '@']) {
        return false;
    }

    if authority == "localhost" || authority == "127.0.0.1" || authority == "[::1]" {
        return true;
    }

    ["localhost:", "127.0.0.1:", "[::1]:"]
        .iter()
        .find_map(|prefix| authority.strip_prefix(prefix))
        .is_some_and(|port| !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit()))
}

pub fn is_authorized_ipc_client(
    origin: Option<&str>,
    auth_header: Option<&str>,
    custom_token_header: Option<&str>,
    query_token: Option<&str>,
) -> bool {
    // 1. Strict Origin check: if an Origin header is present, it MUST be an allowed local/tauri origin
    if let Some(org) = origin {
        if !is_allowed_origin(org) {
            return false;
        }
    }

    // 2. Token enforcement: if CUSTOS_IPC_TOKEN is mandated, origin cannot bypass it
    if let Ok(expected_token) = std::env::var("CUSTOS_IPC_TOKEN") {
        let expected = expected_token.trim();
        if !expected.is_empty() {
            let provided = auth_header
                .and_then(|h| {
                    h.strip_prefix("Bearer ")
                        .or_else(|| h.strip_prefix("bearer "))
                })
                .map(|s| s.trim())
                .or_else(|| custom_token_header.map(|s| s.trim()))
                .or_else(|| query_token.map(|s| s.trim()));

            return provided == Some(expected);
        }
    }

    // 3. Default loopback allowance when CUSTOS_IPC_TOKEN is not strictly enforced
    true
}
