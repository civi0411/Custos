# Vi's Workspace — Founder / AI Systems & Product Intelligence Lead

> **Role:** Founder / AI Systems & Product Intelligence Lead  
> **Core Direction:** Designs Custos' cognitive architecture, System One/System Two interaction, context and memory intelligence, model strategy, domain agents, evidence semantics, and AI evaluation.  
> **Language & Invariants:** Rust (domain interfaces, Zero I/O contracts), Python (fast heuristics in sidecar), TypeScript (agent sidecars). Zero LangChain/LlamaIndex. Native HTTP/JSON-RPC protocols.

---

## 1. Core Focus Areas

1. **System One — Judgment Fabric:**
   - Task classification, relevance scoring, context sufficiency, and risk prediction.
   - Action screening, loop/repetition detection, confidence scoring, and abstention policies.
   - Escalation trigger to System Two, calibration, and evaluation of rules, Jev, Layla, local classifiers, and rerankers.
   - Core Decision Matrix:
     ```text
     When to use deterministic rules?
     When to use local classifier models?
     When to escalate to frontier models?
     When to halt and consult the human operator?
     When to terminate execution?
     ```
2. **System Two — Deliberation Fabric:**
   - Planner behavior, multi-step coding/research reasoning, and critic/reviewer behaviors.
   - Model capability requirements, structured output enforcement, and reasoning workflow graphs.
   - Provider handoff semantics, continuation strategies, and multi-model fallback topologies.
3. **Context Intelligence & Token Economy:**
   - Repository comprehension, AST symbol and file relevance mapping.
   - Retrieval and reranking recipes, token-budgeted `ContextPack` assembly, and context compression.
   - Long-term memory selection, context provenance tracking, and retrieval quality benchmarking.
4. **Three Domain Agents:**
   - **Coding Agent:** Repository understanding, debugging, implementation, code review, verification strategies.
   - **Research Agent:** Retrieval, source screening, claim extraction, contradiction detection, synthesis, citations.
   - **Assistant Agent:** Personal workflows, preference memory, privacy classification, task planning.
5. **Model Intelligence & Strategy:**
   - Model capability taxonomy, selection policies (manual, assisted, automatic).
   - Local versus cloud execution policy, cost–quality–latency trade-offs, and provider compatibility.
   - Separation of Judge versus Reasoning models, and model evaluation test fixtures.
6. **AI Quality Evaluation & Benchmarking:**
   - Canonical evaluation datasets, relevance labels, expected answers, and quality rubrics.
   - Hallucination metrics, evidence-groundedness scoring, and verified task success rate.
   - Empirical comparisons across models, prompts, and cognitive workflows.

---

## 2. Code Ownership & Repository Layout

```text
crates/
├── judgment-sdk/               # System 1 reflex classification traits and scoring
├── cognitive-runtime/          # Cognitive arbiter and two-tier reasoning engine
├── context-compiler/           # Token budgeting, AST symbol extraction, and context packs
└── knowledge-services/         # Workspace profiling, memory selection, and provenance

domain-packs/
├── engineering/                # Coding agent prompts, recipes, and verification rules
├── research/                   # Research agent citation schemas and synthesis recipes
└── assistant/                  # Personal workflow definitions and privacy classifiers

adapters/
├── judgments/
│   ├── rules/                  # Fast deterministic heuristics
│   ├── jev/                    # Judgment evaluation runtime
│   ├── local-classifier/       # Embedded SLM / ONNX classifiers
│   └── reranker/               # Local cross-encoder rerankers
└── providers/semantics/        # Provider-specific capability mapping and tokenizers

evals/
├── ai-quality/                 # Output quality rubrics and hallucination metrics
├── context-retrieval/          # Recall@K and Precision@K retrieval benchmarks
├── system-one/                 # Reflex accuracy and calibration tests
├── coding/                     # SWE-bench style coding verification suites
├── research/                   # Citation validity and synthesis accuracy benchmarks
└── assistant/                  # Workflow completion and privacy leak evaluations

docs/
├── product/                    # Product experience and user workflow specifications
├── ai/                         # Cognitive architecture, prompt design, and reasoning specs
└── research/                   # AI methodology papers, model comparisons, and benchmarks
```

---

## 3. Workspace Purpose & Directory Structure

This space (`dev_docs/vi/`) serves two distinct purposes:
1. **Domain Architecture (`notes/`)**: Localized technical specifications, prompt schemas, cognitive flow diagrams, and evaluation designs.
2. **Development Reports (`reports/`)**: Chronological record of daily sprint progress and sync logs committed directly to branch `vi`.

---

## 4. Standard Daily Report Template (`reports/YYYY-MM-DD.md`)

When committing daily progress, write your report to `reports/YYYY-MM-DD.md` in this directory, commit directly to the `vi` branch alongside your code, and open a Pull Request to `dev`:

```markdown
# Vi Progress Report — YYYY-MM-DD

## 1. Accomplished Today
- [x] Task description (Crate: `cognitive-runtime`)
- [x] Task description (Eval: `evals/ai-quality`)

## 2. Tests & Verification
- AI benchmark results: Recall@10 = 0.94 on repository fixture.
- Invariant verification: Zero I/O invariants preserved in `core-domain`.

## 3. In-Flight Work & Next Steps
- Currently implementing: Reranker integration in `context-compiler`.
- Tomorrow: Define System 1 escalation threshold for coding tasks.

## 4. Blockers & Questions for Truong / Vinh
- Question for Truong on SQLite foreign key constraints for `ActionIntent`.
- Question for Vinh on `ContinuationPacket` handoff transport schema.
```
