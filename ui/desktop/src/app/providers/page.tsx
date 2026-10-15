import React, { useEffect } from 'react';
import { Navigate } from 'react-router-dom';
import { useAppContext } from '@/context/AppContext';

/**
 * ProvidersPage (Legacy route: `/providers`)
 * Redirects seamlessly to `/studio` and opens the unified in-place SettingsModal on the 'providers' tab.
 * Eliminates the duplicate standalone ProvidersView page and consolidates all provider/model management.
 */
export const ProvidersPage: React.FC = () => {
  const { openSettings } = useAppContext();

  useEffect(() => {
    openSettings('providers');
  }, [openSettings]);

  return <Navigate to="/studio" replace />;
};

export default ProvidersPage;

