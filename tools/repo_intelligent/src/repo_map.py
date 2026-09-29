from typing import Dict, List, Tuple, Optional
from pathlib import Path
from src.workspace import resolve_repo_path, list_repo_files
from src.indexer.index_store import IndexStore

CACHE_MAP: Dict[str, str] = {}

def generate_repo_map(
    repo: str,
    path_prefix: Optional[str] = None,
    max_depth: int = 5,
    max_symbols_per_file: int = 15
) -> str:
    """
    Sinh ra một Repo Map chất lượng cao (AST-based Compressed Skeleton)
    lấy trực tiếp từ SQLite thay vì Regex, lý tưởng cho LLM context window.
    Hỗ trợ path_prefix để lọc riêng từng crate hoặc thư mục con.
    """
    cache_key = f"{repo}:{path_prefix or ''}:{max_depth}:{max_symbols_per_file}"
    if cache_key in CACHE_MAP:
        return CACHE_MAP[cache_key]

    base = resolve_repo_path(repo)
    files = list_repo_files(repo)
    store = IndexStore()
    
    title = f"=== DEEP REPO MAP: {repo.upper()}"
    if path_prefix:
        title += f" [Filter: {path_prefix}]"
    title += " (AST-Powered) ==="
    lines = [title]
    
    for rel_path in files:
        if path_prefix and not rel_path.startswith(path_prefix):
            continue
            
        p = Path(rel_path)
        if len(p.parts) > max_depth:
            continue
        
        if p.suffix in (".rs", ".py"):
            try:
                # Lấy trực tiếp biểu đồ AST từ SQLite (nhanh, 100% chuẩn xác)
                symbols = store.get_symbols_in_file(repo, rel_path)
                if symbols:
                    sym_lines = []
                    for s in symbols[:max_symbols_per_file]:
                        kind = s.get("kind", "sym")
                        name = s.get("name", "")
                        sig = s.get("signature", "")
                        
                        # Rút gọn signature để nhét vừa context window
                        if sig and sig.startswith("("):
                            sig_short = sig[:30] + "..." if len(sig) > 30 else sig
                            sym_lines.append(f"{kind} {name}{sig_short}")
                        else:
                            sym_lines.append(f"{kind} {name}")
                    
                    sym_summary = " | ".join(sym_lines)
                    if len(symbols) > max_symbols_per_file:
                        sym_summary += f" ...(+{len(symbols) - max_symbols_per_file} symbols)"
                    
                    lines.append(f"📄 {rel_path}\n   ↳ [{sym_summary}]")
                else:
                    lines.append(f"📄 {rel_path}")
            except Exception:
                lines.append(f"📄 {rel_path}")
        elif p.name in ("Cargo.toml", "pyproject.toml", "README.md", "task-lifecycle.md"):
            lines.append(f"📘 {rel_path} (manifest/docs)")

    result = "\n".join(lines)
    CACHE_MAP[cache_key] = result
    return result
