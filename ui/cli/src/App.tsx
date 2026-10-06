import { useState, useEffect } from 'react';
import { ExecutionPermit, OperationalMode, ResponsiveTier, ViewMode } from './types';
import { Header } from './components/Header';
import { TerminalView } from './components/TerminalView';
import { MascotShowcase } from './components/MascotShowcase';
import { VibeWizard } from './components/VibeWizard';
import { TaskManager } from './components/TaskManager';
import { StatusBar } from './components/StatusBar';
import { ExecutionPermitModal } from './components/ExecutionPermitModal';
import { CustosApi, subscribeToApi } from './services/custosApi';
import './App.css';

export function App() {
  const [currentMode, setCurrentMode] = useState<OperationalMode>('Custos');
  const [viewMode, setViewMode] = useState<ViewMode>('split');
  const [crtEffect, setCrtEffect] = useState<boolean>(false);
  const [termWidth, setTermWidth] = useState<number>(100);
  const [responsiveTier, setResponsiveTier] = useState<ResponsiveTier>('Standard');
  const [activePermit, setActivePermit] = useState<ExecutionPermit | null>(null);
  const [initialCliCommand, setInitialCliCommand] = useState<string | undefined>();

  useEffect(() => {
    const handleResize = () => {
      const calculatedCols = Math.max(20, Math.floor(window.innerWidth / 8.8));
      setTermWidth(calculatedCols);

      if (calculatedCols < 70) {
        setResponsiveTier('Compact');
      } else if (calculatedCols < 105) {
        setResponsiveTier('Standard');
      } else if (calculatedCols < 160) {
        setResponsiveTier('Wide');
      } else {
        setResponsiveTier('UltraWide');
      }
    };

    handleResize();
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  useEffect(() => {
    const checkPermits = () => {
      const pending = CustosApi.getPendingPermits();
      if (pending.length > 0 && !activePermit) {
        setActivePermit(pending[0]);
      } else if (pending.length === 0 && activePermit) {
        setActivePermit(null);
      }
    };

    const unsub = subscribeToApi(checkPermits);
    return unsub;
  }, [activePermit]);

  const handlePermitApprove = (permitId: string) => {
    CustosApi.resolvePermit(permitId, true);
    setActivePermit(null);
  };

  const handlePermitReject = (permitId: string) => {
    CustosApi.resolvePermit(permitId, false);
    setActivePermit(null);
  };

  const handleStartVibeFromMascot = (mode: OperationalMode) => {
    setCurrentMode(mode);
    setInitialCliCommand(`custos vibe --mode ${mode.toLowerCase()}`);
  };

  const handleSelectTaskInCli = (taskId: string) => {
    setInitialCliCommand(`custos status -i ${taskId}`);
    if (viewMode === 'visual') {
      setViewMode('split');
    }
  };

  return (
    <div className="cli-app-root">
      {crtEffect && <div className="crt-overlay" />}

      <Header
        currentMode={currentMode}
        onModeChange={setCurrentMode}
        viewMode={viewMode}
        onViewModeChange={setViewMode}
        responsiveTier={responsiveTier}
        crtEffect={crtEffect}
        onToggleCrt={() => setCrtEffect((prev) => !prev)}
        termWidth={termWidth}
      />

      <main className="cli-main-content">
        {viewMode === 'terminal' && (
          <div className="view-container-terminal">
            <TerminalView
              currentMode={currentMode}
              onModeChange={setCurrentMode}
              initialCommand={initialCliCommand}
              onClearInitialCommand={() => setInitialCliCommand(undefined)}
            />
          </div>
        )}

        {viewMode === 'visual' && (
          <div className="view-container-visual">
            <MascotShowcase
              currentMode={currentMode}
              onSelectMode={setCurrentMode}
              onStartVibe={handleStartVibeFromMascot}
            />

            <VibeWizard
              initialMode={currentMode}
              onOpenTerminalWithCommand={(cmd) => {
                setInitialCliCommand(cmd);
                setViewMode('split');
              }}
            />

            <TaskManager onSelectTaskInCli={handleSelectTaskInCli} />
          </div>
        )}

        {viewMode === 'split' && (
          <div className="view-container-split">
            <div className="split-left">
              <TerminalView
                currentMode={currentMode}
                onModeChange={setCurrentMode}
                initialCommand={initialCliCommand}
                onClearInitialCommand={() => setInitialCliCommand(undefined)}
              />
            </div>
            <div className="split-right">
              <VibeWizard
                initialMode={currentMode}
                onOpenTerminalWithCommand={(cmd) => {
                  setInitialCliCommand(cmd);
                }}
              />

              <MascotShowcase
                currentMode={currentMode}
                onSelectMode={setCurrentMode}
                onStartVibe={handleStartVibeFromMascot}
              />

              <TaskManager onSelectTaskInCli={handleSelectTaskInCli} />
            </div>
          </div>
        )}
      </main>

      {activePermit && (
        <ExecutionPermitModal
          permit={activePermit}
          onApprove={handlePermitApprove}
          onReject={handlePermitReject}
        />
      )}

      <StatusBar
        currentMode={currentMode}
        responsiveTier={responsiveTier}
        termWidth={termWidth}
      />
    </div>
  );
}

export default App;
