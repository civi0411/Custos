import re
from pathlib import Path
from typing import Optional
from src.workspace import resolve_repo_path, list_repo_files, is_ignored
from src.repo_map import generate_repo_map

MAX_READ_LINES = 120
MAX_SEARCH_RESULTS = 25

def tool_get_repo_map(repo: str = "custos", max_depth: int = 3) -> str:
    """Trả về bản đồ khung xương (Skeleton) tóm tắt các files và symbols chính."""
    try:
        return generate_repo_map(repo, max_depth=int(max_depth))
    except Exception as e:
        return f"Error generating repo map: {str(e)}"

def tool_list_files(repo: str = "custos", subpath: str = "") -> str:
    """Liệt kê danh sách các file trong repo hoặc một thư mục con."""
    try:
        files = list_repo_files(repo, subpath)
        if not files:
            return f"No files found in '{repo}' under '{subpath}'."
        display_files = files[:60]
        summary = f"Found {len(files)} files in '{repo}' (showing first {len(display_files)}):\n"
        summary += "\n".join(f"- {f}" for f in display_files)
        if len(files) > 60:
            summary += f"\n... and {len(files) - 60} more files."
        return summary
    except Exception as e:
        return f"Error listing files: {str(e)}"

def tool_read_file(repo: str = "custos", path: str = "", start_line: int = 1, end_line: int = 100) -> str:
    """Đọc một đoạn mã nguồn trong file kèm theo số dòng."""
    try:
        base = resolve_repo_path(repo)
        target = base / path.lstrip("/")
        if not target.exists() or not target.is_file():
            return f"Error: File '{path}' does not exist in repo '{repo}'."
        
        start_line = max(1, int(start_line))
        end_line = int(end_line)
        if end_line - start_line > MAX_READ_LINES:
            end_line = start_line + MAX_READ_LINES

        lines = target.read_text(encoding="utf-8", errors="replace").splitlines()
        total_lines = len(lines)
        
        selected_lines = lines[start_line - 1 : min(end_line, total_lines)]
        output = [f"=== {repo}:{path} (Lines {start_line} - {min(end_line, total_lines)} of {total_lines}) ==="]
        for idx, line in enumerate(selected_lines, start=start_line):
            output.append(f"{idx:4d} | {line}")
        return "\n".join(output)
    except Exception as e:
        return f"Error reading file '{path}': {str(e)}"

def tool_search_text(repo: str = "custos", query: str = "", path_filter: str = "") -> str:
    """Tìm kiếm từ khóa trong các file văn bản của repo."""
    try:
        base = resolve_repo_path(repo)
        files = list_repo_files(repo, path_filter)
        matches = []
        pattern = re.compile(re.escape(query), re.IGNORECASE)

        for f_rel in files:
            file_path = base / f_rel
            try:
                content = file_path.read_text(encoding="utf-8", errors="replace")
                for line_no, line in enumerate(content.splitlines(), start=1):
                    if pattern.search(line):
                        matches.append(f"{f_rel}:{line_no}: {line.strip()[:140]}")
                        if len(matches) >= MAX_SEARCH_RESULTS:
                            break
            except Exception:
                continue
            if len(matches) >= MAX_SEARCH_RESULTS:
                break

        if not matches:
            return f"No matches found for '{query}' in '{repo}'."
        return f"Found {len(matches)} matches for '{query}' in '{repo}':\n" + "\n".join(matches)
    except Exception as e:
        return f"Error searching text: {str(e)}"

def tool_find_symbol(repo: str = "custos", name: str = "") -> str:
    """Định vị các định nghĩa struct, function, trait, class, enum trùng tên."""
    try:
        base = resolve_repo_path(repo)
        files = list_repo_files(repo)
        
        patterns = [
            re.compile(rf"\b(fn|struct|enum|trait|type|impl)\s+{re.escape(name)}\b"),
            re.compile(rf"\b(def|class)\s+{re.escape(name)}\b"),
        ]
        
        hits = []
        for f_rel in files:
            if not (f_rel.endswith(".rs") or f_rel.endswith(".py")):
                continue
            file_path = base / f_rel
            try:
                lines = file_path.read_text(encoding="utf-8", errors="replace").splitlines()
                for line_no, line in enumerate(lines, start=1):
                    for pat in patterns:
                        if pat.search(line):
                            hits.append(f"{f_rel}:{line_no}: {line.strip()}")
                            break
            except Exception:
                continue
            if len(hits) >= MAX_SEARCH_RESULTS:
                break

        if not hits:
            return f"Symbol '{name}' definition not found in '{repo}'."
        return f"Definitions for symbol '{name}' in '{repo}':\n" + "\n".join(hits)
    except Exception as e:
        return f"Error finding symbol '{name}': {str(e)}"

def tool_find_references(repo: str = "custos", symbol: str = "") -> str:
    """Tìm tất cả các nơi đang sử dụng/gọi (call/reference) đến một symbol."""
    try:
        base = resolve_repo_path(repo)
        files = list_repo_files(repo)
        
        # Word boundary pattern để tìm chỗ gọi hàm/struct
        pattern = re.compile(rf"\b{re.escape(symbol)}\b")
        hits = []
        
        for f_rel in files:
            if not (f_rel.endswith(".rs") or f_rel.endswith(".py") or f_rel.endswith(".toml")):
                continue
            file_path = base / f_rel
            try:
                lines = file_path.read_text(encoding="utf-8", errors="replace").splitlines()
                for line_no, line in enumerate(lines, start=1):
                    stripped = line.strip()
                    # Loại trừ chính dòng định nghĩa để chỉ lấy dòng gọi
                    if stripped.startswith(("//", "#", "/*")):
                        continue
                    if pattern.search(stripped):
                        hits.append(f"{f_rel}:{line_no}: {stripped[:140]}")
                        if len(hits) >= MAX_SEARCH_RESULTS:
                            break
            except Exception:
                continue
            if len(hits) >= MAX_SEARCH_RESULTS:
                break

        if not hits:
            return f"No references found for '{symbol}' in '{repo}'."
        return f"Found {len(hits)} references for '{symbol}' in '{repo}':\n" + "\n".join(hits)
    except Exception as e:
        return f"Error finding references for '{symbol}': {str(e)}"
