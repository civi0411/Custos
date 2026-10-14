import React, { useState } from 'react';
import {
  Terminal,
  FileCode,
  CheckCircle2,
  AlertTriangle,
  ChevronDown,
  ChevronRight,
  Copy,
  Check,
  ExternalLink,
  Globe,
  Sparkles,
  Bot,
  ArrowRight,
  ShieldCheck,
  ShieldQuestion,
  HelpCircle,
  Send,
} from 'lucide-react';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';

export interface AgentActivity {
  kind: string;
  payload: Record<string, any>;
}

export interface AgentActionCardProps {
  activity: AgentActivity;
  onOpenTab?: (id: OrcaTabId) => void;
  onApproveAction?: (actionId: string) => void;
  onRejectAction?: (actionId: string) => void;
  onHandoffToCoding?: (claimId: string, statement: string) => void;
  onSendAnswer?: (answer: string) => void;
}

export const AgentActionCard: React.FC<AgentActionCardProps> = ({
  activity,
  onOpenTab,
  onApproveAction,
  onRejectAction,
  onHandoffToCoding,
  onSendAnswer,
}) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const [copied, setCopied] = useState(false);

  // State for interactive AskUserQuestion
  const [selectedOptions, setSelectedOptions] = useState<number[]>([]);
  const [customAnswer, setCustomAnswer] = useState('');
  const [hasAnswered, setHasAnswered] = useState(false);

  const { kind, payload } = activity;
  const isInput = kind === 'tool_input';
  const isOutput = kind === 'tool_output';
  const isReasoning = kind === 'reasoning_summary' || kind === 'thinking';

  // Normalize parameters: tool parameters may be at root or under payload.input
  const inputObj = typeof payload?.input === 'object' && payload?.input !== null ? payload.input : {};
  const params = { ...(payload || {}), ...inputObj };

  const isApproval = kind === 'approval_request' || kind === 'waiting_approval' || Boolean(params.approval_id || params.permit_id);
  const isHandoff = kind === 'handoff_proposal' || kind === 'claim' || Boolean(params.claim_id);

  const toolName = String(params.name || params.tool || params.command || '').toLowerCase();
  const isQuestion =
    toolName === 'askuserquestion' ||
    toolName === 'ask_user_question' ||
    kind === 'question' ||
    Boolean(params.questions) ||
    Boolean(params.question && params.options);

  const isSubagent =
    kind.includes('subagent') ||
    toolName.includes('subagent') ||
    toolName === 'delegate_task' ||
    Boolean(params.subagent_id || params.subagents);

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  // 1. REASONING / THINKING CARD
  if (isReasoning) {
    const summary = payload.summary || payload.text || 'Deliberating agent strategy...';
    return (
      <div className="my-2 rounded-lg border border-border-default/60 bg-surface-1/70 overflow-hidden text-xs transition">
        <button
          type="button"
          onClick={() => setIsExpanded(!isExpanded)}
          className="w-full flex items-center justify-between px-3 py-2 text-fg-muted hover:text-fg-editor hover:bg-surface-2/50 transition cursor-pointer"
        >
          <div className="flex items-center gap-2 min-w-0">
            <Sparkles className="w-3.5 h-3.5 text-amber-400 shrink-0 animate-pulse" />
            <span className="font-medium text-fg-editor text-[11px] uppercase tracking-wider">Reasoning</span>
            <span className="truncate text-fg-subtle text-[11px] max-w-sm">{String(summary).slice(0, 70)}...</span>
          </div>
          {isExpanded ? <ChevronDown className="w-3.5 h-3.5 text-fg-muted" /> : <ChevronRight className="w-3.5 h-3.5 text-fg-muted" />}
        </button>
        {isExpanded && (
          <div className="px-3 pb-3 pt-1 border-t border-border-muted/50 text-[12px] text-fg-muted leading-relaxed font-sans whitespace-pre-wrap">
            {String(summary)}
          </div>
        )}
      </div>
    );
  }

  // 2. APPROVAL GATE CARD (Orca NativeChatApprovalCard parity)
  if (isApproval) {
    const actionId = payload.approval_id || payload.id || payload.permit_id || 'action';
    const actionTitle = payload.title || payload.action || 'Permission Request';
    const actionDesc = payload.description || payload.reason || 'Agent requests permission to proceed with this action.';

    return (
      <div className="my-2.5 rounded-lg border border-amber-500/40 bg-amber-500/5 p-3 text-xs shadow-xs">
        <div className="flex items-start gap-2.5">
          <ShieldQuestion className="w-4 h-4 text-amber-400 shrink-0 mt-0.5" />
          <div className="min-w-0 flex-1">
            <div className="font-semibold text-fg-editor text-[12px]">{actionTitle}</div>
            <p className="text-[11px] text-fg-muted mt-0.5 leading-relaxed">{actionDesc}</p>
            <div className="flex items-center gap-2 mt-2.5">
              {onApproveAction && (
                <button
                  type="button"
                  onClick={() => onApproveAction(actionId)}
                  className="px-2.5 py-1 rounded bg-emerald-600 hover:bg-emerald-500 text-white font-medium text-[11px] transition shadow-xs cursor-pointer"
                >
                  Approve & Run
                </button>
              )}
              {onRejectAction && (
                <button
                  type="button"
                  onClick={() => onRejectAction(actionId)}
                  className="px-2.5 py-1 rounded bg-surface-2 hover:bg-surface-3 text-fg-editor text-[11px] border border-border-default transition cursor-pointer"
                >
                  Reject
                </button>
              )}
            </div>
          </div>
        </div>
      </div>
    );
  }

  // 3. INTERACTIVE QUESTION CARD (Orca NativeChatQuestionCard parity)
  if (isQuestion && !hasAnswered) {
    const questionsList = Array.isArray(params.questions)
      ? params.questions
      : [
          {
            question: params.question || params.prompt || 'Please clarify your requirement:',
            header: params.header,
            multiSelect: Boolean(params.multiSelect || params.multiple),
            options: params.options || [],
          },
        ];

    const currentQ = questionsList[0] || {};
    const questionText = currentQ.question || 'Action needed';
    const rawOptions = currentQ.options || [];
    const isMultiSelect = Boolean(currentQ.multiSelect || currentQ.multiple);

    const normalizedOptions: Array<{ label: string; description?: string }> = rawOptions.map(
      (opt: any) => (typeof opt === 'string' ? { label: opt } : opt)
    );

    const toggleOption = (idx: number) => {
      if (isMultiSelect) {
        setSelectedOptions((prev) =>
          prev.includes(idx) ? prev.filter((i) => i !== idx) : [...prev, idx]
        );
      } else {
        setSelectedOptions([idx]);
      }
    };

    const handleSubmitAnswer = () => {
      const pickedLabels = selectedOptions
        .map((i) => normalizedOptions[i]?.label)
        .filter(Boolean);
      if (customAnswer.trim()) {
        pickedLabels.push(customAnswer.trim());
      }
      const answerStr = pickedLabels.join(', ');
      if (answerStr && onSendAnswer) {
        onSendAnswer(answerStr);
        setHasAnswered(true);
      }
    };

    return (
      <div className="my-3 rounded-lg border border-primary/40 bg-surface-1 p-3.5 text-xs shadow-md">
        <div className="flex items-start gap-2.5 mb-3">
          <HelpCircle className="w-4 h-4 text-primary shrink-0 mt-0.5" />
          <div className="min-w-0 flex-1">
            <div className="font-semibold text-fg-editor text-[13px] leading-snug">
              {questionText}
            </div>
            {currentQ.header && (
              <div className="text-[11px] text-fg-subtle mt-0.5">{currentQ.header}</div>
            )}
          </div>
        </div>

        {/* Options list */}
        {normalizedOptions.length > 0 && (
          <div className="space-y-1.5 mb-3">
            {normalizedOptions.map((opt, idx) => {
              const isSelected = selectedOptions.includes(idx);
              return (
                <button
                  key={idx}
                  type="button"
                  onClick={() => toggleOption(idx)}
                  className={`w-full flex items-start gap-2.5 p-2 rounded-md text-left transition border cursor-pointer ${
                    isSelected
                      ? 'bg-primary/10 border-primary text-fg-editor shadow-xs'
                      : 'bg-surface-2/60 border-border-default/60 text-fg-muted hover:bg-surface-2 hover:text-fg-editor'
                  }`}
                >
                  <div
                    className={`w-3.5 h-3.5 rounded mt-0.5 flex items-center justify-center border text-[9px] shrink-0 ${
                      isSelected
                        ? 'bg-primary border-primary text-white'
                        : 'border-border-default bg-surface-1'
                    }`}
                  >
                    {isSelected && <Check className="w-2.5 h-2.5 stroke-[3]" />}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="font-medium text-[12px]">{opt.label}</div>
                    {opt.description && (
                      <div className="text-[10px] text-fg-subtle mt-0.5 leading-normal">
                        {opt.description}
                      </div>
                    )}
                  </div>
                </button>
              );
            })}
          </div>
        )}

        {/* Free text custom answer */}
        <div className="flex items-center gap-2 pt-1 border-t border-border-muted/50">
          <input
            type="text"
            value={customAnswer}
            onChange={(e) => setCustomAnswer(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleSubmitAnswer();
            }}
            placeholder="Type custom answer or additional notes..."
            className="flex-1 bg-surface-2 px-2.5 py-1.5 rounded border border-border-default text-fg-editor placeholder-fg-subtle text-[11px] outline-none focus:border-primary"
          />
          <button
            type="button"
            onClick={handleSubmitAnswer}
            disabled={selectedOptions.length === 0 && !customAnswer.trim()}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded bg-primary hover:bg-primary/90 text-white font-medium text-[11px] disabled:opacity-40 disabled:cursor-not-allowed transition cursor-pointer shadow-xs"
          >
            <span>Submit</span>
            <Send className="w-3 h-3" />
          </button>
        </div>
      </div>
    );
  }

  // 4. SUBAGENT DELEGATION RUN CARD (Orca NativeChatSubagentRun parity)
  if (isSubagent) {
    const subagentName =
      payload.subagent_name || payload.name || payload.agent || payload.subagent_id || 'Subagent Worker';
    const subagentState = payload.state || payload.status || 'running';
    const instruction = payload.instruction || payload.task || payload.description || 'Delegated task';
    const steps = payload.steps || payload.activities || [];
    const isDone = subagentState === 'done' || subagentState === 'completed';
    const isWorking = subagentState === 'working' || subagentState === 'running';
    const isFailed = subagentState === 'failed' || subagentState === 'error';

    return (
      <div className="my-2.5 rounded-lg border border-border-default bg-surface-1 overflow-hidden text-xs shadow-xs">
        <div className="flex items-center justify-between px-3 py-2 bg-surface-2/60 border-b border-border-muted">
          <div className="flex items-center gap-2 min-w-0">
            <Bot className="w-3.5 h-3.5 text-purple-400 shrink-0" />
            <span className="font-mono text-fg-editor font-semibold text-[11px] truncate">
              {subagentName}
            </span>
            <span
              className={`inline-flex items-center gap-1 text-[10px] px-1.5 py-0.2 rounded font-sans font-medium ${
                isDone
                  ? 'bg-emerald-500/10 text-emerald-400'
                  : isWorking
                  ? 'bg-purple-500/10 text-purple-400'
                  : isFailed
                  ? 'bg-rose-500/10 text-rose-400'
                  : 'bg-surface-3 text-fg-muted'
              }`}
            >
              {isWorking && <span className="w-1.5 h-1.5 rounded-full bg-purple-400 animate-pulse" />}
              {isDone && <CheckCircle2 className="w-2.5 h-2.5" />}
              {isFailed && <AlertTriangle className="w-2.5 h-2.5" />}
              <span>{subagentState}</span>
            </span>
          </div>
          <button
            type="button"
            onClick={() => setIsExpanded(!isExpanded)}
            className="p-1 rounded text-fg-muted hover:text-fg-editor hover:bg-surface-3 transition"
          >
            {isExpanded ? <ChevronDown className="w-3.5 h-3.5" /> : <ChevronRight className="w-3.5 h-3.5" />}
          </button>
        </div>

        <div className="px-3 py-2 text-fg-muted text-[11px] leading-relaxed">
          {instruction}
        </div>

        {isExpanded && steps.length > 0 && (
          <div className="px-3 pb-2.5 pt-1 border-t border-border-muted/50 space-y-1 font-mono text-[10px] text-fg-subtle">
            {steps.map((st: any, i: number) => (
              <div key={i} className="flex items-center gap-2">
                <span className="text-border-default">›</span>
                <span className="truncate">{typeof st === 'string' ? st : JSON.stringify(st)}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    );
  }

  // 5. RESEARCH HANDOFF TO CODING CARD
  if (isHandoff && payload.claim_id) {
    const claimId = String(payload.claim_id);
    const statement = String(payload.statement || payload.title || 'Verified Claim');

    return (
      <div className="my-2.5 rounded-lg border border-purple-500/30 bg-purple-500/5 p-3 text-xs shadow-xs flex items-center justify-between">
        <div className="flex items-center gap-2.5 min-w-0">
          <ShieldCheck className="w-4 h-4 text-purple-400 shrink-0" />
          <div className="min-w-0">
            <div className="font-semibold text-fg-editor text-[12px] truncate">{statement}</div>
            <div className="text-[10px] text-fg-muted">Evidence verified and ready for engineering implementation</div>
          </div>
        </div>
        {onHandoffToCoding && (
          <button
            type="button"
            onClick={() => onHandoffToCoding(claimId, statement)}
            className="flex items-center gap-1.5 px-2.5 py-1 rounded bg-purple-600 hover:bg-purple-500 text-white text-[11px] font-medium transition shrink-0 ml-3 cursor-pointer shadow-xs"
          >
            <span>Handoff to Coding</span>
            <ArrowRight className="w-3 h-3" />
          </button>
        )}
      </div>
    );
  }

  // 6. FILE MODIFICATION & DIFF CARD
  const isFileChange =
    toolName === 'file_change' ||
    toolName === 'edit' ||
    toolName === 'write' ||
    toolName === 'multiedit' ||
    toolName.includes('file') ||
    Boolean(params.changes || params.path || params.filePath || params.file_path);

  if (isFileChange && (params.changes || params.path || params.filePath || params.file_path || params.diff)) {
    const filePath = String(params.path || params.filePath || params.file_path || params.file || 'Modified File');
    const fileName = filePath.split(/[\\/]/).pop() || filePath;
    const diffText = String(params.changes || params.diff || params.content || '');

    return (
      <div className="my-2 rounded-lg border border-border-default bg-surface-1 overflow-hidden text-xs shadow-xs">
        <div className="flex items-center justify-between px-3 py-2 bg-surface-2/60 border-b border-border-muted">
          <div className="flex items-center gap-2 min-w-0">
            <FileCode className="w-3.5 h-3.5 text-blue-400 shrink-0" />
            <span className="font-mono text-fg-editor font-medium text-[12px] truncate" title={filePath}>
              {fileName}
            </span>
            <span className="text-[10px] text-fg-muted truncate max-w-[150px] hidden sm:inline" title={filePath}>
              {filePath}
            </span>
          </div>
          <div className="flex items-center gap-1.5 shrink-0">
            {onOpenTab && (
              <button
                type="button"
                onClick={() => onOpenTab('files')}
                className="flex items-center gap-1 px-2 py-0.5 rounded bg-surface-3 hover:bg-surface-0 text-fg-editor text-[10px] border border-border-default transition cursor-pointer"
                title="Inspect in Files & Changes tab"
              >
                <ExternalLink className="w-2.5 h-2.5 text-blue-400" />
                <span>View Diff</span>
              </button>
            )}
            <button
              type="button"
              onClick={() => setIsExpanded(!isExpanded)}
              className="p-1 rounded text-fg-muted hover:text-fg-editor hover:bg-surface-3 transition"
            >
              {isExpanded ? <ChevronDown className="w-3.5 h-3.5" /> : <ChevronRight className="w-3.5 h-3.5" />}
            </button>
          </div>
        </div>

        {isExpanded && diffText && (
          <div className="relative font-mono text-[11px] p-2 bg-canvas overflow-x-auto max-h-60 leading-relaxed border-t border-border-muted">
            <button
              type="button"
              onClick={() => copyToClipboard(diffText)}
              className="absolute top-2 right-2 p-1 rounded bg-surface-2 hover:bg-surface-3 text-fg-muted hover:text-fg-editor border border-border-default transition"
              title="Copy diff"
            >
              {copied ? <Check className="w-3 h-3 text-emerald-400" /> : <Copy className="w-3 h-3" />}
            </button>
            <pre className="whitespace-pre-wrap break-words">
              {diffText.split('\n').map((line, idx) => {
                const isAdd = line.startsWith('+');
                const isDel = line.startsWith('-');
                return (
                  <div
                    key={idx}
                    className={`px-1 rounded-xs ${
                      isAdd
                        ? 'bg-emerald-500/10 text-emerald-400'
                        : isDel
                        ? 'bg-rose-500/10 text-rose-400'
                        : 'text-fg-muted'
                    }`}
                  >
                    {line}
                  </div>
                );
              })}
            </pre>
          </div>
        )}
      </div>
    );
  }

  // 7. TERMINAL / COMMAND EXECUTION CARD
  const isCommand =
    toolName === 'command_execution' ||
    toolName === 'bash' ||
    toolName === 'exec' ||
    toolName === 'terminal' ||
    toolName === 'shell_exec' ||
    Boolean(params.command || params.cmd);

  if (isCommand) {
    const cmd = String(params.command || params.cmd || params.name || '');
    const output = String(params.output || params.stdout || params.aggregated_output || '');
    const exitCode = params.exit_code ?? params.exitCode ?? 0;
    const isSuccess = exitCode === 0;

    return (
      <div className="my-2 rounded-lg border border-border-default bg-canvas overflow-hidden text-xs shadow-xs font-mono">
        <div className="flex items-center justify-between px-3 py-1.5 bg-surface-1 border-b border-border-muted">
          <div className="flex items-center gap-2 min-w-0">
            <Terminal className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
            <span className="text-[11px] text-fg-editor truncate font-semibold" title={cmd}>
              $ {cmd || 'terminal command'}
            </span>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <span
              className={`inline-flex items-center gap-1 text-[10px] px-1.5 py-0.2 rounded font-sans font-medium ${
                isSuccess ? 'bg-emerald-500/10 text-emerald-400' : 'bg-rose-500/10 text-rose-400'
              }`}
            >
              {isSuccess ? <CheckCircle2 className="w-2.5 h-2.5" /> : <AlertTriangle className="w-2.5 h-2.5" />}
              <span>{isSuccess ? 'Exit 0' : `Exit ${exitCode}`}</span>
            </span>
            <button
              type="button"
              onClick={() => setIsExpanded(!isExpanded)}
              className="p-1 rounded text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition"
            >
              {isExpanded ? <ChevronDown className="w-3.5 h-3.5" /> : <ChevronRight className="w-3.5 h-3.5" />}
            </button>
          </div>
        </div>

        {isExpanded && output && (
          <div className="relative p-2.5 text-[11px] text-fg-muted overflow-x-auto max-h-52 whitespace-pre-wrap leading-relaxed border-t border-border-muted/50 bg-surface-0/60">
            <button
              type="button"
              onClick={() => copyToClipboard(output)}
              className="absolute top-2 right-2 p-1 rounded bg-surface-2 hover:bg-surface-3 text-fg-muted hover:text-fg-editor border border-border-default transition"
              title="Copy output"
            >
              {copied ? <Check className="w-3 h-3 text-emerald-400" /> : <Copy className="w-3 h-3" />}
            </button>
            {output}
          </div>
        )}
      </div>
    );
  }

  // 8. WEB / LITERATURE DISCOVERY SEARCH CARD
  const isSearch =
    toolName === 'web_search' ||
    toolName === 'search' ||
    toolName.includes('pubmed') ||
    toolName.includes('arxiv') ||
    Boolean(params.query || params.search_term);

  if (isSearch) {
    const query = String(params.query || params.keyword || params.name || params.search_term || '');
    const resultsCount = params.results_count ?? params.count ?? params.output?.length;

    return (
      <div className="my-2 rounded-lg border border-border-default bg-surface-1 p-2.5 flex items-center justify-between text-xs shadow-xs">
        <div className="flex items-center gap-2 min-w-0">
          <Globe className="w-4 h-4 text-sky-400 shrink-0" />
          <div className="min-w-0">
            <div className="text-[11px] font-medium text-fg-editor truncate">
              Literature Query: <span className="font-mono text-sky-300">"{query}"</span>
            </div>
            {resultsCount !== undefined && (
              <div className="text-[10px] text-fg-muted">Indexed {resultsCount} evidence citations</div>
            )}
          </div>
        </div>
        {onOpenTab && (
          <button
            type="button"
            onClick={() => onOpenTab('literature')}
            className="flex items-center gap-1 px-2 py-1 rounded bg-surface-2 hover:bg-surface-3 text-fg-editor text-[11px] border border-border-default transition shrink-0 ml-2 cursor-pointer"
          >
            <span>Inspect Sources</span>
            <ArrowRight className="w-3 h-3 text-purple-400" />
          </button>
        )}
      </div>
    );
  }

  // 9. GENERIC STRUCTURED TOOL CARD
  return (
    <div className="my-1.5 rounded-lg border border-border-muted bg-surface-1/50 overflow-hidden text-xs">
      <button
        type="button"
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full flex items-center justify-between px-3 py-1.5 text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition cursor-pointer"
      >
        <div className="flex items-center gap-2 min-w-0">
          <Bot className="w-3.5 h-3.5 text-fg-subtle shrink-0" />
          <span className="font-mono text-[11px] text-fg-editor truncate">{payload.name || kind}</span>
          <span className="text-[10px] font-sans px-1.5 py-0.2 rounded bg-surface-2 text-fg-muted uppercase">
            {isInput ? 'call' : isOutput ? 'result' : kind}
          </span>
        </div>
        {isExpanded ? <ChevronDown className="w-3.5 h-3.5 text-fg-muted" /> : <ChevronRight className="w-3.5 h-3.5 text-fg-muted" />}
      </button>
      {isExpanded && (
        <div className="p-2 border-t border-border-muted bg-surface-0 font-mono text-[11px] max-h-48 overflow-auto text-fg-muted leading-relaxed">
          <pre className="whitespace-pre-wrap break-words">{JSON.stringify(payload, null, 2)}</pre>
        </div>
      )}
    </div>
  );
};
