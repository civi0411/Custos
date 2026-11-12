pub const METHOD_TASKS_CREATE: &str = "v1.tasks.create";
pub const METHOD_TASKS_GET: &str = "v1.tasks.get";
pub const METHOD_TASKS_LIST: &str = "v1.tasks.list";
pub const METHOD_TASKS_SPANS: &str = "v1.tasks.spans";
pub const METHOD_TASKS_CANCEL: &str = "v1.tasks.cancel";
pub const METHOD_TASKS_ADVANCE: &str = "v1.tasks.advance";
pub const METHOD_TASKS_COMPLETE: &str = "v1.tasks.complete";
pub const METHOD_SESSIONS_CREATE: &str = "v1.sessions.create";
pub const METHOD_SESSIONS_GET: &str = "v1.sessions.get";
pub const METHOD_SESSIONS_LIST: &str = "v1.sessions.list";
pub const METHOD_SESSIONS_JOURNAL: &str = "v1.sessions.journal";
pub const METHOD_SESSIONS_OPEN_IN_LENS: &str = "v1.sessions.open_in_lens";
pub const METHOD_SESSIONS_MESSAGE: &str = "v1.sessions.message";
pub const METHOD_SESSIONS_PROMOTE: &str = "v1.sessions.promote";
pub const METHOD_SESSIONS_DELETE: &str = "v1.sessions.delete";
pub const METHOD_BRIDGE_ATTACH: &str = "v1.bridge.attach";
pub const METHOD_BRIDGE_STEER: &str = "v1.bridge.steer";
pub const METHOD_OI_EXPLAIN: &str = "v1.oi.explain";
pub const METHOD_WORKFLOW_START_RUN: &str = "v1.workflow.start_run";
pub const METHOD_WORKFLOW_CANCEL_RUN: &str = "v1.workflow.cancel_run";
pub const METHOD_WORKSPACES_CREATE: &str = "v1.workspaces.create";
pub const METHOD_WORKSPACES_GET: &str = "v1.workspaces.get";
pub const METHOD_WORKSPACES_LIST: &str = "v1.workspaces.list";
pub const METHOD_WORKSPACES_ARCHIVE: &str = "v1.workspaces.archive";
pub const METHOD_WORKSPACES_INSPECT_DIRTY: &str = "v1.workspaces.inspect_dirty";
pub const METHOD_WORKSPACES_RECOVER: &str = "v1.workspaces.recover";
pub const METHOD_WORKSPACES_DETECT: &str = "v1.workspaces.detect";
pub const METHOD_WORKSPACES_PICK_FOLDER: &str = "v1.workspaces.pick_folder";
pub const METHOD_WORKSPACES_CANDIDATES_SPAWN: &str = "v1.workspaces.candidates.spawn";
pub const METHOD_WORKSPACES_CANDIDATES_COMPARE: &str = "v1.workspaces.candidates.compare";
pub const METHOD_WORKSPACES_CANDIDATES_RACE: &str = "v1.workspaces.candidates.race";
pub const METHOD_WORKSPACES_INTEGRATE: &str = "v1.workspaces.integrate";
pub const METHOD_SETTINGS_GET: &str = "v1.settings.get";
pub const METHOD_SETTINGS_SET: &str = "v1.settings.set";

pub const METHOD_QUANT_BACKTEST_SUBMIT: &str = "v1.quant.backtest.submit";
pub const METHOD_QUANT_TEARSHEET_GET: &str = "v1.quant.tearsheet.get";
pub const METHOD_QUANT_ORDERS_CREATE: &str = "v1.quant.orders.create";
pub const METHOD_QUANT_ORDERS_LIST: &str = "v1.quant.orders.list";
pub const METHOD_QUANT_ORDERS_APPROVE: &str = "v1.quant.orders.approve";
pub const METHOD_QUANT_FACTORS_DISCOVER: &str = "v1.quant.factors.discover";
pub const METHOD_QUANT_MARKET_BARS_INGEST: &str = "v1.quant.market_bars.ingest";
pub const METHOD_QUANT_MARKET_BARS_GET: &str = "v1.quant.market_bars.get";
pub const METHOD_PIPELINE_STATUS: &str = "v1.pipeline.status";
pub const METHOD_PIPELINE_STRUCTURE_DETECT: &str = "v1.pipeline.structure.detect";
pub const METHOD_PIPELINE_RUN: &str = "v1.pipeline.run";
pub const METHOD_QUANT_KILL_SWITCH: &str = "v1.quant.kill_switch";
pub const METHOD_QUANT_GRID_EVALUATE: &str = "v1.quant.grid.evaluate";
pub const METHOD_QUANT_INDICATORS_COMPUTE: &str = "v1.quant.indicators.compute";
pub const METHOD_QUANT_GATE_EVALUATE: &str = "v1.quant.gate.evaluate";
pub const METHOD_QUANT_REVIEW_COMPUTE: &str = "v1.quant.review.compute";
pub const METHOD_QUANT_ORDERBOOK_GET: &str = "v1.quant.orderbook.get";
pub const METHOD_QUANT_PBO_COMPUTE: &str = "v1.quant.pbo.compute";

pub const METHOD_WORKSPACE_FILES_TREE: &str = "v1.workspace.files.tree";
pub const METHOD_WORKSPACE_FILES_READ: &str = "v1.workspace.files.read";
pub const METHOD_WORKSPACE_FILES_WRITE: &str = "v1.workspace.files.write";
pub const METHOD_WORKSPACE_DIFF: &str = "v1.workspace.diff";
pub const METHOD_WORKSPACE_FILE_DIFF: &str = "v1.workspace.diff.file";
pub const METHOD_WORKSPACE_GIT_STAGE: &str = "v1.workspace.git.stage";
pub const METHOD_WORKSPACE_GIT_UNSTAGE: &str = "v1.workspace.git.unstage";
pub const METHOD_WORKSPACE_GIT_DISCARD: &str = "v1.workspace.git.discard";
pub const METHOD_WORKSPACE_GIT_COMMIT: &str = "v1.workspace.git.commit";

pub const METHOD_RESEARCH_SOURCES_LIST: &str = "v1.research.sources.list";
pub const METHOD_RESEARCH_SOURCES_SAVE: &str = "v1.research.sources.save";
pub const METHOD_RESEARCH_ANCHORS_SAVE: &str = "v1.research.anchors.save";
pub const METHOD_RESEARCH_ANCHORS_LIST: &str = "v1.research.anchors.list";
pub const METHOD_RESEARCH_CLAIMS_LIST: &str = "v1.research.claims.list";
pub const METHOD_RESEARCH_CLAIMS_SAVE: &str = "v1.research.claims.save";
pub const METHOD_RESEARCH_RUNS_LIST: &str = "v1.research.runs.list";
pub const METHOD_RESEARCH_RUNS_SAVE: &str = "v1.research.runs.save";
pub const METHOD_RESEARCH_LINEAGE_LIST: &str = "v1.research.lineage.list";
pub const METHOD_RESEARCH_HANDOFF_CODING: &str = "v1.research.handoff_coding";
pub const METHOD_CAPABILITIES_LIST: &str = "v1.capabilities.list";
pub const METHOD_CAPABILITIES_GET: &str = "v1.capabilities.get";
pub const METHOD_TERMINAL_SPAWN: &str = "v1.terminal.spawn";
pub const METHOD_TERMINAL_WRITE: &str = "v1.terminal.write";
pub const METHOD_TERMINAL_RESIZE: &str = "v1.terminal.resize";
pub const METHOD_TERMINAL_READ: &str = "v1.terminal.read";
pub const METHOD_TERMINAL_TERMINATE: &str = "v1.terminal.terminate";
pub const METHOD_TERMINAL_LIST: &str = "v1.terminal.list";
pub const METHOD_TERMINAL_GET: &str = "v1.terminal.get";

pub const METHOD_HARNESS_LIST: &str = "v1.harness.list";
pub const METHOD_HARNESS_GET: &str = "v1.harness.get";
pub const METHOD_HARNESS_RUN_NATIVE: &str = "v1.harness.run_native";
pub const METHOD_HARNESS_CANCEL: &str = "v1.harness.cancel";
pub const METHOD_HARNESS_STEER: &str = "v1.harness.steer";

pub const METHOD_ARTIFACTS_LIST: &str = "v1.artifacts.list";
pub const METHOD_ARTIFACTS_GET: &str = "v1.artifacts.get";
pub const METHOD_ARTIFACTS_RECORD_LINEAGE: &str = "v1.artifacts.record_lineage";
pub const METHOD_ARTIFACTS_LINEAGE_GRAPH: &str = "v1.artifacts.lineage_graph";
pub const METHOD_NOTES_LIST: &str = "v1.notes.list";
pub const METHOD_NOTES_GET: &str = "v1.notes.get";
pub const METHOD_NOTES_SAVE: &str = "v1.notes.save";
pub const METHOD_NOTES_HISTORY: &str = "v1.notes.history";

pub const METHOD_NOTEBOOK_CELLS_LIST: &str = "v1.notebook.cells.list";
pub const METHOD_NOTEBOOK_CELLS_SAVE: &str = "v1.notebook.cells.save";
pub const METHOD_NOTEBOOK_EXECUTE: &str = "v1.notebook.execute";
pub const METHOD_NOTEBOOK_INTERRUPT: &str = "v1.notebook.interrupt";
pub const METHOD_NOTEBOOK_RESET: &str = "v1.notebook.reset";
pub const METHOD_NOTEBOOK_STATUS: &str = "v1.notebook.status";

pub const METHOD_REVIEWS_LIST: &str = "v1.reviews.list";
pub const METHOD_REVIEWS_GET: &str = "v1.reviews.get";
pub const METHOD_REVIEWS_RECORD: &str = "v1.reviews.record";
pub const METHOD_REVIEWS_MARK_STALE: &str = "v1.reviews.mark_stale";

pub const METHOD_SYNTHESIS_PROPOSALS_LIST: &str = "v1.synthesis.proposals.list";
pub const METHOD_SYNTHESIS_PROPOSALS_GET: &str = "v1.synthesis.proposals.get";
pub const METHOD_SYNTHESIS_PROPOSALS_SAVE: &str = "v1.synthesis.proposals.save";
pub const METHOD_SYNTHESIS_HANDOFF_EXECUTE: &str = "v1.synthesis.handoff.execute";

pub const METHOD_BROWSER_SESSIONS_LIST: &str = "v1.browser.sessions.list";
pub const METHOD_BROWSER_SESSIONS_CREATE: &str = "v1.browser.sessions.create";
pub const METHOD_BROWSER_TABS_LIST: &str = "v1.browser.tabs.list";
pub const METHOD_BROWSER_TABS_CREATE: &str = "v1.browser.tabs.create";
pub const METHOD_BROWSER_TABS_NAVIGATE: &str = "v1.browser.tabs.navigate";
pub const METHOD_BROWSER_TABS_SNAPSHOT: &str = "v1.browser.tabs.snapshot";
pub const METHOD_BROWSER_TABS_CLICK: &str = "v1.browser.tabs.click";
pub const METHOD_BROWSER_TABS_FILL: &str = "v1.browser.tabs.fill";
pub const METHOD_BROWSER_TABS_SCREENSHOT: &str = "v1.browser.tabs.screenshot";
pub const METHOD_BROWSER_TABS_CLOSE: &str = "v1.browser.tabs.close";

pub const METHOD_EMULATOR_DEVICES_LIST: &str = "v1.emulator.devices.list";
pub const METHOD_EMULATOR_LAUNCH: &str = "v1.emulator.devices.launch";
pub const METHOD_EMULATOR_NAVIGATE: &str = "v1.emulator.devices.navigate";
pub const METHOD_EMULATOR_SNAPSHOT: &str = "v1.emulator.devices.snapshot";
pub const METHOD_EMULATOR_SCREENSHOT: &str = "v1.emulator.devices.screenshot";
pub const METHOD_EMULATOR_TAP: &str = "v1.emulator.devices.tap";

pub const METHOD_FLEET_HOSTS_LIST: &str = "v1.fleet.hosts.list";
pub const METHOD_FLEET_HOSTS_REGISTER: &str = "v1.fleet.hosts.register";
pub const METHOD_FLEET_HOSTS_PING: &str = "v1.fleet.hosts.ping";
pub const METHOD_FLEET_EXEC: &str = "v1.fleet.exec";

pub const METHOD_AUTOMATION_JOBS_LIST: &str = "v1.automation.jobs.list";
pub const METHOD_AUTOMATION_JOBS_CREATE: &str = "v1.automation.jobs.create";
pub const METHOD_AUTOMATION_JOBS_RUN: &str = "v1.automation.jobs.run";

pub const METHOD_COMPUTE_TARGETS_LIST: &str = "v1.compute.targets.list";
pub const METHOD_EXPERIMENT_PREFLIGHT: &str = "v1.experiment.preflight";
pub const METHOD_EXPERIMENT_SUBMIT: &str = "v1.experiment.submit";
pub const METHOD_EXPERIMENT_OBSERVE: &str = "v1.experiment.observe";
pub const METHOD_EXPERIMENT_STOP: &str = "v1.experiment.stop";
pub const METHOD_EXPERIMENT_RELEASE: &str = "v1.experiment.release";
pub const METHOD_EXPERIMENT_COLLECT: &str = "v1.experiment.collect";

pub const METHOD_PROVIDERS_LIST: &str = "v1.providers.list";
pub const METHOD_PROVIDERS_GET: &str = "v1.providers.get";
pub const METHOD_PROVIDERS_SAVE: &str = "v1.providers.save";
pub const METHOD_PROVIDERS_DELETE: &str = "v1.providers.delete";
pub const METHOD_MODELS_PROBE: &str = "v1.models.probe";
pub const METHOD_MODELS_CATALOG: &str = "v1.models.catalog";
pub const METHOD_PROVIDERS_USAGE: &str = "v1.providers.usage";
pub const METHOD_MODELS_PRICING: &str = "v1.models.pricing";

pub const METHOD_OAUTH_AUTHORIZE: &str = "v1.oauth.authorize";
pub const METHOD_OAUTH_EXCHANGE: &str = "v1.oauth.exchange";
pub const METHOD_OAUTH_REFRESH: &str = "v1.oauth.refresh";
pub const METHOD_OAUTH_GET: &str = "v1.oauth.get";
pub const METHOD_OAUTH_DELETE: &str = "v1.oauth.delete";
pub const METHOD_OAUTH_CANCEL: &str = "v1.oauth.cancel";
pub const METHOD_OAUTH_STATUS: &str = "v1.oauth.status";

pub const METHOD_GITHUB_ACCOUNTS: &str = "v1.github.accounts";
pub const METHOD_GIT_REPOSITORY: &str = "v1.workspace.git.repository";
pub const METHOD_GIT_SWITCH: &str = "v1.workspace.git.switch";
pub const METHOD_GIT_FETCH: &str = "v1.workspace.git.fetch";
pub const METHOD_GIT_ACCOUNT: &str = "v1.workspace.git.account";
pub const METHOD_GIT_GITHUB: &str = "v1.workspace.git.github";
pub const METHOD_GITHUB_LOGIN: &str = "v1.github.login";
pub const METHOD_GITHUB_AUTH_STATUS: &str = "v1.github.auth_status";
pub const METHOD_GITHUB_VIEWER: &str = "v1.github.viewer";

pub const METHOD_PACKS_MANIFESTS_LIST: &str = "v1.packs.manifests.list";
pub const METHOD_PACKS_JOBS_LIST: &str = "v1.packs.jobs.list";
pub const METHOD_PACKS_JOBS_GET: &str = "v1.packs.jobs.get";
pub const METHOD_PACKS_ROLES_LIST: &str = "v1.packs.roles.list";
pub const METHOD_PACKS_JOBS_RUN: &str = "v1.packs.jobs.run";

pub const METHOD_ASSISTANT_DRAFTS_LIST: &str = "v1.assistant.drafts.list";
pub const METHOD_ASSISTANT_DRAFTS_GET: &str = "v1.assistant.drafts.get";
pub const METHOD_ASSISTANT_DRAFTS_SAVE: &str = "v1.assistant.drafts.save";
pub const METHOD_ASSISTANT_DRAFTS_DELETE: &str = "v1.assistant.drafts.delete";
pub const METHOD_ASSISTANT_OUTBOX_LIST: &str = "v1.assistant.outbox.list";
pub const METHOD_ASSISTANT_OUTBOX_CREATE: &str = "v1.assistant.outbox.create";
pub const METHOD_ASSISTANT_OUTBOX_APPROVE: &str = "v1.assistant.outbox.approve";
pub const METHOD_ASSISTANT_OUTBOX_DISPATCH: &str = "v1.assistant.outbox.dispatch";

pub const METHOD_WORKSPACES_AGENT_STEER: &str = "v1.workspaces.agent_steer";
pub const METHOD_RESOURCES_CAPABILITIES: &str = "v1.resources.capabilities";
