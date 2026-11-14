import React from 'react';
import { HashRouter, Routes, Route, Navigate } from 'react-router-dom';
import { AppProvider } from './context/AppContext';
import { JobRegistryProvider } from './context/JobRegistryContext';
import { ScreenProvider } from './context/ScreenContext';
import { RootLayout } from './app/layout';
import { ErrorBoundary } from './components/common/ErrorBoundary';

// Sovereign Studio Canvas & Core Management Views
import { StudioPage } from './app/studio/page';
import { HistoryPage } from './app/history/page';
import { DashboardPage } from './app/dashboard/page';

export const App: React.FC = () => {
  return (
    <ErrorBoundary>
      <JobRegistryProvider>
        <AppProvider>
          <ScreenProvider>
            <HashRouter>
              <Routes>
                <Route path="/" element={<RootLayout />}>
                  {/* Sovereign Studio Canvas: Root entrypoint */}
                  <Route index element={<StudioPage />} />
                  <Route path="studio" element={<StudioPage />} />
                  <Route path="studio/:sessionId" element={<StudioPage />} />

                  {/* Operational Supervision & Cross-Workbench History */}
                  <Route path="dashboard" element={<DashboardPage />} />
                  <Route path="conversations" element={<HistoryPage kind="conversations" />} />
                  <Route path="tasks" element={<HistoryPage kind="tasks" />} />

                  {/* Catch-all fallback directly to Studio */}
                  <Route path="*" element={<Navigate to="/" replace />} />
                </Route>
              </Routes>
            </HashRouter>
          </ScreenProvider>
        </AppProvider>
      </JobRegistryProvider>
    </ErrorBoundary>
  );
};

export default App;
