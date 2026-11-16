import { useCallback, useEffect, useId, useRef, useState } from 'react';
import { Plus, RefreshCw, Search, ExternalLink, Copy, MessageSquare, Bookmark, GitCompare, PanelLeftClose, PanelLeft, X } from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { JobActionButton } from '@/components/common';
import { WorkbenchPane, WorkbenchEmptyState, WorkbenchError } from '@/components/common/WorkbenchPane';
import type { SourceRecord, PassageAnchor } from '@/types/research';
import type { OrcaTabId } from '../OrcaTabbedContainer';

interface LiteraturePaneProps {
  sessionId?: string;
  onShowToast?: (message: string) => void;
  onOpenTab?: (id: OrcaTabId) => void;
}

const digestText = async (text: string) => Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text))), byte => byte.toString(16).padStart(2, '0')).join('');

export function LiteraturePane({ sessionId, onShowToast, onOpenTab }: LiteraturePaneProps) {
  const [sources, setSources] = useState<SourceRecord[]>([]);
  const [selectedId, setSelectedId] = useState('');
  const [query, setQuery] = useState('');
  const [checked, setChecked] = useState<Set<string>>(new Set());
  const [showList, setShowList] = useState(true);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [anchors, setAnchors] = useState<PassageAnchor[]>([]);
  const [anchorLoading, setAnchorLoading] = useState(false);
  const [selection, setSelection] = useState('');
  const [savingPassage, setSavingPassage] = useState(false);
  const [adding, setAdding] = useState(false);
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);
  const [form, setForm] = useState({ title: '', doi: '', authors: '', year: String(new Date().getFullYear()), abstract: '' });
  const dialog = useRef<HTMLDialogElement>(null);
  const formId = useId();
  const loadEpoch = useRef(0);
  const selectedRef = useRef(selectedId);
  selectedRef.current = selectedId;
  const activeSource = sources.find(source => source.id === selectedId);
  const load = useCallback(async () => {
    const epoch = ++loadEpoch.current;
    setLoading(true);
    try {
      const loaded = await daemonClient.listResearchSources();
      if (epoch !== loadEpoch.current) return;
      setSources(loaded);
      setSelectedId(previous => loaded.some(item => item.id === previous) ? previous : loaded[0]?.id || '');
      setError(null);
    } catch (reason) { if (epoch === loadEpoch.current) setError(String(reason)); }
    finally { if (epoch === loadEpoch.current) setLoading(false); }
  }, []);
  useEffect(() => { void load(); return () => { loadEpoch.current++; }; }, [load]);
  useEffect(() => {
    let cancelled = false;
    setSelection(''); setAnchors([]);
    if (!selectedId) { setAnchorLoading(false); return; }
    setAnchorLoading(true);
    daemonClient.listResearchAnchors(selectedId).then(records => { if (!cancelled) setAnchors(records); })
      .catch(reason => { if (!cancelled) setError('Could not load passages: ' + String(reason)); })
      .finally(() => { if (!cancelled) setAnchorLoading(false); });
    return () => { cancelled = true; };
  }, [selectedId]);
  useEffect(() => {
    const element = dialog.current;
    if (!element) return;
    if (adding && !element.open) element.showModal();
    else if (!adding && element.open) element.close();
  }, [adding]);

  const savePassage = async () => {
    const source = activeSource, text = selection;
    if (!source?.abstractText || !text || savingPassage) return;
    const start = source.abstractText.indexOf(text);
    if (start < 0 || source.abstractText.indexOf(text, start + 1) >= 0) { setError('Select a unique excerpt so its exact location can be recorded.'); return; }
    setSavingPassage(true);
    try {
      await daemonClient.saveResearchAnchor({ id: crypto.randomUUID(), sourceId: source.id, sourceTitle: source.title,
        sectionTitle: 'Abstract', startOffset: start, endOffset: start + text.length, exactText: text, passageHash: await digestText(text) });
      const saved = await daemonClient.listResearchAnchors(source.id);
      if (selectedRef.current === source.id) { setAnchors(saved); setSelection(''); }
      onShowToast?.('Passage saved with its source and text digest.');
      setError(null);
    } catch (reason) { setError(String(reason)); }
    finally { setSavingPassage(false); }
  };

  const createSource = async (event: React.FormEvent) => {
    event.preventDefault();
    const year = Number(form.year);
    if (!form.title.trim() || !Number.isInteger(year) || year < 1000 || year > new Date().getFullYear() + 1) { setFormError('Enter a title and valid publication year.'); return; }
    if (saving) return;
    setSaving(true); setFormError(null);
    try {
      const source: SourceRecord = { id: crypto.randomUUID(), sourceType: 'paper', title: form.title.trim(), doi: form.doi.trim() || undefined,
        authors: form.authors.split(',').map(author => author.trim()).filter(Boolean), year,
        contentHash: await digestText(form.title.trim() + '\n' + form.abstract.trim()), verified: false, abstractText: form.abstract.trim() || undefined, createdAt: Date.now() };
      await daemonClient.saveResearchSource(source);
      await load();
      setSelectedId(source.id);
      setAdding(false);
      setForm({ title: '', doi: '', authors: '', year: String(new Date().getFullYear()), abstract: '' });
      onShowToast?.('Source added.');
    } catch (reason) { setFormError(String(reason)); }
    finally { setSaving(false); }
  };
  const filtered = sources.filter(source => (source.title + ' ' + (source.authors || []).join(' ') + ' ' + (source.doi || '')).toLowerCase().includes(query.toLowerCase()));
  const field = (name: keyof typeof form, value: string) => setForm(previous => ({ ...previous, [name]: value }));

  return <WorkbenchPane toolbar={<>
    <button type="button" className="wb-icon-button" aria-label={showList ? 'Hide source list' : 'Show source list'} title={showList ? 'Hide source list' : 'Show source list'} onClick={() => setShowList(value => !value)}>{showList ? <PanelLeftClose size={14} /> : <PanelLeft size={14} />}</button>
    <span className="wb-meta">{sources.length} sources</span>
    <div className="wb-toolbar-spacer" />
    <button type="button" className="wb-icon-button" aria-label="Refresh sources" title="Refresh" disabled={loading} onClick={() => void load()}><RefreshCw size={14} /></button>
    <button type="button" className="canvas-open-button" onClick={() => { setFormError(null); setAdding(true); }}><Plus size={12} />Add source</button>
  </>}>
    <WorkbenchError message={error} onRetry={() => void load()} />
    <div className="wb-split flex flex-1 min-h-0">
      {showList && <aside className="wb-master wb-source-list flex min-h-0 flex-col border-r border-border-subtle">
        <label className="wb-search"><Search size={13} aria-hidden="true" /><input aria-label="Search research sources" placeholder="Find a source…" value={query} onChange={event => setQuery(event.target.value)} /></label>
        <div className="flex-1 overflow-auto" role="list" aria-label="Research sources">
          {loading && !sources.length ? <WorkbenchEmptyState title="Loading sources…" busy /> : !filtered.length ? <WorkbenchEmptyState title={query ? 'No matching sources' : 'No sources yet'} description={query ? 'Try another title, author or DOI.' : 'Add a paper and its stored abstract to begin.'} /> : filtered.map(source => <div role="listitem" key={source.id} className={'wb-source-row ' + (selectedId === source.id ? 'is-selected' : '')}>
            <input type="checkbox" aria-label={'Select ' + source.title + ' for comparison'} checked={checked.has(source.id)} onChange={event => setChecked(previous => { const next = new Set(previous); if (event.target.checked) next.add(source.id); else next.delete(source.id); return next; })} />
            <button type="button" aria-current={selectedId === source.id ? 'true' : undefined} onClick={() => setSelectedId(source.id)}>
              <span className="line-clamp-2">{source.title}</span>
              <small>{source.authors?.join(', ') || source.sourceType}{source.year ? ' · ' + source.year : ''}</small>
            </button>
          </div>)}
        </div>
        {checked.size >= 2 && <div className="wb-context-line">
          <JobActionButton jobId="research__compare" label={'Compare ' + checked.size + ' sources'} icon={GitCompare}
            getInputs={() => ({ sources: Array.from(checked).filter(id => sources.some(source => source.id === id)), comparison_criteria: 'Methods, datasets, findings, limitations and reproducibility' })}
            sessionId={sessionId} sourceTabId="literature" onNavigate={onOpenTab} onShowToast={onShowToast} variant="toolbar" size="sm" />
        </div>}
      </aside>}
      <main className="min-h-0 min-w-0 flex-1 overflow-auto">
        {activeSource ? <article className="wb-document select-text">
          <div className="wb-document-actions">
            {activeSource.doi && <a className="canvas-open-button" href={'https://doi.org/' + activeSource.doi} target="_blank" rel="noreferrer"><ExternalLink size={12} />Open source</a>}
            <JobActionButton jobId="research__source_qa" label="Ask about source" icon={MessageSquare}
              getInputs={() => ({ source_locator: activeSource.doi || activeSource.id, question: 'Analyze the claims, methodology, evidence and limitations in "' + activeSource.title + '"' })}
              sessionId={sessionId} sourceTabId="literature" onNavigate={onOpenTab} onShowToast={onShowToast} variant="toolbar" />
          </div>
          <h1>{activeSource.title}</h1>
          <p className="wb-document-byline">{activeSource.authors?.join(', ') || 'Authors not recorded'}{activeSource.year ? ' · ' + activeSource.year : ''}</p>
          <details className="wb-disclosure">
            <summary>Source record</summary>
            <dl className="wb-property-list">
              <dt>Type</dt><dd>{activeSource.sourceType}</dd>
              <dt>DOI</dt><dd>{activeSource.doi || 'Not recorded'}</dd>
              <dt>Content digest</dt><dd className="break-all font-mono">{activeSource.contentHash || 'Not recorded'}{activeSource.contentHash && <button className="wb-icon-button" type="button" aria-label="Copy source digest" onClick={() => { void navigator.clipboard.writeText(activeSource.contentHash).catch(reason => setError(String(reason))); }}><Copy size={12} /></button>}</dd>
              <dt>Record status</dt><dd>{activeSource.verified ? 'Marked verified in source ledger' : 'Not verified'}</dd>
            </dl>
          </details>
          <section>
            <h2>Abstract</h2>
            <p className="wb-meta">Stored text · select an excerpt to save its location</p>
            <div className="wb-reader-text" onMouseUp={() => { const text = window.getSelection()?.toString().trim() || ''; setSelection(text.length > 5 && activeSource.abstractText?.includes(text) ? text : ''); }} onKeyUp={() => { const text = window.getSelection()?.toString().trim() || ''; setSelection(text.length > 5 && activeSource.abstractText?.includes(text) ? text : ''); }}>
              {activeSource.abstractText || <p className="wb-meta">No abstract is stored for this source. Open the original to read its full text.</p>}
            </div>
            {selection && <div className="wb-passage-selection">
              <blockquote>{selection}</blockquote>
              <div className="flex gap-2"><button type="button" className="canvas-open-button" disabled={savingPassage} onClick={() => void savePassage()}><Bookmark size={12} />{savingPassage ? 'Saving…' : 'Save passage'}</button><button type="button" className="wb-icon-button" aria-label="Dismiss passage selection" onClick={() => setSelection('')}><X size={13} /></button></div>
            </div>}
          </section>
          <section>
            <h2>Saved passages <span className="wb-meta">{anchors.length}</span></h2>
            {anchorLoading ? <p role="status">Loading passages…</p> : !anchors.length ? <p className="wb-meta">Select a unique excerpt above to keep a source reference.</p> : anchors.map(anchor => <blockquote key={anchor.id} className="wb-passage"><p>{anchor.exactText}</p><footer>{anchor.sectionTitle || 'Source'} · offsets {anchor.startOffset}–{anchor.endOffset}</footer></blockquote>)}
          </section>
        </article> : <WorkbenchEmptyState title="Read a source" description="Choose a source from the list, or add a paper to your corpus." action={<button type="button" className="canvas-open-button" onClick={() => setAdding(true)}><Plus size={12} />Add source</button>} />}
      </main>
    </div>
    <dialog ref={dialog} className="wb-dialog" onCancel={event => { if (saving) event.preventDefault(); else setAdding(false); }} onClose={() => setAdding(false)} aria-labelledby={formId + '-heading'}>
      <form onSubmit={event => void createSource(event)}>
        <header><h2 id={formId + '-heading'}>Add a source</h2><button type="button" className="wb-icon-button" aria-label="Close add source" disabled={saving} onClick={() => setAdding(false)}><X size={15} /></button></header>
        <WorkbenchError message={formError} />
        <div className="wb-form-fields">
          <label>Title<input autoFocus required value={form.title} onChange={event => field('title', event.target.value)} /></label>
          <div className="wb-form-row"><label>DOI<input value={form.doi} onChange={event => field('doi', event.target.value)} placeholder="Optional" /></label><label>Year<input type="number" min="1000" max={new Date().getFullYear() + 1} required value={form.year} onChange={event => field('year', event.target.value)} /></label></div>
          <label>Authors<input value={form.authors} onChange={event => field('authors', event.target.value)} placeholder="Separated by commas" /></label>
          <label>Abstract<textarea rows={6} value={form.abstract} onChange={event => field('abstract', event.target.value)} placeholder="Paste the source text you want to read and cite" /></label>
        </div>
        <footer><button type="button" className="canvas-open-button" disabled={saving} onClick={() => setAdding(false)}>Cancel</button><button type="submit" className="canvas-open-button" disabled={saving || !form.title.trim()}>{saving ? 'Saving…' : 'Add source'}</button></footer>
      </form>
    </dialog>
  </WorkbenchPane>;
}
export default LiteraturePane;
