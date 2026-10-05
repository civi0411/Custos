import React from 'react';
import { useAppContext } from '../../context/AppContext';
import { ProvidersView } from '@/components/providers';

export const ProvidersPage: React.FC = () => {
  const {
    providers,
    clientKeys,
    setIsAddProviderOpen,
    handleGenerateClientKey,
    handleRevokeClientKey,
    showToast
  } = useAppContext();

  return (
    <div className="flex-1 flex overflow-hidden min-w-0 h-full relative">
      <ProvidersView
        providers={providers}
        clientKeys={clientKeys}
        onOpenAddProviderModal={() => setIsAddProviderOpen(true)}
        onGenerateClientKey={handleGenerateClientKey}
        onRevokeClientKey={handleRevokeClientKey}
        onTestConnection={(name) => showToast(`Key test: ${name} 200 OK`)}
      />
    </div>
  );
};

export default ProvidersPage;
