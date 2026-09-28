import os
import re
from pathlib import Path
from typing import Dict, List, Tuple
from src.workspace import resolve_repo_path, list_repo_files

CACHE_MAP: Dict[str, str] = {}

def extract_symbols_from_rust(code: str) -> List[Tuple[str, str, int]]:
    """Trích xuất symbol (loại, tên, dòng) từ mã nguồn Rust bằng regex tốc độ cao."""
    symbols = []
    # Pattern tìm function, struct, enum, trait
    pattern = re.compile(
        r"^\s*(?:pub(?:\([^)]+\))?\s+)?(fn|struct|enum|trait)\s+([a-zA-Z0-9_]+)",
        re.MULTILINE
    )
    for match in pattern.finditer(code):
        sym_type, sym_name = match.groups()
        line_no = code[:match.start()].count("\n") + 1
        symbols.append((sym_type, sym_name, line_no))
    return symbols

def extract_symbols_from_python(code: str) -> List[Tuple[str, str, int]]:
    """Trích xuất symbol từ mã nguồn Python."""
    symbols = []
    pattern = re.compile(r"^\s*(def|class)\s+([a-zA-Z0-9_]+)", re.MULTILINE)
    for match in pattern.finditer(code):
        sym_type, sym_name = match.groups()
        line_no = code[:match.start()].count("\n") + 1
        symbols.append((sym_type, sym_name, line_no))
    return symbols

def generate_repo_map(repo: str, max_depth: int = 4, max_symbols_per_file: int = 6) -> str:
    """
    Sinh ra một Repo Map thu gọn (Compressed Skeleton) phù hợp với context window của LLM.
    """
    if repo in CACHE_MAP:
        return CACHE_MAP[repo]

    base = resolve_repo_path(repo)
    files = list_repo_files(repo)
    
    lines = [f"=== REPO MAP: {repo.upper()} ==="]
    
    # Gom nhóm theo module/package
    for rel_path in files:
        p = Path(rel_path)
        if len(p.parts) > max_depth:
            continue
        
        file_path = base / rel_path
        if p.suffix in (".rs", ".py"):
            try:
                content = file_path.read_text(encoding="utf-8", errors="replace")
                if p.suffix == ".rs":
                    syms = extract_symbols_from_rust(content)
                else:
                    syms = extract_symbols_from_python(content)
                
                if syms:
                    sym_summary = ", ".join(f"{st} {sn}:{ln}" for st, sn, ln in syms[:max_symbols_per_file])
                    if len(syms) > max_symbols_per_file:
                        sym_summary += f", ...(+{len(syms) - max_symbols_per_file})"
                    lines.append(f"{rel_path} -> [{sym_summary}]")
                else:
                    lines.append(f"{rel_path}")
            except Exception:
                lines.append(f"{rel_path}")
        elif p.name in ("Cargo.toml", "pyproject.toml", "README.md"):
            lines.append(f"{rel_path} (manifest/docs)")

    result = "\n".join(lines)
    CACHE_MAP[repo] = result
    return result
