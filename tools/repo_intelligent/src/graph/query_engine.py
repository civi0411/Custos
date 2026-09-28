from typing import List, Dict, Any, Optional
from src.indexer.index_store import IndexStore
from src.graph.symbol_graph import SymbolGraph
from src.graph.module_graph import ModuleGraph

class QueryEngine:
    def __init__(self, store: Optional[IndexStore] = None):
        self.store = store or IndexStore()
        self._symbol_graphs: Dict[str, SymbolGraph] = {}
        self._module_graphs: Dict[str, ModuleGraph] = {}

    def get_symbol_graph(self, repo: str) -> SymbolGraph:
        repo_key = repo.lower().strip()
        if repo_key not in self._symbol_graphs:
            self._symbol_graphs[repo_key] = SymbolGraph(repo_key, self.store)
        return self._symbol_graphs[repo_key]

    def get_module_graph(self, repo: str) -> ModuleGraph:
        repo_key = repo.lower().strip()
        if repo_key not in self._module_graphs:
            self._module_graphs[repo_key] = ModuleGraph(repo_key)
        return self._module_graphs[repo_key]

    def search_symbols(self, query: str, repo: Optional[str] = None, limit: int = 30) -> List[Dict[str, Any]]:
        return self.store.search_symbols(query, repo=repo, limit=limit)

    def get_symbol_detail(self, symbol_name: str, repo: Optional[str] = None) -> List[Dict[str, Any]]:
        syms = self.store.get_symbol_by_name(symbol_name, repo=repo)
        results = []
        for s in syms:
            r = s["repo"]
            sg = self.get_symbol_graph(r)
            callers = sg.who_calls(s["name"])
            callees = sg.what_does_call(s["name"])
            results.append({
                **s,
                "callers": callers[:10],
                "callees": callees[:10],
                "caller_count": len(callers),
                "callee_count": len(callees)
            })
        return results

    def get_callers(self, symbol_name: str, repo: str) -> List[Dict[str, Any]]:
        sg = self.get_symbol_graph(repo)
        return sg.who_calls(symbol_name)

    def get_callees(self, caller_name: str, repo: str) -> List[Dict[str, Any]]:
        sg = self.get_symbol_graph(repo)
        return sg.what_does_call(caller_name)

    def trace_flow(self, from_symbol: str, to_symbol: str, repo: str) -> List[List[str]]:
        sg = self.get_symbol_graph(repo)
        return sg.trace_flow(from_symbol, to_symbol)

    def get_crates(self, repo: str) -> List[str]:
        mg = self.get_module_graph(repo)
        return mg.get_crates()

    def get_crate_deps(self, crate_name: str, repo: str) -> Dict[str, Any]:
        mg = self.get_module_graph(repo)
        return {
            "crate": crate_name,
            "path": mg.get_crate_path(crate_name),
            "depends_on": mg.dependencies_of(crate_name),
            "depended_by": mg.dependents_of(crate_name)
        }

    def get_file_symbols(self, file_path: str, repo: str) -> List[Dict[str, Any]]:
        return self.store.get_symbols_in_file(repo, file_path)
