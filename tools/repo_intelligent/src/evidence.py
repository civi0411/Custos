import os
import sqlite3
from pathlib import Path
from typing import List, Dict, Optional

TOOL_ROOT = Path(__file__).resolve().parents[1]
DB_PATH = Path(os.environ.get("CUSTOS_NEXUS_EVIDENCE_DB", TOOL_ROOT / "evidence.db"))

def get_connection() -> sqlite3.Connection:
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    return conn

def init_db():
    with get_connection() as conn:
        conn.execute("""
            CREATE TABLE IF NOT EXISTS findings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo TEXT NOT NULL,
                file_path TEXT NOT NULL,
                start_line INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                topic TEXT NOT NULL,
                fact TEXT NOT NULL,
                snippet TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        """)
        conn.commit()

init_db()

def save_finding(
    repo: str,
    file_path: str,
    start_line: int,
    end_line: int,
    topic: str,
    fact: str,
    snippet: Optional[str] = None
) -> int:
    with get_connection() as conn:
        cursor = conn.cursor()
        cursor.execute("""
            INSERT INTO findings (repo, file_path, start_line, end_line, topic, fact, snippet)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        """, (repo, file_path, int(start_line), int(end_line), topic, fact, snippet or ""))
        conn.commit()
        return cursor.lastrowid

def list_findings(repo: Optional[str] = None, topic: Optional[str] = None) -> List[Dict]:
    query = "SELECT * FROM findings WHERE 1=1"
    params = []
    if repo:
        query += " AND repo = ?"
        params.append(repo)
    if topic:
        query += " AND topic LIKE ?"
        params.append(f"%{topic}%")
    query += " ORDER BY id DESC"

    with get_connection() as conn:
        rows = conn.execute(query, params).fetchall()
        return [dict(r) for r in rows]

def export_findings_markdown() -> str:
    findings = list_findings()
    if not findings:
        return "No findings recorded yet."
    
    md = ["# Verified Codebase Evidence & Findings\n"]
    for f in findings:
        md.append(f"### [{f['repo'].upper()}] {f['topic']} (Finding #{f['id']})")
        md.append(f"- **Fact:** {f['fact']}")
        md.append(f"- **Evidence Location:** `{f['file_path']}:{f['start_line']}-{f['end_line']}`")
        if f['snippet']:
            md.append("```rust\n" + f['snippet'].strip() + "\n```")
        md.append("")
    return "\n".join(md)
