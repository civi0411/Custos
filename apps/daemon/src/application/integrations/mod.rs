//! Integrations application use cases organized by domain submodules:
//! - browser: headless browser sessions, tabs, navigation, snapshot, click, fill
//! - emulator: android/mobile device emulator inspect, launch, tap, snapshot
//! - fleet: remote host registration, ping, remote command execution
//! - automation: headless automation jobs lifecycle and execution

pub mod automation;
pub mod browser;
pub mod emulator;
pub mod fleet;

use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_integrations(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_BROWSER_SESSIONS_LIST
            | METHOD_BROWSER_SESSIONS_CREATE
            | METHOD_BROWSER_TABS_LIST
            | METHOD_BROWSER_TABS_CREATE
            | METHOD_BROWSER_TABS_NAVIGATE
            | METHOD_BROWSER_TABS_SNAPSHOT
            | METHOD_BROWSER_TABS_CLICK
            | METHOD_BROWSER_TABS_FILL
            | METHOD_BROWSER_TABS_SCREENSHOT
            | METHOD_BROWSER_TABS_CLOSE => self.handle_integrations_browser(req).await,

            METHOD_EMULATOR_DEVICES_LIST
            | METHOD_EMULATOR_LAUNCH
            | METHOD_EMULATOR_NAVIGATE
            | METHOD_EMULATOR_SNAPSHOT
            | METHOD_EMULATOR_SCREENSHOT
            | METHOD_EMULATOR_TAP => self.handle_integrations_emulator(req).await,

            METHOD_FLEET_HOSTS_LIST
            | METHOD_FLEET_HOSTS_REGISTER
            | METHOD_FLEET_HOSTS_PING
            | METHOD_FLEET_EXEC => self.handle_integrations_fleet(req).await,

            METHOD_AUTOMATION_JOBS_LIST
            | METHOD_AUTOMATION_JOBS_CREATE
            | METHOD_AUTOMATION_JOBS_RUN => self.handle_integrations_automation(req).await,

            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
