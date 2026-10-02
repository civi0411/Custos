import React, { useState, useRef, useEffect } from 'react';
import { 
  MessageSquareCode, 
  KeyRound, 
  GitFork, 
  Activity, 
  Database, 
  SlidersHorizontal, 
  Key, 
  Settings as SettingsIcon, 
  LogOut 
} from 'lucide-react';
import { MainTab } from '../types';

interface IconSidebarProps {
  currentTab: MainTab;
  onSwitchTab: (tab: MainTab) => void;
  onOpenSettings: () => void;
  onShowToast: (msg: string) => void;
}

export const IconSidebar: React.FC<IconSidebarProps> = ({
  currentTab,
  onSwitchTab,
  onOpenSettings,
  onShowToast
}) => {
  const [isAvatarMenuOpen, setIsAvatarMenuOpen] = useState(false);
  const avatarRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (avatarRef.current && !avatarRef.current.contains(e.target as Node)) {
        setIsAvatarMenuOpen(false);
      }
    };
    document.addEventListener('click', handleClickOutside);
    return () => document.removeEventListener('click', handleClickOutside);
  }, []);

  return (
    <aside className="w-12 sm:w-14 bg-black border-r border-surface-border flex flex-col items-center py-2.5 sm:py-3 justify-between shrink-0 z-20">
      {/* Top Navigation Group */}
      <div className="flex flex-col items-center gap-2 w-full px-1">
        {/* Tab 1: Chat & Sessions Studio */}
        <div className="has-tooltip relative flex items-center justify-center">
          <button 
            onClick={() => onSwitchTab('studio')} 
            className={`w-8 sm:w-9 h-8 sm:h-9 rounded-xl flex items-center justify-center transition ${
              currentTab === 'studio'
                ? 'bg-brand-blue text-white shadow-lg shadow-brand-blueGlow border border-blue-400/30'
                : 'text-neutral-400 hover:text-white hover:bg-surface-elevated border border-transparent hover:border-surface-border'
            }`}
          >
            <MessageSquareCode className="w-4 h-4" />
          </button>
          <span className="tooltip-label absolute left-12 px-2 py-1 bg-surface-elevated border border-surface-border text-neutral-200 text-[11px] font-medium rounded-md whitespace-nowrap shadow-xl z-50 pointer-events-none">
            Chat & Code Sessions
          </span>
        </div>

        {/* Tab 2: API Keys & Providers */}
        <div className="has-tooltip relative flex items-center justify-center">
          <button 
            onClick={() => onSwitchTab('providers')} 
            className={`w-8 sm:w-9 h-8 sm:h-9 rounded-xl flex items-center justify-center transition ${
              currentTab === 'providers'
                ? 'bg-brand-blue text-white shadow-lg shadow-brand-blueGlow border border-blue-400/30'
                : 'text-neutral-400 hover:text-white hover:bg-surface-elevated border border-transparent hover:border-surface-border'
            }`}
          >
            <KeyRound className="w-4 h-4" />
          </button>
          <span className="tooltip-label absolute left-12 px-2 py-1 bg-surface-elevated border border-surface-border text-neutral-200 text-[11px] font-medium rounded-md whitespace-nowrap shadow-xl z-50 pointer-events-none">
            API Keys & Providers
          </span>
        </div>

        {/* Tab 3: Model Evaluation & Routing */}
        <div className="has-tooltip relative flex items-center justify-center">
          <button 
            onClick={onOpenSettings} 
            className="w-8 sm:w-9 h-8 sm:h-9 rounded-xl flex items-center justify-center text-neutral-400 hover:text-white hover:bg-surface-elevated transition border border-transparent hover:border-surface-border"
          >
            <GitFork className="w-4 h-4" />
          </button>
          <span className="tooltip-label absolute left-12 px-2 py-1 bg-surface-elevated border border-surface-border text-neutral-200 text-[11px] font-medium rounded-md whitespace-nowrap shadow-xl z-50 pointer-events-none">
            OmniRoute Chains
          </span>
        </div>

        {/* Tab 4: Logs & Traces */}
        <div className="has-tooltip relative flex items-center justify-center">
          <button 
            onClick={() => onShowToast('Viewing live trace telemetry (18ms)')} 
            className="w-8 sm:w-9 h-8 sm:h-9 rounded-xl flex items-center justify-center text-neutral-400 hover:text-white hover:bg-surface-elevated transition border border-transparent hover:border-surface-border"
          >
            <Activity className="w-4 h-4" />
          </button>
          <span className="tooltip-label absolute left-12 px-2 py-1 bg-surface-elevated border border-surface-border text-neutral-200 text-[11px] font-medium rounded-md whitespace-nowrap shadow-xl z-50 pointer-events-none">
            Logs & Telemetry
          </span>
        </div>

        <div className="w-5 h-[1px] bg-surface-border my-0.5"></div>

        {/* Tab 5: Storage & SQLite Cache */}
        <div className="has-tooltip relative flex items-center justify-center">
          <button 
            onClick={() => onShowToast('Local SQLite KV cache active (420 entries)')} 
            className="w-8 sm:w-9 h-8 sm:h-9 rounded-xl flex items-center justify-center text-neutral-400 hover:text-white hover:bg-surface-elevated transition border border-transparent hover:border-surface-border"
          >
            <Database className="w-4 h-4" />
          </button>
          <span className="tooltip-label absolute left-12 px-2 py-1 bg-surface-elevated border border-surface-border text-neutral-200 text-[11px] font-medium rounded-md whitespace-nowrap shadow-xl z-50 pointer-events-none">
            Local KV Cache
          </span>
        </div>
      </div>

      {/* Bottom Group: Preferences & User Avatar */}
      <div className="flex flex-col items-center gap-2.5">
        {/* Settings Button */}
        <div className="has-tooltip relative flex items-center justify-center">
          <button 
            onClick={onOpenSettings} 
            className="w-8 h-8 rounded-lg flex items-center justify-center text-neutral-500 hover:text-neutral-200 hover:bg-surface-elevated transition"
          >
            <SlidersHorizontal className="w-3.5 h-3.5" />
          </button>
          <span className="tooltip-label absolute left-12 px-2 py-1 bg-surface-elevated border border-surface-border text-neutral-200 text-[11px] font-medium rounded-md whitespace-nowrap shadow-xl z-50 pointer-events-none">
            Preferences
          </span>
        </div>

        {/* User Avatar */}
        <div className="relative" ref={avatarRef}>
          <button 
            onClick={(e) => {
              e.stopPropagation();
              setIsAvatarMenuOpen(!isAvatarMenuOpen);
            }} 
            className="w-7 sm:w-8 h-7 sm:h-8 rounded-full bg-gradient-to-tr from-brand-blue to-purple-600 p-[1.5px] focus:outline-none hover:ring-2 hover:ring-brand-blue/60 transition cursor-pointer" 
            title="Alex Smith (alex@custos.io)"
          >
            <div className="w-full h-full rounded-full bg-neutral-900 flex items-center justify-center text-[10px] sm:text-[11px] font-bold text-white">
              AS
            </div>
          </button>

          {isAvatarMenuOpen && (
            <div className="absolute left-12 bottom-0 w-56 bg-surface-card border border-surface-border rounded-xl shadow-2xl p-1.5 text-xs z-50">
              <div className="px-3 py-2 border-b border-surface-border mb-1">
                <div className="font-semibold text-white flex items-center gap-1.5">
                  <span>Alex Smith</span>
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
                </div>
                <div className="text-[11px] text-neutral-500 truncate">alex@custos.io</div>
              </div>
              
              <button 
                onClick={() => {
                  onSwitchTab('providers');
                  setIsAvatarMenuOpen(false);
                }} 
                className="w-full text-left px-2.5 py-1.5 rounded-lg hover:bg-surface-hover text-neutral-300 flex items-center gap-2"
              >
                <Key className="w-3.5 h-3.5" />
                <span>API Keys & Providers</span>
              </button>
              
              <button 
                onClick={() => {
                  onOpenSettings();
                  setIsAvatarMenuOpen(false);
                }} 
                className="w-full text-left px-2.5 py-1.5 rounded-lg hover:bg-surface-hover text-neutral-300 flex items-center gap-2"
              >
                <SettingsIcon className="w-3.5 h-3.5" />
                <span>Preferences</span>
              </button>
              
              <div className="h-[1px] bg-surface-border my-1"></div>
              
              <button 
                onClick={() => {
                  onShowToast('Signed out of session');
                  setIsAvatarMenuOpen(false);
                }} 
                className="w-full text-left px-2.5 py-1.5 rounded-lg hover:bg-red-500/10 text-red-400 flex items-center gap-2"
              >
                <LogOut className="w-3.5 h-3.5" />
                <span>Sign out</span>
              </button>
            </div>
          )}
        </div>
      </div>
    </aside>
  );
};
