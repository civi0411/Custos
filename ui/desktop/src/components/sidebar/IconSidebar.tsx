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
} from 'lucide-react';
import { MainTab } from '@/types';

interface IconSidebarProps {
  currentTab: MainTab;
  onSwitchTab: (tab: MainTab) => void;
  onOpenSettings?: () => void;
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
  onOpenSettings
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
        <span className="tooltip-label absolute left-12 px-2 py-1 bg-surface-elevated border border-surface-border text-neutral-200 text-[11px] font-medium rounded-md whitespace-nowrap shadow-xl z-50 pointer-events-none">
          {label}
        </span>
      </div>
    );
  };

  return (
    <aside className="w-12 sm:w-14 bg-surface border-r border-surface-border flex flex-col items-center py-2.5 sm:py-3 justify-between shrink-0 z-20 select-none">
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
        <div className="relative" ref={avatarRef}>
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

          {isAvatarMenuOpen && (
            <div className="absolute left-12 bottom-0 w-56 bg-surface-card border border-surface-border rounded-xl shadow-2xl p-1.5 text-xs z-50">
              <div className="px-3 py-2 border-b border-surface-border mb-1">
                <div className="font-semibold text-white flex items-center gap-1.5">
                  <span>Custos</span>
                </div>
                <div className="text-[11px] text-neutral-500 truncate">Workspace controls</div>
              </div>

              <button 
                onClick={() => {
                  onSwitchTab('dashboard');
                  setIsAvatarMenuOpen(false);
                }} 
                className="w-full text-left px-2.5 py-1.5 rounded-lg hover:bg-surface-hover text-neutral-300 flex items-center gap-2"
              >
                <Activity className="w-3.5 h-3.5" />
                <span>Dashboard Overview</span>
              </button>
              
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
                  if (onOpenSettings) onOpenSettings();
                  else onSwitchTab('settings');
                  setIsAvatarMenuOpen(false);
                }} 
                className="w-full text-left px-2.5 py-1.5 rounded-lg hover:bg-surface-hover text-neutral-300 flex items-center gap-2"
              >
                <SettingsIcon className="w-3.5 h-3.5" />
                <span>Preferences</span>
              </button>
              
            </div>
          )}
        </div>
      </div>
    </aside>
  );
};

export default IconSidebar;
