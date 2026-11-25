import React from 'react';

interface ToastProps {
  message: string | null;
}

export const Toast: React.FC<ToastProps> = ({ message }) => {
  if (!message) return null;

  return (
    <div className="fixed bottom-10 right-4 z-50 flex items-center gap-2 bg-surface-2 border border-border-default text-fg-editor text-xs px-3.5 py-2.5 rounded-xl shadow-2xl transition-all duration-200 animate-in fade-in slide-in-from-bottom-2">
      <div className="w-2 h-2 rounded-full workbench-accent-bg animate-pulse"></div>
      <span className="font-medium">{message}</span>
    </div>
  );
};
