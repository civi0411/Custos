import React, { useState, useMemo } from 'react';
import { Search, ArrowUpDown, ArrowUp, ArrowDown, Copy, Check, Table as TableIcon, FileText } from 'lucide-react';

interface TabularDataViewerProps {
  content: string;
  isReadOnly?: boolean;
  onChangeText?: (text: string) => void;
  className?: string;
  isTsv?: boolean;
}

function parseDelimited(text: string, delimiter: string = ','): { headers: string[]; rows: string[][] } {
  if (!text || !text.trim()) {
    return { headers: [], rows: [] };
  }

  const lines = text.trim().split(/\r?\n/);
  if (lines.length === 0) return { headers: [], rows: [] };

  const parseLine = (line: string): string[] => {
    const fields: string[] = [];
    let current = '';
    let inQuotes = false;

    for (let i = 0; i < line.length; i++) {
      const char = line[i];
      if (char === '"') {
        if (inQuotes && line[i + 1] === '"') {
          current += '"';
          i++;
        } else {
          inQuotes = !inQuotes;
        }
      } else if (char === delimiter && !inQuotes) {
        fields.push(current.trim());
        current = '';
      } else {
        current += char;
      }
    }
    fields.push(current.trim());
    return fields;
  };

  const headers = parseLine(lines[0]);
  const rows: string[][] = [];

  for (let i = 1; i < lines.length; i++) {
    if (!lines[i].trim()) continue;
    const row = parseLine(lines[i]);
    // Pad row with empty strings if shorter than headers
    while (row.length < headers.length) {
      row.push('');
    }
    rows.push(row);
  }

  return { headers, rows };
}

export const TabularDataViewer: React.FC<TabularDataViewerProps> = ({
  content,
  isReadOnly = true,
  onChangeText,
  className = '',
  isTsv = false,
}) => {
  const [viewMode, setViewMode] = useState<'grid' | 'raw'>('grid');
  const [searchQuery, setSearchQuery] = useState('');
  const [sortCol, setSortCol] = useState<number | null>(null);
  const [sortAsc, setSortAsc] = useState<boolean>(true);
  const [pageSize, setPageSize] = useState<number>(100);
  const [currentPage, setCurrentPage] = useState<number>(1);
  const [copied, setCopied] = useState(false);

  const delimiter = isTsv ? '\t' : ',';
  const { headers, rows } = useMemo(() => parseDelimited(content, delimiter), [content, delimiter]);

  const filteredRows = useMemo(() => {
    if (!searchQuery.trim()) return rows;
    const q = searchQuery.toLowerCase();
    return rows.filter((r) => r.some((cell) => cell.toLowerCase().includes(q)));
  }, [rows, searchQuery]);

  const sortedRows = useMemo(() => {
    if (sortCol === null || sortCol >= headers.length) return filteredRows;
    return [...filteredRows].sort((a, b) => {
      const valA = a[sortCol] || '';
      const valB = b[sortCol] || '';
      const numA = Number(valA);
      const numB = Number(valB);

      if (!isNaN(numA) && !isNaN(numB)) {
        return sortAsc ? numA - numB : numB - numA;
      }
      return sortAsc ? valA.localeCompare(valB) : valB.localeCompare(valA);
    });
  }, [filteredRows, sortCol, sortAsc, headers.length]);

  const totalPages = Math.max(1, Math.ceil(sortedRows.length / pageSize));
  const paginatedRows = useMemo(() => {
    if (pageSize >= 10000) return sortedRows;
    const start = (currentPage - 1) * pageSize;
    return sortedRows.slice(start, start + pageSize);
  }, [sortedRows, currentPage, pageSize]);

  const handleSort = (colIdx: number) => {
    if (sortCol === colIdx) {
      if (sortAsc) {
        setSortAsc(false);
      } else {
        setSortCol(null);
        setSortAsc(true);
      }
    } else {
      setSortCol(colIdx);
      setSortAsc(true);
    }
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(content);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className={`flex flex-col h-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] ${className}`}>
      {/* Sub-Header Toolbar */}
      <div className="flex flex-wrap items-center justify-between gap-2 px-4 py-2 border-b border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] text-xs select-none">
        <div className="flex items-center gap-2">
          {/* Mode Switcher */}
          <div className="flex items-center gap-1 bg-[var(--color-surface-2)] p-0.5 rounded-lg border border-[var(--color-border-subtle)]">
            <button
              type="button"
              onClick={() => setViewMode('grid')}
              className={`flex items-center gap-1 px-2.5 py-1 rounded text-xs font-medium transition ${
                viewMode === 'grid'
                  ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)] shadow-sm'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              <TableIcon className="h-3 w-3" />
              <span>Table Grid</span>
            </button>
            <button
              type="button"
              onClick={() => setViewMode('raw')}
              className={`flex items-center gap-1 px-2.5 py-1 rounded text-xs font-medium transition ${
                viewMode === 'raw'
                  ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)] shadow-sm'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              <FileText className="h-3 w-3" />
              <span>Raw Text</span>
            </button>
          </div>

          {/* Dimension Badges */}
          <span className="text-[11px] font-mono text-[var(--color-fg-muted)] px-2 py-0.5 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)]">
            {rows.length.toLocaleString()} rows • {headers.length} columns
          </span>
        </div>

        {/* Search & Actions */}
        {viewMode === 'grid' && (
          <div className="flex items-center gap-2">
            <div className="relative">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-[var(--color-fg-muted)]" />
              <input
                type="text"
                placeholder="Filter rows..."
                value={searchQuery}
                onChange={(e) => {
                  setSearchQuery(e.target.value);
                  setCurrentPage(1);
                }}
                className="pl-8 pr-3 py-1 bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)] rounded text-xs text-[var(--color-editor-fg)] focus:outline-none focus:border-blue-500 w-44"
              />
            </div>

            {/* Page Size Selector */}
            <select
              value={pageSize}
              onChange={(e) => {
                setPageSize(Number(e.target.value));
                setCurrentPage(1);
              }}
              className="bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)] rounded px-2 py-1 text-xs text-[var(--color-editor-fg)] focus:outline-none"
            >
              <option value={50}>50 / page</option>
              <option value={100}>100 / page</option>
              <option value={250}>250 / page</option>
              <option value={100000}>All rows</option>
            </select>

            <button
              type="button"
              onClick={handleCopy}
              className="flex items-center gap-1 px-2 py-1 rounded text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
              title="Copy table content"
            >
              {copied ? <Check className="h-3.5 w-3.5 text-emerald-400" /> : <Copy className="h-3.5 w-3.5" />}
              <span>{copied ? 'Copied' : 'Copy'}</span>
            </button>
          </div>
        )}
      </div>

      {/* Main Content Area */}
      <div className="flex-1 overflow-auto">
        {viewMode === 'grid' ? (
          headers.length === 0 ? (
            <div className="flex flex-col items-center justify-center p-12 text-center text-[var(--color-fg-muted)]">
              <TableIcon className="h-10 w-10 opacity-40 mb-3" />
              <p className="text-xs">No tabular data could be parsed from this file.</p>
            </div>
          ) : (
            <div className="inline-block min-w-full align-middle">
              <table className="min-w-full divide-y divide-[var(--color-border-subtle)] text-xs border-collapse">
                <thead className="bg-[var(--color-surface-2)] sticky top-0 z-10 shadow-sm">
                  <tr>
                    <th className="w-12 px-3 py-2 text-center text-[10px] font-mono font-medium text-[var(--color-fg-subtle)] border-r border-[var(--color-border-subtle)]">
                      #
                    </th>
                    {headers.map((h, i) => (
                      <th
                        key={i}
                        onClick={() => handleSort(i)}
                        className="px-3.5 py-2 text-left font-semibold text-[var(--color-editor-fg)] cursor-pointer hover:bg-[var(--color-surface-3)] transition select-none border-r border-[var(--color-border-subtle)]/60 last:border-r-0"
                      >
                        <div className="flex items-center justify-between gap-1.5">
                          <span className="truncate">{h || `Col ${i + 1}`}</span>
                          {sortCol === i ? (
                            sortAsc ? (
                              <ArrowUp className="h-3 w-3 text-blue-400 shrink-0" />
                            ) : (
                              <ArrowDown className="h-3 w-3 text-blue-400 shrink-0" />
                            )
                          ) : (
                            <ArrowUpDown className="h-2.5 w-2.5 opacity-30 shrink-0" />
                          )}
                        </div>
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody className="divide-y divide-[var(--color-border-subtle)]/40 bg-[var(--color-canvas)] font-mono text-[11px]">
                  {paginatedRows.length === 0 ? (
                    <tr>
                      <td colSpan={headers.length + 1} className="p-8 text-center text-[var(--color-fg-muted)]">
                        No rows matching &quot;{searchQuery}&quot;
                      </td>
                    </tr>
                  ) : (
                    paginatedRows.map((row, rowIdx) => {
                      const absoluteIdx = (currentPage - 1) * pageSize + rowIdx + 1;
                      return (
                        <tr
                          key={rowIdx}
                          className="hover:bg-[var(--color-surface-1)] transition-colors group"
                        >
                          <td className="w-12 px-3 py-1.5 text-center text-[10px] text-[var(--color-fg-subtle)] border-r border-[var(--color-border-subtle)]/60 bg-[var(--color-surface-1)]/30 group-hover:text-[var(--color-editor-fg)]">
                            {absoluteIdx}
                          </td>
                          {row.map((cell, colIdx) => (
                            <td
                              key={colIdx}
                              className="px-3.5 py-1.5 text-[var(--color-editor-fg)] truncate max-w-xs border-r border-[var(--color-border-subtle)]/30 last:border-r-0 select-text"
                              title={cell}
                            >
                              {cell}
                            </td>
                          ))}
                        </tr>
                      );
                    })
                  )}
                </tbody>
              </table>
            </div>
          )
        ) : (
          <textarea
            value={content}
            readOnly={isReadOnly}
            onChange={(e) => onChangeText?.(e.target.value)}
            spellCheck={false}
            className="flex-1 w-full h-full resize-none bg-[var(--color-canvas)] p-5 font-mono text-xs leading-relaxed text-[var(--color-editor-fg)] focus:outline-none selection:bg-zinc-800"
          />
        )}
      </div>

      {/* Pagination Footer */}
      {viewMode === 'grid' && totalPages > 1 && (
        <div className="flex items-center justify-between px-4 py-2 border-t border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] text-xs select-none">
          <span className="text-[11px] text-[var(--color-fg-muted)]">
            Showing {((currentPage - 1) * pageSize + 1).toLocaleString()} to{' '}
            {Math.min(currentPage * pageSize, sortedRows.length).toLocaleString()} of{' '}
            {sortedRows.length.toLocaleString()} rows
          </span>

          <div className="flex items-center gap-1">
            <button
              type="button"
              disabled={currentPage <= 1}
              onClick={() => setCurrentPage((p) => Math.max(1, p - 1))}
              className="px-2.5 py-1 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)] disabled:opacity-40 disabled:cursor-not-allowed hover:bg-[var(--color-surface-3)] transition text-xs"
            >
              Previous
            </button>
            <span className="px-2 text-xs font-mono">
              {currentPage} / {totalPages}
            </span>
            <button
              type="button"
              disabled={currentPage >= totalPages}
              onClick={() => setCurrentPage((p) => Math.min(totalPages, p + 1))}
              className="px-2.5 py-1 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)] disabled:opacity-40 disabled:cursor-not-allowed hover:bg-[var(--color-surface-3)] transition text-xs"
            >
              Next
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
