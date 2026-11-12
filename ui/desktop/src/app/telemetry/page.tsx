import React, { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';

/**
 * TelemetryPage redirects to the unified Supervision Console on `/dashboard`.
 */
export const TelemetryPage: React.FC = () => {
  const navigate = useNavigate();

  useEffect(() => {
    navigate('/dashboard', { replace: true });
  }, [navigate]);

  return null;
};

export default TelemetryPage;
