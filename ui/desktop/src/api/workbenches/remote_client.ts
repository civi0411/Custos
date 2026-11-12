import type {
  BrowserSession,
  BrowserTab,
  BrowserPageSnapshot,
  EmulatorDevice,
  EmulatorSession,
  EmulatorSnapshotResult,
  EmulatorScreenshotResult,
  RemoteHostNode,
  RegisterHostParams,
  FleetExecReceipt,
  HeadlessAutomationJob,
  CreateHeadlessJobParams,
  DomainPackManifest,
  PacksJobCatalogItem,
  WorkerRole,
  RunPacksJobResult,
} from '@/types/domain';
import type { DaemonTransport } from './quant_client';

export class RemoteClient {
  constructor(private transport: DaemonTransport) {}

  async listBrowserSessions(workspaceId?: string): Promise<BrowserSession[]> {
    return this.transport.request<BrowserSession[]>('v1.browser.sessions.list', {
      workspace_id: workspaceId,
    });
  }

  async createBrowserSession(name: string, workspaceId?: string): Promise<BrowserSession> {
    return this.transport.request<BrowserSession>('v1.browser.sessions.create', {
      name,
      workspace_id: workspaceId,
    });
  }

  async listBrowserTabs(sessionId: string): Promise<BrowserTab[]> {
    return this.transport.request<BrowserTab[]>('v1.browser.tabs.list', {
      session_id: sessionId,
    });
  }

  async createBrowserTab(sessionId: string, url: string): Promise<BrowserTab> {
    return this.transport.request<BrowserTab>('v1.browser.tabs.create', {
      session_id: sessionId,
      url,
    });
  }

  async navigateBrowserTab(tabId: string, url: string): Promise<BrowserTab> {
    return this.transport.request<BrowserTab>('v1.browser.tabs.navigate', {
      tab_id: tabId,
      url,
    });
  }

  async snapshotBrowserTab(
    snapshot: BrowserPageSnapshot | { tab_id: string }
  ): Promise<{ saved: boolean; tab_id: string; snapshot?: BrowserPageSnapshot }> {
    return this.transport.request<{
      saved: boolean;
      tab_id: string;
      snapshot?: BrowserPageSnapshot;
    }>('v1.browser.tabs.snapshot', snapshot);
  }

  async clickBrowserTab(tabId: string, ref: string): Promise<BrowserTab> {
    return this.transport.request<BrowserTab>('v1.browser.tabs.click', {
      tab_id: tabId,
      ref,
    });
  }

  async fillBrowserTab(tabId: string, ref: string, text: string): Promise<BrowserTab> {
    return this.transport.request<BrowserTab>('v1.browser.tabs.fill', {
      tab_id: tabId,
      ref,
      text,
    });
  }

  async screenshotBrowserTab(
    tabId: string
  ): Promise<{ path: string; tab_id?: string; image_base64?: string }> {
    return this.transport.request<{ path: string; tab_id?: string; image_base64?: string }>(
      'v1.browser.tabs.screenshot',
      {
        tab_id: tabId,
      }
    );
  }

  async closeBrowserTab(tabId: string): Promise<{ closed: boolean; tab_id: string }> {
    return this.transport.request<{ closed: boolean; tab_id: string }>('v1.browser.tabs.close', {
      tab_id: tabId,
    });
  }

  async listEmulatorDevices(): Promise<EmulatorDevice[]> {
    return this.transport.request<EmulatorDevice[]>('v1.emulator.devices.list', {});
  }

  async launchEmulatorDevice(params: {
    device_id?: string;
    session_id?: string;
    url?: string;
  }): Promise<EmulatorSession> {
    return this.transport.request<EmulatorSession>('v1.emulator.devices.launch', params);
  }

  async navigateEmulator(
    sessionId: string,
    url: string
  ): Promise<{ session_id: string; url: string; status: string }> {
    return this.transport.request<{ session_id: string; url: string; status: string }>(
      'v1.emulator.devices.navigate',
      {
        session_id: sessionId,
        url,
      }
    );
  }

  async snapshotEmulator(sessionId: string): Promise<EmulatorSnapshotResult> {
    return this.transport.request<EmulatorSnapshotResult>('v1.emulator.devices.snapshot', {
      session_id: sessionId,
    });
  }

  async captureEmulatorScreenshot(sessionId: string): Promise<EmulatorScreenshotResult> {
    return this.transport.request<EmulatorScreenshotResult>(
      'v1.emulator.devices.screenshot',
      {
        session_id: sessionId,
      }
    );
  }

  async tapEmulator(
    sessionId: string,
    ref: string
  ): Promise<{ session_id: string; ref: string; tapped: boolean }> {
    return this.transport.request<{ session_id: string; ref: string; tapped: boolean }>(
      'v1.emulator.devices.tap',
      {
        session_id: sessionId,
        ref,
      }
    );
  }

  async listRemoteHosts(): Promise<RemoteHostNode[]> {
    return this.transport.request<RemoteHostNode[]>('v1.fleet.hosts.list', {});
  }

  async registerRemoteHost(params: RegisterHostParams): Promise<RemoteHostNode> {
    return this.transport.request<RemoteHostNode>('v1.fleet.hosts.register', params);
  }

  async pingRemoteHost(
    hostId: string
  ): Promise<{ host_id: string; status: string; ping_ms?: number; error?: string }> {
    return this.transport.request<{
      host_id: string;
      status: string;
      ping_ms?: number;
      error?: string;
    }>('v1.fleet.hosts.ping', {
      host_id: hostId,
    });
  }

  async execFleet(hostId: string, command: string): Promise<FleetExecReceipt> {
    return this.transport.request<FleetExecReceipt>('v1.fleet.exec', {
      host_id: hostId,
      command,
    });
  }

  async listHeadlessJobs(status?: string): Promise<HeadlessAutomationJob[]> {
    return this.transport.request<HeadlessAutomationJob[]>('v1.automation.jobs.list', {
      status,
    });
  }

  async createHeadlessJob(params: CreateHeadlessJobParams): Promise<HeadlessAutomationJob> {
    return this.transport.request<HeadlessAutomationJob>('v1.automation.jobs.create', params);
  }

  async runHeadlessJob(jobId: string, permitId?: string): Promise<HeadlessAutomationJob> {
    return this.transport.request<HeadlessAutomationJob>('v1.automation.jobs.run', {
      job_id: jobId,
      permit_id: permitId,
      force_execute: true,
    });
  }

  async listPacksManifests(): Promise<DomainPackManifest[]> {
    return this.transport.request<DomainPackManifest[]>('v1.packs.manifests.list', {});
  }

  async listPacksJobs(params?: {
    lens?: string;
    pack_id?: string;
  }): Promise<PacksJobCatalogItem[]> {
    return this.transport.request<PacksJobCatalogItem[]>('v1.packs.jobs.list', params ?? {});
  }

  async getPacksJob(jobId: string): Promise<PacksJobCatalogItem> {
    return this.transport.request<PacksJobCatalogItem>('v1.packs.jobs.get', { job_id: jobId });
  }

  async listPacksRoles(packId?: string): Promise<WorkerRole[]> {
    return this.transport.request<WorkerRole[]>('v1.packs.roles.list', { pack_id: packId });
  }

  async runPacksJob(params: {
    jobId: string;
    sessionId?: string;
    workspaceId?: string;
    inputs?: Record<string, any>;
  }): Promise<RunPacksJobResult> {
    return this.transport.request<RunPacksJobResult>('v1.packs.jobs.run', {
      job_id: params.jobId,
      session_id: params.sessionId,
      workspace_id: params.workspaceId,
      inputs: params.inputs ?? {},
    });
  }
}
