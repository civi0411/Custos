#!/usr/bin/env python3
import re
from pathlib import Path

docs = [
    Path('/Users/mac/Project/AgentHub/GOOSE_COMPLETE_ARCHITECTURE_DEEP_DIVE_VI.md'),
    Path('/Users/mac/Project/AgentHub/CUSTOS_COMPLETE_ARCHITECTURE_DEEP_DIVE_VI.md'),
]

for doc in docs:
    print(f"\n=======================================================")
    print(f"VALIDATING: {doc.name}")
    print(f"=======================================================")
    text = doc.read_text(encoding='utf-8')
    lines = text.splitlines()
    print(f"Lines: {len(lines)}, Characters: {len(text)}, Size: {doc.stat().st_size} bytes")

    # 1. Validate file:/// links
    file_links = re.findall(r'file://(/[^#\)\s\"]+)', text)
    unique_links = sorted(set(file_links))
    missing = [l for l in unique_links if not Path(l).exists()]
    print(f"Total file links: {len(file_links)}, Unique links: {len(unique_links)}")
    if missing:
        print(f"  ❌ Missing files referenced ({len(missing)}):")
        for m in missing:
            print(f"    - {m}")
    else:
        print(f"  ✅ All {len(unique_links)} unique file:/// links exist on disk!")

    # 2. Validate Mermaid codeblocks
    mermaid_blocks = re.findall(r'```mermaid\s*\n(.*?)\n```', text, re.DOTALL)
    print(f"Mermaid diagrams found: {len(mermaid_blocks)}")
    for idx, b in enumerate(mermaid_blocks):
        first_line = b.strip().splitlines()[0] if b.strip() else "EMPTY"
        line_count = len(b.strip().splitlines())
        print(f"  - Diagram {idx+1}: '{first_line}' ({line_count} lines)")

    # 3. Check for unfinished TODOs or placeholders
    todos = [i+1 for i, line in enumerate(lines) if "TODO" in line or "todo!" in line or "FIXME" in line]
    if todos:
        print(f"  ⚠️ Potential TODO found at lines: {todos}")
    else:
        print(f"  ✅ No unfinished TODOs or placeholders found.")

print("\nAll validation checks completed.")
