import React from 'react';
import { useAppContext } from '@/context/AppContext';
import { ProvidersView } from '@/components/providers/ProvidersView';

/** Keep the dedicated route on the same daemon-backed provider surface as Settings. */
export const ProvidersPage: React.FC = () => {
  const {
    providers,
    clientKeys,
    setIsAddProviderOpen,
    handleGenerateClientKey,
    handleRevokeClientKey,
    handleTestConnection,
    handleDeleteProvider,
  } = useAppContext();

  return <ProvidersView
    providers={providers}
    clientKeys={clientKeys}
    onOpenAddProviderModal={() => setIsAddProviderOpen(true)}
    onGenerateClientKey={() => { void handleGenerateClientKey(); }}
    onRevokeClientKey={(id) => { void handleRevokeClientKey(id); }}
    onTestConnection={(id) => { void handleTestConnection(id); }}
    onDeleteProvider={(id) => { void handleDeleteProvider(id); }}
  />;
};

export default ProvidersPage;
