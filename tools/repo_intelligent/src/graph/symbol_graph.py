from typing import List, Dict, Any, Optional, Set
import networkx as nx
from src.indexer.index_store import IndexStore

class SymbolGraph:
    def __init__(self, repo: str, store: Optional[IndexStore] = None):
        self.repo = repo.lower().strip()
        self.store = store or IndexStore()
        self.graph = nx.DiGraph()
        self._build_graph()

    def _build_graph(self):
        # 1. Fetch all symbols in repo and add as nodes
        with self.store._get_conn() as conn:
            sym_rows = conn.execute("""
                SELECT id, name, kind, visibility, container, signature, docstring, file_path, start_line, end_line
                FROM symbols
                WHERE repo = ?
            """, (self.repo,)).fetchall()

            for s in sym_rows:
                node_id = f"{s['container']}::{s['name']}" if s['container'] else s['name']
                self.graph.add_node(
                    node_id,
                    name=s['name'],
                    kind=s['kind'],
                    visibility=s['visibility'],
                    container=s['container'],
                    signature=s['signature'],
                    docstring=s['docstring'],
                    file_path=s['file_path'],
                    start_line=s['start_line'],
                    end_line=s['end_line'],
                    symbol_id=s['id']
                )

            # 2. Fetch all calls in repo and add as edges
            call_rows = conn.execute("""
                SELECT caller_name, callee_name, caller_file
                FROM symbol_calls
                WHERE repo = ?
            """, (self.repo,)).fetchall()

            for c in call_rows:
                caller = c['caller_name']
                callee = c['callee_name']
                # Callee might be un-scoped (e.g. `connect`) while node is scoped (`ExtensionManager::connect`)
                self.graph.add_edge(caller, callee, file=c['caller_file'])

    def who_calls(self, symbol_name: str) -> List[Dict[str, Any]]:
        """
        Finds all symbols that call symbol_name.
        """
        callers = []
        target_nodes = [n for n in self.graph.nodes if n == symbol_name or n.endswith(f"::{symbol_name}")]
        
        visited = set()
        for target in target_nodes:
            if target in self.graph:
                for pred in self.graph.predecessors(target):
                    if pred not in visited:
                        visited.add(pred)
                        data = self.graph.nodes.get(pred, {})
                        callers.append({
                            "caller": pred,
                            "target": target,
                            "file": data.get("file_path", ""),
                            "signature": data.get("signature", ""),
                            "start_line": data.get("start_line", 0),
                            "docstring": data.get("docstring", "")
                        })

        # Also fallback to direct DB lookup for callees that weren't exact node matches
        if not callers:
            db_callers = self.store.get_callers_of(symbol_name, repo=self.repo)
            for c in db_callers:
                if c["caller_name"] not in visited:
                    visited.add(c["caller_name"])
                    callers.append({
                        "caller": c["caller_name"],
                        "target": c["callee_name"],
                        "file": c["caller_file"],
                        "signature": "",
                        "start_line": 0,
                        "docstring": ""
                    })

        return callers

    def what_does_call(self, caller_name: str) -> List[Dict[str, Any]]:
        """
        Finds all functions/symbols called by caller_name.
        """
        callees = []
        caller_nodes = [n for n in self.graph.nodes if n == caller_name or n.endswith(f"::{caller_name}")]

        visited = set()
        for node in caller_nodes:
            if node in self.graph:
                for succ in self.graph.successors(node):
                    if succ not in visited:
                        visited.add(succ)
                        data = self.graph.nodes.get(succ, {})
                        callees.append({
                            "callee": succ,
                            "caller": node,
                            "file": data.get("file_path", ""),
                            "signature": data.get("signature", ""),
                            "start_line": data.get("start_line", 0),
                            "docstring": data.get("docstring", "")
                        })

        if not callees:
            db_callees = self.store.get_callees_of(caller_name, repo=self.repo)
            for c in db_callees:
                if c["callee_name"] not in visited:
                    visited.add(c["callee_name"])
                    callees.append({
                        "callee": c["callee_name"],
                        "caller": caller_name,
                        "file": c.get("caller_file", ""),
                        "signature": "",
                        "start_line": 0,
                        "docstring": ""
                    })

        return callees

    def trace_flow(self, from_symbol: str, to_symbol: str, max_depth: int = 5) -> List[List[str]]:
        """
        Finds shortest execution flow path from from_symbol to to_symbol.
        """
        start_nodes = [n for n in self.graph.nodes if n == from_symbol or n.endswith(f"::{from_symbol}")]
        end_nodes = [n for n in self.graph.nodes if n == to_symbol or n.endswith(f"::{to_symbol}")]

        paths = []
        for s in start_nodes:
            for e in end_nodes:
                try:
                    p = nx.shortest_path(self.graph, source=s, target=e)
                    paths.append(p)
                except (nx.NetworkXNoPath, nx.NodeNotFound):
                    continue

        return sorted(paths, key=len)

    def find_entry_points(self) -> List[Dict[str, Any]]:
        """
        Discovers main(), run(), entrypoint functions in the repository.
        """
        entry_names = {"main", "run", "start", "bootstrap", "execute", "custosd", "goose"}
        entries = []
        for node, data in self.graph.nodes.items():
            name = data.get("name", "")
            if name in entry_names or "main.rs" in data.get("file_path", ""):
                entries.append({
                    "symbol": node,
                    "kind": data.get("kind", ""),
                    "file": data.get("file_path", ""),
                    "line": data.get("start_line", 0),
                    "signature": data.get("signature", ""),
                    "docstring": data.get("docstring", "")
                })
        return entries
