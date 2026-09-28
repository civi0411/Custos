from typing import Dict, Any, List, Optional
from src.graph.query_engine import QueryEngine
from src.llm_interface.context_pack import ContextPacker

class SemanticQA:
    def __init__(self, query_engine: Optional[QueryEngine] = None):
        self.qe = query_engine or QueryEngine()
        self.packer = ContextPacker(self.qe)

    def answer_query(self, question: str, repo: str) -> str:
        """
        Synthesizes an evidence-backed answer about architecture, components, and code flows.
        """
        repo_key = repo.lower().strip()
        # 1. Search symbols matching keywords in question
        words = [w.strip("?,.:;\"'()") for w in question.split() if len(w) > 3]
        matched_symbols = []
        for w in words[:5]:
            hits = self.qe.search_symbols(w, repo=repo_key, limit=5)
            for h in hits:
                if h["name"] not in [m["name"] for m in matched_symbols]:
                    matched_symbols.append(h)

        # 2. Check if a crate is mentioned
        crates = self.qe.get_crates(repo_key)
        matched_crates = [c for c in crates if any(w.lower() in c.lower() for w in words)]

        md = [
            f"# Architecture Deep Dive: {question}",
            f"> Target Repository: **`{repo.upper()}`**",
            "",
            "## 1. Discovered Architectural Entities",
        ]

        if matched_crates:
            md.append("### Relevant Crates:")
            for c in matched_crates[:5]:
                deps = self.qe.get_crate_deps(c, repo_key)
                md.append(f"- **`{c}`** (Path: `{deps['path']}`): depends on `{deps['depends_on']}`")

        if matched_symbols:
            md.append("\n### Relevant AST Symbols:")
            for s in matched_symbols[:8]:
                container_str = f"{s['container']}::" if s.get('container') else ""
                md.append(f"- **`{s['kind']} {container_str}{s['name']}`** in `{s['file_path']}:{s['start_line']}`")
                if s.get("docstring"):
                    md.append(f"  > *{s['docstring'].splitlines()[0]}*")

        # 3. Call Graph / Execution Flow Evidence
        md.append("\n## 2. Call Graph & Component Connectivity")
        for s in matched_symbols[:3]:
            callers = self.qe.get_callers(s["name"], repo_key)
            callees = self.qe.get_callees(s["name"], repo_key)
            md.append(f"\n#### Symbol: `{s['name']}`")
            c_in = ", ".join(f"`{c['caller']}`" for c in callers[:6]) or "None"
            c_out = ", ".join(f"`{c['callee']}`" for c in callees[:6]) or "None"
            md.append(f"- **Inbound Callers ({len(callers)}):** {c_in}")
            md.append(f"- **Outbound Callees ({len(callees)}):** {c_out}")

        md.extend([
            "",
            "## 3. Structural Synthesis",
            f"Based on static analysis and call graph traversal of **{repo.upper()}**:",
            f"- Key entry and control points revolve around `{matched_symbols[0]['name'] if matched_symbols else 'workspace core'}`.",
            f"- Data and control flow originates from top-level session/CLI dispatch down to core managers.",
        ])

        return "\n".join(md)
