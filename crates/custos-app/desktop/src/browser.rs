//! Manual browser presentation. Agent browser execution remains daemon-owned.
use std::collections::HashSet;
use tauri::webview::{NewWindowResponse, PageLoadEvent, WebviewBuilder};
use tauri::{Emitter, LogicalPosition, LogicalSize, Manager, Rect, Webview, WebviewUrl};

#[derive(Default)]
pub struct BrowserRegistry(pub tokio::sync::Mutex<HashSet<String>>);

#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    #[error("Browser commands require the main application webview")]
    Forbidden,
    #[error("Invalid browser request: {0}")]
    Invalid(String),
    #[error(transparent)]
    Native(#[from] tauri::Error),
}

#[derive(serde::Deserialize)]
pub struct BrowserRequest {
    id: String,
    action: String,
    instance: Option<String>,
    user_agent: Option<String>,
    contain_fullscreen: Option<bool>,
    url: Option<String>,
    x: Option<f64>,
    y: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
    zoom: Option<f64>,
    visible: Option<bool>,
}

#[derive(Clone, serde::Serialize)]
pub struct BrowserState {
    pub id: String,
    pub instance: Option<String>,
    pub url: String,
    pub loading: bool,
    pub error: Option<String>,
}

fn allowed_url(url: &tauri::Url) -> bool {
    matches!(url.scheme(), "http" | "https") || url.as_str() == "about:blank"
}

fn parse_url(raw: &str) -> Result<tauri::Url, BrowserError> {
    let url = tauri::Url::parse(raw).map_err(|e| BrowserError::Invalid(e.to_string()))?;
    if !allowed_url(&url) {
        return Err(BrowserError::Invalid(
            "Only HTTP, HTTPS and about:blank are supported".into(),
        ));
    }
    Ok(url)
}

fn emit(app: &tauri::AppHandle, state: BrowserState) {
    if let Err(error) = app.emit_to("main", "custos-browser-state", state) {
        tracing::debug!(%error, "Browser state delivery failed");
    }
}

pub fn authorize(caller: &Webview) -> Result<(), BrowserError> {
    let url = caller.url()?;
    let app_origin = url.scheme() == "tauri" && url.host_str() == Some("localhost")
        || url.scheme() == "http" && url.host_str() == Some("tauri.localhost");
    let dev_origin = cfg!(debug_assertions)
        && url.scheme() == "http"
        && url.host_str() == Some("localhost")
        && url.port() == Some(1420);
    if caller.label() == "main" && (app_origin || dev_origin) {
        Ok(())
    } else {
        Err(BrowserError::Forbidden)
    }
}

async fn execute(
    caller: Webview,
    registry: tauri::State<'_, BrowserRegistry>,
    req: BrowserRequest,
) -> Result<Option<String>, BrowserError> {
    authorize(&caller)?;
    if req.id.is_empty()
        || req.id.len() > 100
        || !req
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(BrowserError::Invalid("Invalid tab identity".into()));
    }
    // Async commands run off the main thread: add_child dispatches to that thread.
    let mut tabs = registry.0.lock().await;
    let app = caller.app_handle();
    let label = format!("browser-{}", req.id);
    tracing::debug!(tab_id = %req.id, action = %req.action, "Manual browser command");
    if req.action == "close" {
        if let Some(view) = app.get_webview(&label) {
            view.close()?;
        }
        tabs.remove(&req.id);
        return Ok(None);
    }
    if req.action == "create" && app.get_webview(&label).is_none() {
        let url = parse_url(req.url.as_deref().unwrap_or("about:blank"))?;
        let navigation_app = app.clone();
        let navigation_id = req.id.clone();
        let navigation_instance = req.instance.clone();
        let popup_app = app.clone();
        let popup_id = req.id.clone();
        let popup_instance = req.instance.clone();
        let load_app = app.clone();
        let load_id = req.id.clone();
        let load_instance = req.instance.clone();
        let builder = WebviewBuilder::new(&label, WebviewUrl::External(url));
        let builder = if req.contain_fullscreen == Some(true) {
            builder.initialization_script_for_all_frames(include_str!("browser_fullscreen.js"))
        } else {
            builder
        };
        let builder = if let Some(user_agent) = req.user_agent.as_deref() {
            if user_agent.len() > 512 || user_agent.chars().any(char::is_control) {
                return Err(BrowserError::Invalid("Invalid browser user agent".into()));
            }
            builder.user_agent(user_agent)
        } else {
            builder
        };
        let builder = builder
            .on_navigation(move |url| {
                let allowed = allowed_url(url);
                if !allowed {
                    emit(
                        &navigation_app,
                        BrowserState {
                            id: navigation_id.clone(),
                            instance: navigation_instance.clone(),
                            url: url.to_string(),
                            loading: false,
                            error: Some("Unsupported navigation scheme was blocked".into()),
                        },
                    );
                }
                allowed
            })
            .on_new_window(move |url, _| {
                emit(
                    &popup_app,
                    BrowserState {
                        id: popup_id.clone(),
                        instance: popup_instance.clone(),
                        url: url.to_string(),
                        loading: false,
                        error: Some(
                            "Popup blocked. Open its address in a Browser tab instead.".into(),
                        ),
                    },
                );
                NewWindowResponse::Deny
            })
            .on_page_load(move |_, payload| {
                emit(
                    &load_app,
                    BrowserState {
                        id: load_id.clone(),
                        instance: load_instance.clone(),
                        url: payload.url().to_string(),
                        loading: payload.event() == PageLoadEvent::Started,
                        error: None,
                    },
                );
            });
        let view = caller.window().add_child(
            builder,
            LogicalPosition::new(-10000.0, -10000.0),
            LogicalSize::new(1.0, 1.0),
        )?;
        view.hide()?;
        tabs.insert(req.id.clone());
    }
    let view = app
        .get_webview(&label)
        .ok_or_else(|| BrowserError::Invalid("Browser tab is not open".into()))?;
    match req.action.as_str() {
        "create" | "state" => {}
        "navigate" => view.navigate(parse_url(
            req.url
                .as_deref()
                .ok_or_else(|| BrowserError::Invalid("Missing URL".into()))?,
        )?)?,
        "back" => view.eval("window.history.back()")?,
        "forward" => view.eval("window.history.forward()")?,
        "reload" => view.reload()?,
        "layout" => {
            let (x, y, width, height) = (
                req.x.unwrap_or(0.0),
                req.y.unwrap_or(0.0),
                req.width.unwrap_or(0.0),
                req.height.unwrap_or(0.0),
            );
            if ![x, y, width, height].iter().all(|v| v.is_finite()) || width < 0.0 || height < 0.0 {
                return Err(BrowserError::Invalid("Invalid bounds".into()));
            }
            if req.visible == Some(true) && width > 0.0 && height > 0.0 {
                view.set_bounds(Rect {
                    position: LogicalPosition::new(x, y).into(),
                    size: LogicalSize::new(width, height).into(),
                })?;
                view.show()?;
            } else {
                view.hide()?;
            }
        }
        "zoom" => {
            let zoom = req.zoom.unwrap_or(1.0);
            if !zoom.is_finite() || !(0.1..=4.0).contains(&zoom) {
                return Err(BrowserError::Invalid("Invalid zoom".into()));
            }
            view.set_zoom(zoom)?;
        }
        _ => return Err(BrowserError::Invalid("Unknown browser action".into())),
    }
    Ok(Some(view.url()?.to_string()))
}

#[tauri::command]
pub async fn browser_command(
    caller: Webview,
    registry: tauri::State<'_, BrowserRegistry>,
    request: BrowserRequest,
) -> Result<Option<String>, String> {
    execute(caller, registry, request)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn navigation_is_limited_to_manual_web_content() {
        for url in [
            "https://example.com",
            "http://localhost:5173",
            "about:blank",
        ] {
            assert!(parse_url(url).is_ok());
        }
        for url in [
            "file:///secret",
            "javascript:alert(1)",
            "tauri://localhost",
            "about:config",
            "data:text/html,hi",
            "not a url",
        ] {
            assert!(parse_url(url).is_err());
        }
    }
}
