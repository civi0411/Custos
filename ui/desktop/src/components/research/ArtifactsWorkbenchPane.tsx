import React, { useEffect, useState, useMemo } from 'react';
import {
  FileCode,
  GitBranch,
  Search,
  RefreshCw,
  Clock,
  ArrowRight,
  FileText,
  Terminal,
  AlertCircle,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type {
  ArtifactSummary,
  ArtifactLineageGraph,
  ArtifactDetailResponse,
  LineageGraphNode,
} from '@/types/domain';
import type { ArtifactInspectorData, ArtifactVersion } from '@/types/research';
import { ArtifactInspector } from './ArtifactInspector';

interface ArtifactsWorkbenchPaneProps {
  onShowToast?: (msg: string) => void;
  selectedArtifactPath?: string;
  onAskAgent?: (draft: string) => void;
}

export const ArtifactsWorkbenchPane: React.FC<ArtifactsWorkbenchPaneProps> = ({
  onShowToast,
  selectedArtifactPath: initialPath,
  onAskAgent,
}) => {
  const [artifacts, setArtifacts] = useState<ArtifactSummary[]>([]);
  const [selectedPath, setSelectedPath] = useState<string | null>(initialPath || null);
  const [artifactDetail, setArtifactDetail] = useState<ArtifactDetailResponse | null>(null);
  const [lineageGraph, setLineageGraph] = useState<ArtifactLineageGraph | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [activeView, setActiveView] = useState<'inspector' | 'lineage'>('inspector');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchArtifacts = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const list = await daemonClient.listArtifacts();
      setArtifacts(list);
      if (!selectedPath && list.length > 0) {
        setSelectedPath(list[0].artifact_path || list[0].artifactPath || null);
      }
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const fetchDetailAndGraph = async (path: string) => {
    try {
      const [detail, graph] = await Promise.all([
        daemonClient.getArtifact(path),
        daemonClient.getArtifactLineageGraph(path),
      ]);
      setArtifactDetail(detail);
      setLineageGraph(graph);
    } catch (err: any) {
      onShowToast?.(`Failed to load artifact details: ${err?.message || err}`);
    }
  };

  useEffect(() => {
    fetchArtifacts();
  }, []);

  useEffect(() => {
    if (selectedPath) {
      fetchDetailAndGraph(selectedPath);
    } else {
      setArtifactDetail(null);
      setLineageGraph(null);
    }
  }, [selectedPath]);

  const filteredArtifacts = useMemo(() => {
    if (!searchQuery.trim()) return artifacts;
    const q = searchQuery.toLowerCase();
    return artifacts.filter(
      (a) =>
        (a.artifact_path || a.artifactPath || '').toLowerCase().includes(q) ||
        (a.latest_content_hash || a.latestContentHash || '').toLowerCase().includes(q)
    );
  }, [artifacts, searchQuery]);

  // Convert ArtifactDetailResponse into ArtifactInspectorData for ArtifactInspector
  const inspectorData: ArtifactInspectorData | null = useMemo(() => {
    if (!artifactDetail) return null;
    const path = artifactDetail.artifact_path || artifactDetail.artifactPath || 'artifact';
    const versions: ArtifactVersion[] = (artifactDetail.versions || []).map((v) => ({
      label: `v${v.version}`,
      code:
        artifactDetail.content ||
        `# Artifact: ${path}\n# Version: v${v.version}\n# Produced at: ${new Date(
          v.timestamp * 1000
        ).toISOString()}`,
      executionLog:
        v.produced_by_run_id || v.producedByRunId
          ? `Produced by run: ${v.produced_by_run_id || v.producedByRunId}`
          : undefined,
      timestamp: v.timestamp * 1000,
      reviewPassed: undefined,
    }));

    return {
      title: path.split('/').pop() || path,
      filename: path,
      activeVersion: `v${artifactDetail.latest_version || artifactDetail.latestVersion || 1}`,
      versions:
        versions.length > 0
          ? versions
          : [
              {
                label: 'v1',
                code: artifactDetail.content || `# Artifact: ${path}\n`,
                reviewPassed: undefined,
              },
            ],
      inputs: [],
      code: artifactDetail.content || `# Artifact: ${path}\n`,
      language: path.endsWith('.py')
        ? 'python'
        : path.endsWith('.md')
        ? 'markdown'
        : path.endsWith('.json')
        ? 'json'
        : path.endsWith('.ts') || path.endsWith('.tsx')
        ? 'typescript'
        : 'text',
      reviewPassed: undefined,
      reviewFindings: (artifactDetail.annotations || []).map((ann) => ({
        level: ann.status === 'addressed' ? 'ok' : 'warn',
        title: `Annotation by ${ann.actor}`,
        evidence: ann.note,
        check: `target: ${ann.target?.type || 'general'}`,
      })),
    };
  }, [artifactDetail]);

  return (
    <div className="flex h-full w-full flex-col bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
      {/* Clean 44px (h-11) Top Bar */}
      <div className="flex h-11 flex-shrink-0 items-center justify-between border-b border-[var(--color-border-default)] px-3.5 bg-[var(--color-surface-1)]">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <FileCode className="h-4 w-4 text-emerald-400" />
            <span className="text-xs font-semibold text-[var(--color-editor-fg)]">
              Session Artifacts
            </span>
          </div>

          <div className="h-3.5 w-px bg-[var(--color-border-muted)]" />

          <span className="text-[11px] text-[var(--color-fg-subtle)]">
            {artifacts.length} {artifacts.length === 1 ? 'file' : 'files'} tracked
          </span>
        </div>

        <div className="flex items-center gap-2">
          {/* Segmented View Mode Toggle */}
          <div className="flex rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] p-0.5">
            <button
              onClick={() => setActiveView('inspector')}
              className={`flex items-center gap-1.5 rounded px-2.5 py-1 text-xs font-medium transition ${
                activeView === 'inspector'
                  ? 'bg-[var(--color-surface-1)] text-[var(--color-editor-fg)] shadow-sm'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              <FileText className="h-3.5 w-3.5" />
              <span>Inspector</span>
            </button>
            <button
              onClick={() => setActiveView('lineage')}
              className={`flex items-center gap-1.5 rounded px-2.5 py-1 text-xs font-medium transition ${
                activeView === 'lineage'
                  ? 'bg-[var(--color-surface-1)] text-[var(--color-editor-fg)] shadow-sm'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              <GitBranch className="h-3.5 w-3.5" />
              <span>Lineage</span>
            </button>
          </div>

          <button
            onClick={fetchArtifacts}
            disabled={isLoading}
            className="flex items-center gap-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1 text-xs font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] transition disabled:opacity-50"
            title="Refresh Artifacts"
          >
            <RefreshCw className={`h-3.5 w-3.5 ${isLoading ? 'animate-spin' : ''}`} />
            Refresh
          </button>
        </div>
      </div>

      {/* Main Workspace Layout */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left Sidebar: Artifacts Ledger */}
        <div className="flex w-64 flex-shrink-0 flex-col border-r border-[var(--color-border-default)] bg-[var(--color-surface-1)]">
          <div className="border-b border-[var(--color-border-muted)] p-2.5">
            <div className="relative">
              <Search className="absolute left-2.5 top-2 h-3.5 w-3.5 text-[var(--color-fg-muted)]" />
              <input
                type="text"
                placeholder="Search artifacts..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas)] py-1 pl-8 pr-2.5 text-xs text-[var(--color-editor-fg)] placeholder:text-[var(--color-fg-subtle)] focus:border-emerald-500 focus:outline-none"
              />
            </div>
          </div>

          <div className="flex-1 overflow-y-auto p-2 space-y-1">
            {error && (
              <div className="m-2 rounded-lg border border-red-500/20 bg-red-500/10 p-2.5 text-xs text-red-400">
                <AlertCircle className="mb-1 h-3.5 w-3.5 inline mr-1" />
                {error}
              </div>
            )}

            {filteredArtifacts.length === 0 && !isLoading && (
              <div className="py-8 text-center text-xs text-[var(--color-fg-muted)]">
                <FileCode className="mx-auto mb-2 h-6 w-6 opacity-40" />
                <p>No tracked artifacts yet.</p>
                <p className="mt-1 text-[10px] text-[var(--color-fg-subtle)]">
                  Artifacts generated by agents or tools will appear here.
                </p>
              </div>
            )}

            {filteredArtifacts.map((art) => {
              const path = art.artifact_path || art.artifactPath || '';
              const isSelected = selectedPath === path;
              const version = art.latest_version || art.latestVersion || 1;
              const count = art.versions_count || art.versionsCount || 1;
              const isNote = path.startsWith('notes/');

              return (
                <button
                  key={path}
                  onClick={() => setSelectedPath(path)}
                  className={`w-full rounded-lg p-2.5 text-left transition border ${
                    isSelected
                      ? 'border-emerald-500/30 bg-emerald-500/10 text-[var(--color-editor-fg)]'
                      : 'border-transparent hover:border-[var(--color-border-muted)] hover:bg-[var(--color-surface-2)] text-[var(--color-fg-muted)]'
                  }`}
                >
                  <div className="flex items-start justify-between gap-1.5">
                    <div className="flex items-center gap-1.5 truncate">
                      {isNote ? (
                        <FileText className="h-3.5 w-3.5 text-indigo-400 flex-shrink-0" />
                      ) : (
                        <FileCode className="h-3.5 w-3.5 text-emerald-400 flex-shrink-0" />
                      )}
                      <span className="truncate text-xs font-medium text-[var(--color-editor-fg)]">
                        {path.split('/').pop() || path}
                      </span>
                    </div>
                    <span className="rounded bg-[var(--color-surface-3)] px-1.5 py-0.5 text-[9px] font-semibold text-[var(--color-fg-muted)]">
                      v{version}
                    </span>
                  </div>

                  <div className="mt-1 text-[10px] text-[var(--color-fg-subtle)] truncate font-mono">
                    {path}
                  </div>

                  <div className="mt-2 flex items-center justify-between text-[9px] text-[var(--color-fg-muted)]">
                    <span>
                      {count} {count === 1 ? 'version' : 'versions'}
                    </span>
                    <span>Ready</span>
                  </div>
                </button>
              );
            })}
          </div>

          <div className="border-t border-[var(--color-border-muted)] px-3 py-2 text-[10px] text-[var(--color-fg-subtle)] flex items-center justify-between">
            <span>Artifacts: {artifacts.length}</span>
            <span>Versioned</span>
          </div>
        </div>

        {/* Right Content Area */}
        <div className="flex-1 overflow-hidden bg-[var(--color-canvas)]">
          {activeView === 'inspector' ? (
            inspectorData ? (
              <ArtifactInspector data={inspectorData} onAskAgent={onAskAgent} />
            ) : (
              <div className="flex h-full w-full flex-col items-center justify-center p-8 text-center">
                <FileCode className="h-8 w-8 text-[var(--color-fg-muted)] opacity-50 mb-2" />
                <h3 className="text-sm font-semibold text-[var(--color-editor-fg)]">
                  Select an Artifact
                </h3>
                <p className="mt-1 max-w-sm text-xs text-[var(--color-fg-muted)]">
                  Choose an artifact from the list to preview code, inspect execution logs, and insert into chat.
                </p>
              </div>
            )
          ) : (
            /* Lineage DAG Visualizer */
            <div className="flex h-full w-full flex-col overflow-y-auto p-6">
              <div className="mb-4">
                <h2 className="text-sm font-semibold text-[var(--color-editor-fg)] flex items-center gap-2">
                  <GitBranch className="h-4 w-4 text-emerald-500" />
                  Provenance & Lineage Graph
                </h2>
                <p className="text-xs text-[var(--color-fg-muted)] mt-0.5">
                  Visual graph of dependencies, parent versions, and execution runs.
                </p>
              </div>

              {lineageGraph && lineageGraph.nodes.length > 0 ? (
                <div className="space-y-6">
                  {/* Nodes List / Progression */}
                  <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-4">
                    <div className="text-xs font-semibold text-[var(--color-fg-muted)] mb-3 uppercase tracking-wider">
                      Nodes in Graph ({lineageGraph.nodes.length})
                    </div>
                    <div className="grid gap-3 md:grid-cols-2 lg:grid-cols-3">
                      {lineageGraph.nodes.map((node: LineageGraphNode) => (
                        <div
                          key={node.id}
                          className="rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-2)] p-3 text-xs"
                        >
                          <div className="flex items-center justify-between">
                            <span className="flex items-center gap-1.5 font-medium text-[var(--color-editor-fg)]">
                              {node.node_type === 'run' ? (
                                <Terminal className="h-3.5 w-3.5 text-blue-400" />
                              ) : node.node_type === 'note' ? (
                                <FileText className="h-3.5 w-3.5 text-indigo-400" />
                              ) : (
                                <FileCode className="h-3.5 w-3.5 text-emerald-400" />
                              )}
                              {node.label}
                            </span>
                            <span className="rounded bg-[var(--color-surface-3)] px-1.5 py-0.5 text-[9px] uppercase tracking-wide text-[var(--color-fg-muted)]">
                              {node.node_type || node.nodeType}
                            </span>
                          </div>

                          <div className="mt-2 text-[10px] text-[var(--color-fg-muted)] flex items-center gap-1">
                            <Clock className="h-3 w-3" />
                            {new Date(node.timestamp * 1000).toLocaleString()}
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>

                  {/* Edges / Dependencies */}
                  <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-4">
                    <div className="text-xs font-semibold text-[var(--color-fg-muted)] mb-3 uppercase tracking-wider">
                      Directed Provenance Edges ({lineageGraph.edges.length})
                    </div>
                    {lineageGraph.edges.length === 0 ? (
                      <p className="text-xs text-[var(--color-fg-subtle)]">
                        No cross-node lineage edges recorded yet (single root artifact).
                      </p>
                    ) : (
                      <div className="space-y-2">
                        {lineageGraph.edges.map((edge, idx) => (
                          <div
                            key={idx}
                            className="flex items-center gap-3 rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-2)] px-3 py-2 text-xs"
                          >
                            <span className="font-mono text-[11px] text-[var(--color-editor-fg)]">
                              {edge.from}
                            </span>
                            <ArrowRight className="h-3.5 w-3.5 text-emerald-500 flex-shrink-0" />
                            <span className="font-mono text-[11px] text-[var(--color-editor-fg)]">
                              {edge.to}
                            </span>
                            <span className="ml-auto rounded bg-[var(--color-surface-3)] px-1.5 py-0.5 text-[9px] uppercase tracking-wider text-[var(--color-fg-muted)]">
                              {edge.edge_type || edge.edgeType}
                            </span>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              ) : (
                <div className="flex flex-col items-center justify-center p-12 text-center text-xs text-[var(--color-fg-muted)]">
                  <GitBranch className="h-8 w-8 opacity-40 mb-2" />
                  <p>No lineage graph available for the selected path.</p>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
