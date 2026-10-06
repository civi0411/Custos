import os
import sqlite3
from pathlib import Path
from typing import List, Dict, Any, Optional, Tuple
from src.indexer.chunk_builder import SemanticChunk

TOOL_ROOT = Path(__file__).resolve().parents[2]
DB_PATH = Path(os.environ.get("CUSTOS_NEXUS_DB", TOOL_ROOT / "nexus_index.db"))

class IndexStore:
    def __init__(self, db_path: Path = DB_PATH):
        self.db_path = db_path
        self._init_db()

    def _get_conn(self) -> sqlite3.Connection:
        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        conn.execute("PRAGMA journal_mode=WAL")
        conn.execute("PRAGMA synchronous=NORMAL")
        return conn

    def _init_db(self):
        with self._get_conn() as conn:
            conn.executescript("""
            CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo TEXT NOT NULL,
                path TEXT NOT NULL,
                language TEXT NOT NULL,
                line_count INTEGER DEFAULT 0,
                size_bytes INTEGER DEFAULT 0,
                UNIQUE(repo, path)
            );

            CREATE TABLE IF NOT EXISTS symbols (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_id INTEGER,
                repo TEXT NOT NULL,
                file_path TEXT NOT NULL,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                visibility TEXT NOT NULL,
                container TEXT DEFAULT '',
                signature TEXT DEFAULT '',
                docstring TEXT DEFAULT '',
                start_line INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                FOREIGN KEY(file_id) REFERENCES files(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS symbol_calls (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo TEXT NOT NULL,
                caller_symbol_id INTEGER,
                caller_name TEXT NOT NULL,
                caller_file TEXT NOT NULL,
                callee_name TEXT NOT NULL,
                FOREIGN KEY(caller_symbol_id) REFERENCES symbols(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS file_imports (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo TEXT NOT NULL,
                file_path TEXT NOT NULL,
                statement TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS chunks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo TEXT NOT NULL,
                file_path TEXT NOT NULL,
                symbol_name TEXT DEFAULT '',
                kind TEXT DEFAULT '',
                context_header TEXT NOT NULL,
                content TEXT NOT NULL,
                docstring TEXT DEFAULT '',
                start_line INTEGER NOT NULL,
                end_line INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(repo, name);
            CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(repo, file_path);
            CREATE INDEX IF NOT EXISTS idx_calls_caller ON symbol_calls(repo, caller_name);
            CREATE INDEX IF NOT EXISTS idx_calls_callee ON symbol_calls(repo, callee_name);
            CREATE INDEX IF NOT EXISTS idx_chunks_file ON chunks(repo, file_path);
            CREATE INDEX IF NOT EXISTS idx_imports_file ON file_imports(repo, file_path);

            CREATE VIRTUAL TABLE IF NOT EXISTS symbols_fts USING fts5(
                name,
                container,
                signature,
                docstring,
                file_path,
                repo UNINDEXED,
                symbol_id UNINDEXED
            );
            """)

    def clear_repo(self, repo: str):
        repo_key = repo.lower().strip()
        with self._get_conn() as conn:
            conn.execute("DELETE FROM symbols_fts WHERE repo = ?", (repo_key,))
            conn.execute("DELETE FROM chunks WHERE repo = ?", (repo_key,))
            conn.execute("DELETE FROM symbol_calls WHERE repo = ?", (repo_key,))
            conn.execute("DELETE FROM file_imports WHERE repo = ?", (repo_key,))
            conn.execute("DELETE FROM symbols WHERE repo = ?", (repo_key,))
            conn.execute("DELETE FROM files WHERE repo = ?", (repo_key,))

    def swap_from_staging(self, staging_store: "IndexStore", repo: str):
        """
        Atomically promotes a completed staging index generation into the active store
        within a single transaction. Active readers see zero partial-state downtime.
        """
        repo_key = repo.lower().strip()
        staging_path = str(staging_store.db_path.resolve())
        with self._get_conn() as conn:
            conn.execute("ATTACH DATABASE ? AS staging_db", (staging_path,))
            try:
                # Clear active repo data
                conn.execute("DELETE FROM symbols_fts WHERE repo = ?", (repo_key,))
                conn.execute("DELETE FROM chunks WHERE repo = ?", (repo_key,))
                conn.execute("DELETE FROM symbol_calls WHERE repo = ?", (repo_key,))
                conn.execute("DELETE FROM file_imports WHERE repo = ?", (repo_key,))
                conn.execute("DELETE FROM symbols WHERE repo = ?", (repo_key,))
                conn.execute("DELETE FROM files WHERE repo = ?", (repo_key,))

                # Copy records from staging into active store
                conn.execute("""
                    INSERT INTO files (id, repo, path, language, line_count, size_bytes)
                    SELECT id, repo, path, language, line_count, size_bytes FROM staging_db.files WHERE repo = ?
                """, (repo_key,))

                conn.execute("""
                    INSERT INTO symbols (id, file_id, repo, file_path, name, kind, visibility, container, signature, docstring, start_line, end_line)
                    SELECT id, file_id, repo, file_path, name, kind, visibility, container, signature, docstring, start_line, end_line
                    FROM staging_db.symbols WHERE repo = ?
                """, (repo_key,))

                conn.execute("""
                    INSERT INTO symbol_calls (id, repo, caller_symbol_id, caller_name, caller_file, callee_name)
                    SELECT id, repo, caller_symbol_id, caller_name, caller_file, callee_name
                    FROM staging_db.symbol_calls WHERE repo = ?
                """, (repo_key,))

                conn.execute("""
                    INSERT INTO file_imports (id, repo, file_path, statement)
                    SELECT id, repo, file_path, statement
                    FROM staging_db.file_imports WHERE repo = ?
                """, (repo_key,))

                conn.execute("""
                    INSERT INTO chunks (id, repo, file_path, symbol_name, kind, context_header, content, docstring, start_line, end_line)
                    SELECT id, repo, file_path, symbol_name, kind, context_header, content, docstring, start_line, end_line
                    FROM staging_db.chunks WHERE repo = ?
                """, (repo_key,))

                conn.execute("""
                    INSERT INTO symbols_fts (name, container, signature, docstring, file_path, repo, symbol_id)
                    SELECT name, container, signature, docstring, file_path, repo, symbol_id
                    FROM staging_db.symbols_fts WHERE repo = ?
                """, (repo_key,))
            finally:
                conn.execute("DETACH DATABASE staging_db")


    def insert_file(self, repo: str, path: str, language: str, line_count: int, size_bytes: int) -> int:
        with self._get_conn() as conn:
            cur = conn.execute("""
                INSERT OR REPLACE INTO files (repo, path, language, line_count, size_bytes)
                VALUES (?, ?, ?, ?, ?)
            """, (repo.lower(), path, language, line_count, size_bytes))
            return cur.lastrowid

    def insert_symbol(
        self,
        file_id: int,
        repo: str,
        file_path: str,
        name: str,
        kind: str,
        visibility: str,
        container: str,
        signature: str,
        docstring: str,
        start_line: int,
        end_line: int,
        callees: List[str]
    ) -> int:
        repo_key = repo.lower()
        with self._get_conn() as conn:
            cur = conn.execute("""
                INSERT INTO symbols (file_id, repo, file_path, name, kind, visibility, container, signature, docstring, start_line, end_line)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """, (file_id, repo_key, file_path, name, kind, visibility, container, signature, docstring, start_line, end_line))
            sym_id = cur.lastrowid

            # Add to FTS
            conn.execute("""
                INSERT INTO symbols_fts (name, container, signature, docstring, file_path, repo, symbol_id)
                VALUES (?, ?, ?, ?, ?, ?, ?)
            """, (name, container, signature, docstring, file_path, repo_key, sym_id))

            # Add callees
            if callees:
                caller_full = f"{container}::{name}" if container else name
                for callee in callees:
                    conn.execute("""
                        INSERT INTO symbol_calls (repo, caller_symbol_id, caller_name, caller_file, callee_name)
                        VALUES (?, ?, ?, ?, ?)
                    """, (repo_key, sym_id, caller_full, file_path, callee))

            return sym_id

    def insert_imports(self, repo: str, file_path: str, imports: List[str]):
        if not imports:
            return
        repo_key = repo.lower()
        with self._get_conn() as conn:
            conn.executemany("""
                INSERT INTO file_imports (repo, file_path, statement)
                VALUES (?, ?, ?)
            """, [(repo_key, file_path, imp) for imp in imports])

    def insert_chunks(self, chunks: List[SemanticChunk]):
        if not chunks:
            return
        with self._get_conn() as conn:
            conn.executemany("""
                INSERT INTO chunks (repo, file_path, symbol_name, kind, context_header, content, docstring, start_line, end_line)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            """, [
                (
                    c.repo.lower(), c.file_path, c.symbol_name, c.kind,
                    c.context_header, c.content, c.docstring, c.start_line, c.end_line
                ) for c in chunks
            ])

    def search_symbols(self, query: str, repo: Optional[str] = None, limit: int = 40) -> List[Dict[str, Any]]:
        with self._get_conn() as conn:
            clean_q = query.replace('"', '""')
            if repo:
                sql = """
                    SELECT s.* FROM symbols s
                    JOIN symbols_fts ON symbols_fts.symbol_id = s.id
                    WHERE symbols_fts MATCH ? AND s.repo = ?
                    LIMIT ?
                """
                rows = conn.execute(sql, (f'"{clean_q}"*', repo.lower(), limit)).fetchall()
            else:
                sql = """
                    SELECT s.* FROM symbols s
                    JOIN symbols_fts ON symbols_fts.symbol_id = s.id
                    WHERE symbols_fts MATCH ?
                    LIMIT ?
                """
                rows = conn.execute(sql, (f'"{clean_q}"*', limit)).fetchall()
            
            # Fallback to LIKE if FTS yielded few results
            if len(rows) < 5:
                like_pat = f"%{query}%"
                if repo:
                    sql_like = """
                        SELECT * FROM symbols
                        WHERE repo = ? AND (name LIKE ? OR signature LIKE ?)
                        LIMIT ?
                    """
                    extra = conn.execute(sql_like, (repo.lower(), like_pat, like_pat, limit)).fetchall()
                else:
                    sql_like = """
                        SELECT * FROM symbols
                        WHERE name LIKE ? OR signature LIKE ?
                        LIMIT ?
                    """
                    extra = conn.execute(sql_like, (like_pat, like_pat, limit)).fetchall()
                
                seen_ids = {r["id"] for r in rows}
                for r in extra:
                    if r["id"] not in seen_ids:
                        rows.append(r)

            return [dict(r) for r in rows[:limit]]

    def get_symbol_by_name(self, name: str, repo: Optional[str] = None) -> List[Dict[str, Any]]:
        with self._get_conn() as conn:
            if repo:
                rows = conn.execute("""
                    SELECT * FROM symbols WHERE repo = ? AND name = ?
                """, (repo.lower(), name)).fetchall()
            else:
                rows = conn.execute("""
                    SELECT * FROM symbols WHERE name = ?
                """, (name,)).fetchall()
            return [dict(r) for r in rows]

    def get_symbols_in_file(self, repo: str, file_path: str) -> List[Dict[str, Any]]:
        with self._get_conn() as conn:
            rows = conn.execute("""
                SELECT * FROM symbols
                WHERE repo = ? AND file_path = ?
                ORDER BY start_line ASC
            """, (repo.lower(), file_path)).fetchall()
            return [dict(r) for r in rows]

    def get_callers_of(self, symbol_name: str, repo: Optional[str] = None) -> List[Dict[str, Any]]:
        with self._get_conn() as conn:
            # Matches symbol_name or Container::symbol_name
            pattern = f"%{symbol_name}"
            if repo:
                rows = conn.execute("""
                    SELECT DISTINCT caller_name, caller_file, callee_name
                    FROM symbol_calls
                    WHERE repo = ? AND (callee_name = ? OR callee_name LIKE ?)
                """, (repo.lower(), symbol_name, pattern)).fetchall()
            else:
                rows = conn.execute("""
                    SELECT DISTINCT caller_name, caller_file, callee_name, repo
                    FROM symbol_calls
                    WHERE callee_name = ? OR callee_name LIKE ?
                """, (symbol_name, pattern)).fetchall()
            return [dict(r) for r in rows]

    def get_callees_of(self, caller_name: str, repo: Optional[str] = None) -> List[Dict[str, Any]]:
        with self._get_conn() as conn:
            pattern = f"%{caller_name}"
            if repo:
                rows = conn.execute("""
                    SELECT DISTINCT callee_name, caller_file
                    FROM symbol_calls
                    WHERE repo = ? AND (caller_name = ? OR caller_name LIKE ?)
                """, (repo.lower(), caller_name, pattern)).fetchall()
            else:
                rows = conn.execute("""
                    SELECT DISTINCT callee_name, caller_file, repo
                    FROM symbol_calls
                    WHERE caller_name = ? OR caller_name LIKE ?
                """, (caller_name, pattern)).fetchall()
            return [dict(r) for r in rows]

    def get_chunks_for_file(self, repo: str, file_path: str) -> List[Dict[str, Any]]:
        with self._get_conn() as conn:
            rows = conn.execute("""
                SELECT * FROM chunks
                WHERE repo = ? AND file_path = ?
                ORDER BY start_line ASC
            """, (repo.lower(), file_path)).fetchall()
            return [dict(r) for r in rows]

    def get_stats(self) -> Dict[str, Any]:
        with self._get_conn() as conn:
            file_cnt = conn.execute("SELECT COUNT(*) FROM files").fetchone()[0]
            sym_cnt = conn.execute("SELECT COUNT(*) FROM symbols").fetchone()[0]
            call_cnt = conn.execute("SELECT COUNT(*) FROM symbol_calls").fetchone()[0]
            chunk_cnt = conn.execute("SELECT COUNT(*) FROM chunks").fetchone()[0]
            repos = [r[0] for r in conn.execute("SELECT DISTINCT repo FROM files").fetchall()]

            repo_breakdown = {}
            for r in repos:
                fc = conn.execute("SELECT COUNT(*) FROM files WHERE repo = ?", (r,)).fetchone()[0]
                sc = conn.execute("SELECT COUNT(*) FROM symbols WHERE repo = ?", (r,)).fetchone()[0]
                cc = conn.execute("SELECT COUNT(*) FROM symbol_calls WHERE repo = ?", (r,)).fetchone()[0]
                repo_breakdown[r] = {"files": fc, "symbols": sc, "calls": cc}

            return {
                "total_files": file_cnt,
                "total_symbols": sym_cnt,
                "total_calls": call_cnt,
                "total_chunks": chunk_cnt,
                "repos": repo_breakdown
            }
