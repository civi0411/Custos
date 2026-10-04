import React, { useEffect, useRef } from 'react';
import { SlashItem } from './types';
import {
  Code2,
  Compass,
  Bot,
  Sparkles,
  ListTodo,
  PlusCircle,
  StepForward,
  FileDiff,
  ShieldCheck,
  Activity,
  Trash2,
  HelpCircle,
  SlidersHorizontal,
} from 'lucide-react';

interface ModeSelectorPaletteProps {
  items: SlashItem[];
  title: string;
  filterQuery: string;
  selectedIndex: number;
  onSelectIndex: (idx: number) => void;
  onSelectItem: (item: SlashItem) => void;
}

export const ModeSelectorPalette: React.FC<ModeSelectorPaletteProps> = ({
  items,
  title,
  filterQuery,
  selectedIndex,
  onSelectIndex,
  onSelectItem,
}) => {
  const listRef = useRef<HTMLDivElement>(null);

  const query = filterQuery.toLowerCase().trim();
  const filteredItems = items.filter((item) => {
    if (!query) return true;
    return (
      item.name.toLowerCase().includes(query) ||
      item.description.toLowerCase().includes(query) ||
      item.category.toLowerCase().includes(query)
    );
  });

  const getIcon = (icon: SlashItem['icon']) => {
    switch (icon) {
      case 'code':
        return <Code2 size={16} />;
      case 'research':
        return <Compass size={16} />;
      case 'assistant':
        return <Bot size={16} />;
      case 'custos':
        return <Sparkles size={16} />;
      case 'vibe':
        return <Sparkles size={16} />;
      case 'tasks':
        return <ListTodo size={16} />;
      case 'create':
        return <PlusCircle size={16} />;
      case 'advance':
        return <StepForward size={16} />;
      case 'diff':
        return <FileDiff size={16} />;
      case 'permit':
        return <ShieldCheck size={16} />;
      case 'status':
        return <Activity size={16} />;
      case 'clear':
        return <Trash2 size={16} />;
      case 'help':
        return <HelpCircle size={16} />;
      case 'mode':
        return <SlidersHorizontal size={16} />;
      default:
        return <Sparkles size={16} />;
    }
  };

  useEffect(() => {
    const activeEl = listRef.current?.querySelector('.slash-palette-item.active');
    if (activeEl) {
      activeEl.scrollIntoView({ block: 'nearest' });
    }
  }, [selectedIndex]);

  if (filteredItems.length === 0) {
    return (
      <div className="slash-palette-dropdown">
        <div className="slash-palette-empty">
          Không tìm thấy chức năng phù hợp với "/{filterQuery}"
        </div>
      </div>
    );
  }

  return (
    <div className="slash-palette-dropdown" ref={listRef}>
      <div className="slash-palette-header">
        <span>{title}</span>
        <span className="slash-palette-count">{filteredItems.length} chức năng</span>
      </div>
      <div className="slash-palette-list">
        {filteredItems.map((item, idx) => {
          const isActive = selectedIndex === idx;

          return (
            <div
              key={item.id}
              className={`slash-palette-item ${isActive ? 'active' : ''}`}
              onClick={() => onSelectItem(item)}
              onMouseEnter={() => onSelectIndex(idx)}
            >
              <div className="slash-item-icon" style={{ color: item.color }}>
                {getIcon(item.icon)}
              </div>
              <div className="slash-item-content">
                <div className="slash-item-top">
                  <span className="slash-item-name" style={{ color: item.color }}>
                    /{item.name}
                  </span>
                  <span className="slash-item-category">{item.category}</span>
                </div>
                <div className="slash-item-description">{item.description}</div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
