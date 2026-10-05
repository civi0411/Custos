from pathlib import Path
import time
from typing import List, Optional
from rich.console import Console
from rich.progress import Progress, SpinnerColumn, TextColumn, BarColumn, TaskProgressColumn
from rich.table import Table

from src.workspace import resolve_repo_path, list_repo_files, language_of
from src.indexer.rust_parser import RustParser
from src.indexer.python_parser import PythonParser
from src.indexer.chunk_builder import ChunkBuilder
from src.indexer.index_store import IndexStore

import uuid

console = Console()

class CodebaseBootstrapper:
    def __init__(self, store: Optional[IndexStore] = None):
        self.store = store or IndexStore()
        self.rust_parser = RustParser()
        self.python_parser = PythonParser()
        self.chunk_builder = ChunkBuilder()

    def index_repo(self, repo: str) -> dict:
        start_time = time.time()
        base_path = resolve_repo_path(repo)
        console.print(f"\n[bold cyan]🚀 Starting Blue/Green index generation for:[/] [bold green]{repo.upper()}[/]")
        console.print(f"[dim]Root: {base_path}[/]")

        # 1. Setup isolated staging store for Blue/Green indexing
        staging_id = uuid.uuid4().hex[:8]
        staging_path = self.store.db_path.parent / f".staging_{repo}_{staging_id}.db"
        staging_store = IndexStore(db_path=staging_path)

        # 2. List all files
        all_files = list_repo_files(repo)
        console.print(f"Discovered [bold yellow]{len(all_files)}[/] files to index.")

        indexed_files = 0
        total_symbols = 0
        total_calls = 0
        total_chunks = 0
        skipped_files = 0

        try:
            with Progress(
                SpinnerColumn(),
                TextColumn("[progress.description]{task.description}"),
                BarColumn(),
                TaskProgressColumn(),
                console=console
            ) as progress:
                task = progress.add_task(f"Indexing {repo} (Staging)...", total=len(all_files))

                for rel_path in all_files:
                    full_path = (base_path / rel_path).resolve()
                    lang = language_of(rel_path)
                    progress.update(task, advance=1, description=f"[cyan]{rel_path[:45]:45s}[/]")

                    try:
                        raw_bytes = full_path.read_bytes()
                        raw_text = raw_bytes.decode("utf-8", errors="replace")
                        line_count = len(raw_text.splitlines())
                        size_bytes = len(raw_bytes)

                        # Insert file entry into staging store
                        file_id = staging_store.insert_file(repo, rel_path, lang, line_count, size_bytes)
                        indexed_files += 1

                        parsed = None
                        if lang == "rust":
                            parsed = self.rust_parser.parse(raw_bytes, rel_path)
                        elif lang == "python":
                            parsed = self.python_parser.parse(raw_bytes, rel_path)

                        # Store symbols and calls in staging store
                        if parsed and parsed.symbols:
                            for s in parsed.symbols:
                                staging_store.insert_symbol(
                                    file_id=file_id,
                                    repo=repo,
                                    file_path=rel_path,
                                    name=s.name,
                                    kind=s.kind,
                                    visibility=s.visibility,
                                    container=s.container,
                                    signature=s.signature,
                                    docstring=s.docstring,
                                    start_line=s.start_line,
                                    end_line=s.end_line,
                                    callees=s.callees
                                )
                                total_symbols += 1
                                total_calls += len(s.callees)

                        if parsed and parsed.imports:
                            staging_store.insert_imports(repo, rel_path, parsed.imports)

                        # Build and insert chunks into staging store (skip large test assets > 500KB)
                        if size_bytes < 500_000:
                            chunks = self.chunk_builder.build_chunks(repo, rel_path, raw_text, parsed)
                            staging_store.insert_chunks(chunks)
                            total_chunks += len(chunks)

                    except Exception as e:
                        skipped_files += 1
                        continue

            # 3. Promote staging generation to active database atomically
            console.print("[dim]Atomically promoting staging index to active production store...[/]")
            self.store.swap_from_staging(staging_store, repo)
            console.print("[bold green]✔ Blue/Green promotion complete![/]")

        finally:
            # Clean up staging sqlite files
            for p in [staging_path, Path(f"{staging_path}-wal"), Path(f"{staging_path}-shm")]:
                if p.exists():
                    try:
                        p.unlink()
                    except OSError:
                        pass

        duration = time.time() - start_time

        table = Table(title=f"Index Summary (Blue/Green): {repo.upper()}", border_style="green")
        table.add_column("Metric", style="cyan")
        table.add_column("Value", style="bold yellow")
        table.add_row("Files Indexed", str(indexed_files))
        table.add_row("Symbols Discovered (AST)", str(total_symbols))
        table.add_row("Call Graph References", str(total_calls))
        table.add_row("Semantic Chunks Built", str(total_chunks))
        table.add_row("Skipped/Unparseable Files", str(skipped_files))
        table.add_row("Elapsed Time", f"{duration:.2f}s")
        table.add_row("Database", str(self.store.db_path))

        console.print(table)
        return {
            "repo": repo,
            "indexed_files": indexed_files,
            "symbols": total_symbols,
            "calls": total_calls,
            "chunks": total_chunks,
            "duration": duration
        }


def bootstrap_all():
    bootstrapper = CodebaseBootstrapper()
    bootstrapper.index_repo("custos")
    bootstrapper.index_repo("goose")

if __name__ == "__main__":
    bootstrap_all()
