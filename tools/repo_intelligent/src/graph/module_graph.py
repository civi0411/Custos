from pathlib import Path
import re
from typing import Dict, List, Set, Tuple, Optional
import networkx as nx
from src.workspace import resolve_repo_path

class ModuleGraph:
    def __init__(self, repo: str):
        self.repo = repo.lower().strip()
        self.root_path = resolve_repo_path(self.repo)
        self.graph = nx.DiGraph()
        self.crates: Dict[str, Path] = {}
        self._build_graph()

    def _build_graph(self):
        # 1. Discover all Cargo.toml files
        cargo_files = list(self.root_path.glob("**/Cargo.toml"))

        # Find crate names
        for cf in cargo_files:
            if "target" in cf.parts:
                continue
            try:
                content = cf.read_text(encoding="utf-8", errors="replace")
                name_match = re.search(r'\[package\][\s\S]*?name\s*=\s*"([^"]+)"', content)
                if name_match:
                    crate_name = name_match.group(1)
                    rel_dir = cf.parent.relative_to(self.root_path)
                    self.crates[crate_name] = cf.parent
                    self.graph.add_node(crate_name, path=str(rel_dir), repo=self.repo)
            except Exception:
                continue

        # 2. Extract internal dependencies
        for crate_name, crate_dir in self.crates.items():
            cf = crate_dir / "Cargo.toml"
            if not cf.exists():
                continue
            try:
                content = cf.read_text(encoding="utf-8", errors="replace")
                # Look for path dependencies: dep = { path = "..." } or name = { ... }
                for other_name in self.crates.keys():
                    if other_name == crate_name:
                        continue
                    # Match exact crate name in dependencies
                    if re.search(rf'\b{re.escape(other_name)}\s*=', content):
                        self.graph.add_edge(crate_name, other_name, rel="depends_on")
            except Exception:
                continue

    def get_crates(self) -> List[str]:
        return sorted(list(self.crates.keys()))

    def get_crate_path(self, crate_name: str) -> Optional[str]:
        if crate_name in self.crates:
            return str(self.crates[crate_name].relative_to(self.root_path))
        return None

    def dependencies_of(self, crate_name: str) -> List[str]:
        if crate_name not in self.graph:
            return []
        return sorted(list(self.graph.successors(crate_name)))

    def dependents_of(self, crate_name: str) -> List[str]:
        if crate_name not in self.graph:
            return []
        return sorted(list(self.graph.predecessors(crate_name)))

    def get_dependency_tree(self) -> Dict[str, List[str]]:
        tree = {}
        for c in self.get_crates():
            tree[c] = self.dependencies_of(c)
        return tree
