import re
from typing import List, Optional

def clean_docstring(raw: Optional[str]) -> str:
    """
    Cleans raw doc comments (///, /** */, or \"\"\"/''' ) into readable markdown/plain text.
    """
    if not raw:
        return ""
    
    lines = raw.splitlines()
    cleaned = []
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("///") or stripped.startswith("//!"):
            cleaned.append(stripped[3:].strip())
        elif stripped.startswith("/**") or stripped.startswith("/*"):
            content = stripped.lstrip("/*").rstrip("*/").strip()
            if content:
                cleaned.append(content)
        elif stripped.startswith("*"):
            cleaned.append(stripped.lstrip("* ").strip())
        elif stripped.startswith('"""') or stripped.startswith("'''"):
            content = stripped.strip('"""').strip("'''").strip()
            if content:
                cleaned.append(content)
        else:
            cleaned.append(line.rstrip())
            
    while cleaned and not cleaned[0]:
        cleaned.pop(0)
    while cleaned and not cleaned[-1]:
        cleaned.pop()
        
    return "\n".join(cleaned)

def extract_leading_rust_docs(lines: List[str], line_idx: int) -> str:
    """
    Looks backward from line_idx (0-based) to collect contiguous /// or //! comments.
    Guaranteed bounds-safe.
    """
    if line_idx <= 0 or line_idx > len(lines):
        return ""
        
    docs = []
    curr = line_idx - 1
    # Avoid scanning too far back (max 60 lines of doc comment)
    scanned = 0
    while 0 <= curr < len(lines) and scanned < 60:
        raw_line = lines[curr].strip()
        if raw_line.startswith("#[") or raw_line.startswith("#!["):
            curr -= 1
            scanned += 1
            continue
        if raw_line.startswith("///") or raw_line.startswith("//!"):
            docs.append(raw_line[3:].strip())
            curr -= 1
            scanned += 1
        else:
            break
            
    docs.reverse()
    return "\n".join(docs).strip()
