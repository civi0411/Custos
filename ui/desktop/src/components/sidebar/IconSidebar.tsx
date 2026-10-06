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
import { MainTab } from '@/types';

interface IconSidebarProps {
  currentTab: MainTab;
  onSwitchTab: (tab: MainTab) => void;
  onOpenSettings?: () => void;
  onShowToast?: (msg: string) => void;
}

interface NavItem {
  id: MainTab;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  dividerAfter?: boolean;
}

const NAV_ITEMS: NavItem[] = [
  { id: 'studio', label: 'Chat & Code Sessions', icon: MessageSquareCode },
  { id: 'providers', label: 'API Keys & Providers', icon: KeyRound },
  { id: 'chains', label: 'OmniRoute Chains', icon: GitFork },
  { id: 'telemetry', label: 'Logs & Telemetry', icon: Activity, dividerAfter: true },
  { id: 'cache', label: 'Local KV Cache', icon: Database }
];

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

  const renderNavButton = (
    id: MainTab,
    label: string,
    Icon: React.ComponentType<{ className?: string }>,
    onClick: () => void,
    sizeClasses: string = 'w-8 sm:w-9 h-8 sm:h-9 rounded-xl',
    iconClasses: string = 'w-4 h-4'
  ) => {
    const isSelected = currentTab === id;
    return (
      <div key={id} className="has-tooltip relative flex items-center justify-center">
        <button 
          onClick={onClick} 
          className={`${sizeClasses} flex items-center justify-center transition ${
            isSelected
              ? 'bg-white text-black shadow-md shadow-white/10 border border-white font-semibold'
              : 'text-neutral-400 hover:text-white hover:bg-surface-elevated border border-transparent hover:border-surface-border'
          }`}
        >
          <Icon className={iconClasses} />
        </button>
        <span className="tooltip-label absolute left-12 sm:left-14 px-2 py-1 bg-surface-elevated border border-surface-border text-neutral-200 text-[11px] font-medium rounded-md whitespace-nowrap shadow-xl z-[9999] pointer-events-none">
          {label}
        </span>
      </div>
    );
  };

  return (
    <aside className="w-12 sm:w-14 bg-surface border-r border-surface-border flex flex-col items-center py-2.5 sm:py-3 justify-between shrink-0 z-40 select-none">
      {/* Top Navigation Group */}
      <div className="flex flex-col items-center gap-2 w-full px-1">
        {NAV_ITEMS.map((item) => (
          <React.Fragment key={item.id}>
            {renderNavButton(item.id, item.label, item.icon, () => onSwitchTab(item.id))}
            {item.dividerAfter && <div className="w-5 h-[1px] bg-surface-border my-0.5" />}
          </React.Fragment>
        ))}
      </div>

      {/* Bottom Group: Preferences & User Avatar */}
      <div className="flex flex-col items-center gap-2.5">
        {/* Settings / Preferences Button */}
        {renderNavButton(
          'settings',
          'Preferences',
          SlidersHorizontal,
          () => {
            if (onOpenSettings) onOpenSettings();
            else onSwitchTab('settings');
          },
          'w-8 h-8 rounded-lg',
          'w-3.5 h-3.5'
        )}

        {/* User Avatar */}
        <div className="has-tooltip relative" ref={avatarRef}>
          <button 
            onClick={(e) => {
              e.stopPropagation();
              setIsAvatarMenuOpen(!isAvatarMenuOpen);
            }} 
            className="w-7 sm:w-8 h-7 sm:h-8 rounded-full bg-gradient-to-tr from-brand-blue to-purple-600 p-[1.5px] focus:outline-none hover:ring-2 hover:ring-brand-blue/60 transition cursor-pointer" 
            title="Custos menu"
          >
            <div className="w-full h-full rounded-full bg-surface-elevated flex items-center justify-center text-[10px] sm:text-[11px] font-bold text-white">
              C
            </div>
          </button>
          {!isAvatarMenuOpen && (
            <span className="tooltip-label absolute left-12 sm:left-14 px-2 py-1 bg-surface-elevated border border-surface-border text-neutral-200 text-[11px] font-medium rounded-md whitespace-nowrap shadow-xl z-[9999] pointer-events-none">
              Alex Smith (alex@custos.io)
            </span>
          )}

          {isAvatarMenuOpen && (
            <div className="absolute left-14 bottom-1 w-60 bg-surface-card/95 backdrop-blur-md border border-surface-border rounded-xl shadow-2xl p-1.5 text-xs z-50 animate-in fade-in zoom-in-95 duration-100">
              <div className="px-2.5 py-2 border-b border-surface-border/40 pb-2 mb-1 bg-surface-elevated/40 rounded-lg">
                <div className="font-semibold text-white flex items-center justify-between text-xs">
                  <span>Alex Smith</span>
                  <span className="w-2 h-2 rounded-full bg-emerald-400 shadow-sm shadow-emerald-400/50"></span>
                </div>
                <div className="text-[10.5px] text-neutral-400 truncate mt-0.5 font-mono">alex@custos.io</div>
              </div>

              <div className="space-y-0.5">
                <button 
                  onClick={() => {
                    onSwitchTab('dashboard');
                    setIsAvatarMenuOpen(false);
                  }} 
                  className="w-full text-left px-2.5 py-2 rounded-lg hover:bg-surface-hover text-neutral-400 hover:text-white flex items-center gap-2.5 transition cursor-pointer"
                >
                  <Activity className="w-3.5 h-3.5 text-brand-blue shrink-0" />
                  <span className="truncate">Dashboard Overview</span>
                </button>
                
                <button 
                  onClick={() => {
                    onSwitchTab('providers');
                    setIsAvatarMenuOpen(false);
                  }} 
                  className="w-full text-left px-2.5 py-2 rounded-lg hover:bg-surface-hover text-neutral-400 hover:text-white flex items-center gap-2.5 transition cursor-pointer"
                >
                  <Key className="w-3.5 h-3.5 text-amber-400 shrink-0" />
                  <span className="truncate">API Keys & Providers</span>
                </button>
                
                <button 
                  onClick={() => {
                    if (onOpenSettings) onOpenSettings();
                    else onSwitchTab('settings');
                    setIsAvatarMenuOpen(false);
                  }} 
                  className="w-full text-left px-2.5 py-2 rounded-lg hover:bg-surface-hover text-neutral-400 hover:text-white flex items-center gap-2.5 transition cursor-pointer"
                >
                  <SettingsIcon className="w-3.5 h-3.5 text-neutral-400 shrink-0" />
                  <span className="truncate">Preferences</span>
                </button>
              </div>
              
              <div className="h-[1px] bg-surface-border/80 my-1"></div>
              
              <button 
                onClick={() => {
                  if (onShowToast) onShowToast('Signed out of session');
                  setIsAvatarMenuOpen(false);
                }} 
                className="w-full text-left px-2.5 py-2 rounded-lg hover:bg-red-500/10 text-red-400 hover:text-red-300 flex items-center gap-2.5 transition cursor-pointer font-medium"
              >
                <LogOut className="w-3.5 h-3.5 shrink-0" />
                <span>Sign out</span>
              </button>
            </div>
          )}
        </div>
      </div>
    </aside>
  );
};

export default IconSidebar;
