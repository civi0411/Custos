from dataclasses import dataclass
from typing import List, Optional
from src.indexer.rust_parser import Symbol, ParsedFile

@dataclass
class SemanticChunk:
    repo: str
    file_path: str
    symbol_name: str
    kind: str
    context_header: str
    content: str
    docstring: str
    start_line: int
    end_line: int

class ChunkBuilder:
    def build_chunks(self, repo: str, file_path: str, content: str, parsed: Optional[ParsedFile] = None) -> List[SemanticChunk]:
        lines = content.splitlines()
        chunks: List[SemanticChunk] = []

        if parsed and parsed.symbols:
            for s in parsed.symbols:
                # Extract the symbol lines
                s_idx = max(0, s.start_line - 1)
                e_idx = min(len(lines), s.end_line)
                code_snippet = "\n".join(lines[s_idx:e_idx])

                header_parts = [
                    f"File: {file_path}",
                    f"Language: {parsed.language}",
                ]
                if s.container:
                    header_parts.append(f"Scope: {s.container}")
                header_parts.append(f"Symbol: {s.name} ({s.kind})")
                if s.signature:
                    header_parts.append(f"Signature: {s.signature}")
                if s.docstring:
                    header_parts.append(f"Description: {s.docstring.splitlines()[0]}")

                comment_prefix = "// " if parsed.language == "rust" else "# "
                context_header = "\n".join(f"{comment_prefix}{p}" for p in header_parts)

                chunks.append(SemanticChunk(
                    repo=repo,
                    file_path=file_path,
                    symbol_name=s.name,
                    kind=s.kind,
                    context_header=context_header,
                    content=code_snippet,
                    docstring=s.docstring,
                    start_line=s.start_line,
                    end_line=s.end_line
                ))
        else:
            # Fallback for config/text files: chunk by 60 lines with 10 lines overlap
            chunk_size = 60
            overlap = 10
            i = 0
            while i < len(lines):
                end_i = min(len(lines), i + chunk_size)
                snippet = "\n".join(lines[i:end_i])
                header = f"// File: {file_path} (Lines {i+1}-{end_i})"
                chunks.append(SemanticChunk(
                    repo=repo,
                    file_path=file_path,
                    symbol_name="",
                    kind="file_chunk",
                    context_header=header,
                    content=snippet,
                    docstring="",
                    start_line=i + 1,
                    end_line=end_i
                ))
                if end_i >= len(lines):
                    break
                i += (chunk_size - overlap)

        return chunks
