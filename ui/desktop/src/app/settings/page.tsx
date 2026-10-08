import React, { useEffect } from 'react';
import { Navigate } from 'react-router-dom';
import { useAppContext } from '@/context/AppContext';

/**
 * SettingsPage (Route `/settings`)
 * Redirects seamlessly to `/studio` and opens the unified in-place SettingsModal.
 * Eliminates the legacy OrCa duplicate layout and prevents context abandonment.
 */
export const SettingsPage: React.FC = () => {
  const { openSettings } = useAppContext();

  useEffect(() => {
    openSettings('general');
  }, [openSettings]);

  return <Navigate to="/studio" replace />;
};

export default SettingsPage;
