import React from 'react';
import { HashRouter, Routes, Route, Navigate } from 'react-router-dom';
import { AppProvider } from './context/AppContext';
import { RootLayout } from './app/layout';

// App Router pages
import { Page as RootPage } from './app/page';
import { StudioPage } from './app/studio/page';
import { ProvidersPage } from './app/providers/page';
import { ChainsPage } from './app/chains/page';
import { TelemetryPage } from './app/telemetry/page';
import { CachePage } from './app/cache/page';
import { DashboardPage } from './app/dashboard/page';
import { DocsPage } from './app/docs/page';
import { SettingsPage } from './app/settings/page';

export const App: React.FC = () => {
  return (
    <AppProvider>
      <HashRouter>
        <Routes>
          <Route path="/" element={<RootLayout />}>
            <Route index element={<RootPage />} />
            <Route path="studio" element={<StudioPage />} />
            <Route path="studio/:sessionId" element={<StudioPage />} />
            <Route path="providers" element={<ProvidersPage />} />
            <Route path="chains" element={<ChainsPage />} />
            <Route path="telemetry" element={<TelemetryPage />} />
            <Route path="cache" element={<CachePage />} />
            <Route path="dashboard" element={<DashboardPage />} />
            <Route path="docs" element={<DocsPage />} />
            <Route path="settings" element={<SettingsPage />} />
            {/* Catch-all fallback */}
            <Route path="*" element={<Navigate to="/studio" replace />} />
          </Route>
        </Routes>
      </HashRouter>
    </AppProvider>
  );
};

export default App;
