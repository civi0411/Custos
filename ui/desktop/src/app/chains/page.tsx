import React, { useEffect } from 'react';
import { Navigate } from 'react-router-dom';
import { useAppContext } from '@/context/AppContext';

/** Legacy route: account routing is configured in Settings; model-chain simulation is removed. */
export const ChainsPage: React.FC = () => {
  const { openSettings } = useAppContext();
  useEffect(() => { openSettings('providers'); }, [openSettings]);
  return <Navigate to="/studio" replace />;
};

export default ChainsPage;
