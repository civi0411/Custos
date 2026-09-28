import os
from pathlib import Path
from typing import Dict, List, Optional, Tuple

CUSTOS_ROOT = Path(os.environ.get("CUSTOS_REPO_ROOT", Path(__file__).resolve().parents[3])).resolve()
WORKSPACE_ROOT = CUSTOS_ROOT.parent

REPO_MAP: Dict[str, Path] = {
    "custos": CUSTOS_ROOT,
    "goose": WORKSPACE_ROOT / "goose",
}

IGNORED_DIRS = {
    "target", ".git", "node_modules", ".venv", "venv", 
    "__pycache__", "dist", "build", ".idea", ".vscode"
}

IGNORED_EXTENSIONS = {
    ".lock", ".png", ".jpg", ".jpeg", ".ico", ".svg", 
    ".wasm", ".bin", ".tar", ".gz", ".zip", ".pyc",
    ".db", ".sqlite", ".sqlite3"
}

LANGUAGE_EXTENSIONS = {
    ".rs": "rust",
    ".py": "python",
    ".toml": "toml",
    ".md": "markdown",
    ".json": "json",
    ".yaml": "yaml",
    ".yml": "yaml",
    ".sh": "shell",
    ".bash": "shell",
    ".js": "javascript",
    ".ts": "typescript",
    ".tsx": "typescript",
    ".jsx": "javascript",
    ".html": "html",
    ".css": "css",
}

def resolve_repo_path(repo: str) -> Path:
    repo_key = repo.lower().strip()
    if repo_key not in REPO_MAP:
        raise ValueError(f"Unknown repo '{repo}'. Supported repos: {list(REPO_MAP.keys())}")
    path = REPO_MAP[repo_key]
    if not path.exists():
        raise FileNotFoundError(f"Repo path '{path}' does not exist.")
    return path

def is_ignored(path: Path) -> bool:
    for part in path.parts:
        if part in IGNORED_DIRS:
            return True
    if path.suffix in IGNORED_EXTENSIONS:
        return True
    return False

def language_of(path: Path | str) -> str:
    suffix = Path(path).suffix.lower()
    return LANGUAGE_EXTENSIONS.get(suffix, "text")

def list_repo_files(repo: str, subpath: str = "", extensions: Optional[List[str]] = None) -> List[str]:
    base = resolve_repo_path(repo)
    target_dir = base / subpath.lstrip("/")
    if not target_dir.exists():
        return []
    
    ext_set = set(extensions) if extensions else None
    results = []
    for root, dirs, files in os.walk(target_dir):
        dirs[:] = [d for d in dirs if d not in IGNORED_DIRS and d not in {".git", ".idea", ".vscode"}]
        for f in files:
            file_path = Path(root) / f
            if not is_ignored(file_path):
                if ext_set and file_path.suffix not in ext_set:
                    continue
                rel_path = file_path.relative_to(base)
                results.append(str(rel_path))
    return sorted(results)

def read_file_raw(repo: str, path: str) -> Tuple[str, int, int]:
    """
    Returns (content, line_count, size_bytes).
    Reads the complete file without truncation.
    """
    base = resolve_repo_path(repo)
    target = base / path.lstrip("/")
    if not target.exists() or not target.is_file():
        raise FileNotFoundError(f"File '{path}' does not exist in repo '{repo}'.")
    
    content = target.read_text(encoding="utf-8", errors="replace")
    lines = content.splitlines()
    return content, len(lines), target.stat().st_size

def get_repo_stats(repo: str) -> Dict[str, any]:
    files = list_repo_files(repo)
    lang_counts: Dict[str, int] = {}
    lang_lines: Dict[str, int] = {}
    total_lines = 0
    total_size = 0

    base = resolve_repo_path(repo)
    for f in files:
        full_p = base / f
        lang = language_of(f)
        lang_counts[lang] = lang_counts.get(lang, 0) + 1
        try:
            size = full_p.stat().st_size
            total_size += size
            # Rough line estimate for text files
            if lang in {"rust", "python", "toml", "markdown", "shell", "json"}:
                cnt = sum(1 for _ in open(full_p, "rb"))
                lang_lines[lang] = lang_lines.get(lang, 0) + cnt
                total_lines += cnt
        except Exception:
            continue

    return {
        "repo": repo,
        "total_files": len(files),
        "total_size_bytes": total_size,
        "total_lines": total_lines,
        "language_files": lang_counts,
        "language_lines": lang_lines,
    }
