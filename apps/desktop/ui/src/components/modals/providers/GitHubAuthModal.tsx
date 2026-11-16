import React, { useEffect, useRef, useState } from 'react';
import { X, Loader2, RefreshCw, ExternalLink } from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { daemonClient } from '@/api/daemon_client';
import type { GitHubAuthStatusResponse } from '@/types/domain';

export interface GitHubAuthModalProps { isOpen: boolean; onClose: () => void }

export function githubTerminalText(value: string): string {
  return value.replace(/\x1b\][^\x07]*(?:\x07|\x1b\\)/g, '').replace(/\x1b\[[0-?]*[ -/]*[@-~]/g, '').replace(/\r/g, '');
}

export const GitHubAuthModal: React.FC<GitHubAuthModalProps> = ({ isOpen, onClose }) => {
  const { handleGitHubLogin, handleGitHubLogout } = useAppContext();
  const [status, setStatus] = useState<GitHubAuthStatusResponse | null>(null);
  const [busy, setBusy] = useState(false);
  const [terminalId, setTerminalId] = useState('');
  const [output, setOutput] = useState('');
  const [error, setError] = useState('');
  const epoch = useRef(0);
  const ownedTerminal = useRef('');
  const starting = useRef(false);

  const verify = async (generation: number) => {
    const result = await daemonClient.getGitHubAuthStatus();
    if (generation !== epoch.current) return;
    setStatus(result);
    if (result.authenticated && result.user?.login) {
      const connected = await handleGitHubLogin({ username: result.user.login });
      if (generation !== epoch.current) return;
      if (!connected) { setStatus(null); setError('Account verification failed. Retry the connection check.'); }
    }
    return result;
  };

  useEffect(() => {
    if (!isOpen) return;
    const generation = ++epoch.current;
    setStatus(null); setError(''); setOutput(''); setTerminalId(''); setBusy(true);
    verify(generation).catch(err => {
      if (generation === epoch.current) setError(String(err?.message || err));
    }).finally(() => { if (generation === epoch.current) setBusy(false); });
    return () => {
      ++epoch.current;
      const id = ownedTerminal.current;
      ownedTerminal.current = '';
      if (id) void daemonClient.terminateTerminal(id).catch(() => {});
    };
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen || !terminalId || ownedTerminal.current !== terminalId) return;
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    let seq = 0;
    const generation = epoch.current;
    const deadline = Date.now() + 15 * 60_000;
    const poll = async () => {
      try {
        if (Date.now() > deadline) throw new Error('GitHub login expired. Start again.');
        const chunk = await daemonClient.readTerminal(terminalId, seq, 16384);
        if (stopped || generation !== epoch.current) return;
        seq = chunk.next_seq ?? chunk.nextSeq ?? seq;
        setOutput(previous => (previous + chunk.data).slice(-32768));
        const session = await daemonClient.getTerminal(terminalId);
        if (stopped || generation !== epoch.current) return;
        if (session.status.type !== 'active') {
          const result = await verify(generation);
          if (stopped || generation !== epoch.current) return;
          if (!result?.authenticated) setError('GitHub login did not complete. Review the CLI output and retry.');
          ownedTerminal.current = ''; setTerminalId(''); setBusy(false);
          return;
        }
        timer = setTimeout(poll, 1000);
      } catch (err: any) {
        if (stopped || generation !== epoch.current) return;
        setError(String(err?.message || err)); setBusy(false); setTerminalId('');
        ownedTerminal.current = '';
        void daemonClient.terminateTerminal(terminalId).catch(() => {});
      }
    };
    void poll();
    return () => { stopped = true; clearTimeout(timer); };
  }, [isOpen, terminalId]);

  const connect = async () => {
    if (starting.current || busy) return;
    starting.current = true;
    const generation = epoch.current;
    setBusy(true); setError(''); setOutput('');
    try {
      const terminal = await daemonClient.startGitHubLogin();
      if (generation !== epoch.current) {
        await daemonClient.terminateTerminal(terminal.id);
        return;
      }
      ownedTerminal.current = terminal.id;
      setTerminalId(terminal.id);
    } catch (err: any) {
      if (generation === epoch.current) { setError(String(err?.message || err)); setBusy(false); }
    } finally { starting.current = false; }
  };

  const refresh = async () => {
    const generation = epoch.current;
    setBusy(true); setError('');
    try { await verify(generation); }
    catch (err: any) { if (generation === epoch.current) setError(String(err?.message || err)); }
    finally { if (generation === epoch.current) setBusy(false); }
  };

  if (!isOpen) return null;
  const text = githubTerminalText(output);
  const connected = status?.authenticated && status.user?.login;
  return (
    <div className="fixed inset-0 z-[200] flex items-center justify-center bg-black/50 p-6" role="dialog" aria-modal="true" aria-labelledby="github-title">
      <div className="w-full max-w-xl rounded-xl border border-border-muted bg-surface-1 text-editor-fg shadow-xl p-6 space-y-4">
        <div className="flex items-center justify-between"><h2 id="github-title" className="font-semibold text-lg">GitHub connection</h2><button onClick={onClose} aria-label="Close GitHub connection"><X size={18} /></button></div>
        <p className="text-sm text-fg-muted">Optional connection for GitHub services. Open local repositories and work with branches and worktrees without signing in. Copilot is configured separately in Connections &amp; Tools.</p>
        {connected ? <div className="space-y-3"><p>Verified GitHub account: <strong>@{status.user!.login}</strong></p><p className="text-xs text-fg-muted">GitHub CLI manages this account on your machine. This does not verify repository permissions, SSH access or commit identity.</p><button className="px-3 py-2 rounded border border-border-muted" onClick={() => { handleGitHubLogout(); onClose(); }}>Hide account in Custos</button></div> : <>
          <p className="text-sm">{status?.installed === false ? 'Install GitHub CLI (gh), then check again.' : 'Connect using GitHub CLI. Follow its prompts below, then approve the device code in your browser.'}</p>
          <button disabled={busy || status?.installed === false} onClick={connect} className="px-4 py-2 rounded bg-blue-600 text-white disabled:opacity-50">{busy ? <span className="flex items-center gap-2"><Loader2 size={15} className="animate-spin" />{terminalId ? 'Waiting for GitHub authorization' : 'Checking connection'}</span> : 'Connect GitHub'}</button>
        </>}
        {terminalId || text ? <div className="space-y-2"><pre className="max-h-60 overflow-auto whitespace-pre-wrap rounded bg-black text-gray-100 p-3 text-xs select-text" aria-live="polite">{text || 'Starting GitHub CLI…'}</pre>{terminalId && <div className="flex gap-3"><button className="px-3 py-2 rounded border border-border-muted text-sm" onClick={() => { void daemonClient.writeTerminal(terminalId, '\r').catch(err => setError(String(err?.message || err))); }}>Send Enter to CLI</button><a href="https://github.com/login/device" target="_blank" rel="noopener noreferrer" className="inline-flex items-center gap-1 text-sm text-blue-400">Open GitHub device page <ExternalLink size={13} /></a></div>}</div> : null}
        {error && <p role="alert" className="text-sm text-red-400">{error}</p>}
        {!connected && !terminalId && status?.error && <p className="text-xs text-fg-muted">{status.error}</p>}
        <div className="flex justify-between gap-3 pt-2"><button onClick={onClose} className="text-sm">{terminalId ? 'Cancel login and return to work' : 'Return to work'}</button><button disabled={busy} onClick={refresh} className="inline-flex items-center gap-1 text-sm disabled:opacity-50"><RefreshCw size={14} />Check connection</button></div>
      </div>
    </div>
  );
};
