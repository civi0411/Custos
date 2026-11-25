import { useState, useEffect, useCallback } from 'react';
import { daemonClient } from '../api/daemon_client';

export interface UseDaemonState {
  isOnline: boolean;
  isChecking: boolean;
  lastChecked: number | null;
  checkHealth: () => Promise<boolean>;
}

export function useDaemon(): UseDaemonState {
  const [isOnline, setIsOnline] = useState<boolean>(daemonClient.isConnected);
  const [isChecking, setIsChecking] = useState<boolean>(false);
  const [lastChecked, setLastChecked] = useState<number | null>(null);

  const checkHealth = useCallback(async () => {
    setIsChecking(true);
    try {
      const online = await daemonClient.checkHealth();
      setIsOnline(online);
      setLastChecked(Date.now());
      return online;
    } finally {
      setIsChecking(false);
    }
  }, []);

  useEffect(() => {
    checkHealth();
  }, [checkHealth]);

  return {
    isOnline,
    isChecking,
    lastChecked,
    checkHealth,
  };
}
