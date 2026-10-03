import React, { useEffect, useRef } from 'react';
import { SLASH_ITEMS, SlashItem } from './types';
import { Code2, Compass, Bot, Sparkles } from 'lucide-react';

interface ModeSelectorPaletteProps {
  filterQuery: string;
  selectedIndex: number;
  onSelectIndex: (idx: number) => void;
  onSelectItem: (item: SlashItem) => void;
}

export const ModeSelectorPalette: React.FC<ModeSelectorPaletteProps> = ({
  filterQuery,
  selectedIndex,
  onSelectIndex,
  onSelectItem,
}) => {
  const listRef = useRef<HTMLDivElement>(null);

  const query = filterQuery.toLowerCase().trim();
  const filteredItems = SLASH_ITEMS.filter((item) => {
    if (!query) return true;
    return item.name.toLowerCase().includes(query);
  });

  const getIcon = (icon: SlashItem['icon']) => {
    switch (icon) {
      case 'code':
        return <Code2 size={15} />;
      case 'research':
        return <Compass size={15} />;
      case 'assistant':
        return <Bot size={15} />;
      case 'custos':
        return <Sparkles size={15} />;
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
          Không tìm thấy mode phù hợp với "/{filterQuery}"
        </div>
      </div>
    );
  }

  return (
    <div className="slash-palette-dropdown" ref={listRef}>
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
            <div className="slash-item-name" style={{ color: item.color }}>
              {item.name}
            </div>
          </div>
        );
      })}
    </div>
  );
};
