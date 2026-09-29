import json
from typing import Dict, Any, List, Optional
from src.graph.query_engine import QueryEngine
from src.llm_interface.context_pack import ContextPacker

class NexusToolServer:
    """
    Exposes Custos Nexus capabilities as standard callable tools for an LLM.
    (Gemini, Claude, Antigravity, Cursor, etc.).
    """
    def __init__(self, query_engine: Optional[QueryEngine] = None):
        self.qe = query_engine or QueryEngine()
        self.packer = ContextPacker(self.qe)

    def get_tool_definitions(self) -> List[Dict[str, Any]]:
        return [
            {
                "name": "nexus_overview",
                "description": "Get high-level architectural overview, crate topology, and entry points of a repo.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "repo": {"type": "string", "enum": ["goose", "custos"], "description": "Target repository"}
                    },
                    "required": ["repo"]
                }
            },
            {
                "name": "nexus_symbol",
                "description": "Inspect a symbol (function, struct, trait, enum) with full implementation code, AST signature, docstring, callers, and callees.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "repo": {"type": "string", "enum": ["goose", "custos"]},
                        "symbol": {"type": "string", "description": "Name of struct, function, or trait"}
                    },
                    "required": ["repo", "symbol"]
                }
            },
            {
                "name": "nexus_callers",
                "description": "Trace all inbound callers of a function/method across the entire workspace.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "repo": {"type": "string", "enum": ["goose", "custos"]},
                        "symbol": {"type": "string", "description": "Target function/symbol"}
                    },
                    "required": ["repo", "symbol"]
                }
            },
            {
                "name": "nexus_callees",
                "description": "Trace all outbound calls invoked inside a function/method body.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "repo": {"type": "string", "enum": ["goose", "custos"]},
                        "symbol": {"type": "string", "description": "Target function/symbol"}
                    },
                    "required": ["repo", "symbol"]
                }
            },
            {
                "name": "nexus_flow",
                "description": "Find the shortest execution flow path from entry function to target function in the call graph.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "repo": {"type": "string", "enum": ["goose", "custos"]},
                        "from_symbol": {"type": "string", "description": "Source function name"},
                        "to_symbol": {"type": "string", "description": "Destination function name"}
                    },
                    "required": ["repo", "from_symbol", "to_symbol"]
                }
            },
            {
                "name": "nexus_file",
                "description": "Deep breakdown of all symbols, signatures, and line numbers inside a specific file.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "repo": {"type": "string", "enum": ["goose", "custos"]},
                        "file_path": {"type": "string", "description": "Relative file path"}
                    },
                    "required": ["repo", "file_path"]
                }
            },
            {
                "name": "nexus_crate",
                "description": "Examine crate dependencies, purpose, and exported symbols.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "repo": {"type": "string", "enum": ["goose", "custos"]},
                        "crate": {"type": "string", "description": "Crate name"}
                    },
                    "required": ["repo", "crate"]
                }
            },
            {
                "name": "nexus_search",
                "description": "AST & FTS5 full-text search across all symbols, signatures, and docstrings.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "repo": {"type": "string", "enum": ["goose", "custos"]},
                        "query": {"type": "string", "description": "Search term"}
                    },
                    "required": ["repo", "query"]
                }
            },
            {
                "name": "nexus_map",
                "description": "Get an AST-powered Deep Repository Map of files, structs, traits, enums, functions, and signatures. Supports filtering by subpath/crate.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "repo": {"type": "string", "enum": ["goose", "custos"], "description": "Target repository", "default": "custos"},
                        "path_prefix": {"type": "string", "description": "Optional prefix to filter files e.g. 'crates/runtime/custos-engine'"},
                        "max_depth": {"type": "integer", "description": "Max directory depth", "default": 5},
                        "max_symbols_per_file": {"type": "integer", "description": "Max symbols per file", "default": 15}
                    }
                }
            }
        ]

    def call_tool(self, name: str, args: Dict[str, Any]) -> str:
        repo = args.get("repo", "custos")
        if name == "nexus_overview":
            return self.packer.pack_overview(repo)
        elif name == "nexus_map":
            from src.repo_map import generate_repo_map
            return generate_repo_map(
                repo=repo,
                path_prefix=args.get("path_prefix"),
                max_depth=args.get("max_depth", 5),
                max_symbols_per_file=args.get("max_symbols_per_file", 15)
            )
        elif name == "nexus_symbol":
            return self.packer.pack_function(repo, args.get("symbol", ""))
        elif name == "nexus_callers":
            callers = self.qe.get_callers(args.get("symbol", ""), repo)
            if not callers:
                return f"No callers found for '{args.get('symbol')}' in '{repo}'."
            lines = [f"Callers for '{args.get('symbol')}' in {repo.upper()}:"]
            for c in callers:
                lines.append(f"- `← {c['caller']}` in `{c['file']}` (Line {c.get('start_line', 0)})")
            return "\n".join(lines)
        elif name == "nexus_callees":
            callees = self.qe.get_callees(args.get("symbol", ""), repo)
            if not callees:
                return f"No callees found for '{args.get('symbol')}' in '{repo}'."
            lines = [f"Callees called by '{args.get('symbol')}' in {repo.upper()}:"]
            for c in callees:
                lines.append(f"- `→ {c['callee']}` in `{c.get('file', '')}`")
            return "\n".join(lines)
        elif name == "nexus_flow":
            return self.packer.pack_flow(repo, args.get("from_symbol", ""), args.get("to_symbol", ""))
        elif name == "nexus_file":
            return self.packer.pack_file_deep(repo, args.get("file_path", ""))
        elif name == "nexus_crate":
            return self.packer.pack_crate(repo, args.get("crate", ""))
        elif name == "nexus_search":
            hits = self.qe.search_symbols(args.get("query", ""), repo=repo, limit=20)
            if not hits:
                return f"No symbols found matching '{args.get('query')}' in '{repo}'."
            lines = [f"Found {len(hits)} symbols matching '{args.get('query')}' in {repo.upper()}:"]
            for h in hits:
                container = f"{h['container']}::" if h.get('container') else ""
                lines.append(f"- `[{h['kind']}] {container}{h['name']}` ({h['file_path']}:{h['start_line']})")
                if h.get('docstring'):
                    lines.append(f"  > {h['docstring'].splitlines()[0]}")
            return "\n".join(lines)
        else:
            return f"Unknown tool '{name}'."
