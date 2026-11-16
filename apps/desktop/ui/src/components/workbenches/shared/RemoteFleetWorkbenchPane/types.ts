export interface RemoteFleetWorkbenchPaneProps {
  onShowToast?: (message: string) => void;
  supportedOperations: string[];
  capabilityReason?: string;
}
