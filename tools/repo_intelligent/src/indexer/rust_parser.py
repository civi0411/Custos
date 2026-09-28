import bisect
from dataclasses import dataclass, field
import re
from typing import List, Optional, Set
from tree_sitter import Language, Parser
import tree_sitter_rust as tsrust
from src.indexer.docstring_extractor import extract_leading_rust_docs

RUST_LANGUAGE = Language(tsrust.language())

RUST_KEYWORDS = {
    "if", "while", "for", "loop", "match", "return", "let", "mut",
    "unsafe", "async", "await", "break", "continue", "move"
}

CALL_PATTERN = re.compile(r'\b([a-zA-Z_][a-zA-Z0-9_]*(?:::[a-zA-Z_][a-zA-Z0-9_]*)*)\s*\(')
MACRO_PATTERN = re.compile(r'\b([a-zA-Z_][a-zA-Z0-9_]*!)\s*[\(\[\{]')

@dataclass
class Symbol:
    name: str
    kind: str
    visibility: str
    container: str
    signature: str
    docstring: str
    start_line: int
    end_line: int
    callees: List[str] = field(default_factory=list)
    file_path: str = ""

@dataclass
class ParsedFile:
    file_path: str
    language: str
    symbols: List[Symbol] = field(default_factory=list)
    imports: List[str] = field(default_factory=list)
    modules: List[str] = field(default_factory=list)
    line_count: int = 0
    size_bytes: int = 0

class RustParser:
    def __init__(self):
        self.parser = Parser(RUST_LANGUAGE)

    def parse(self, source: bytes, file_path: str = "") -> ParsedFile:
        lines = source.decode("utf-8", errors="replace").splitlines()
        
        # Build line starts table for safe, ultra-fast byte-to-line lookup
        line_starts = [0]
        for i, b in enumerate(source):
            if b == 10:  # '\n'
                line_starts.append(i + 1)

        def byte_to_line(byte_offset: int) -> int:
            return bisect.bisect_right(line_starts, byte_offset)

        def get_vis(sb: int) -> str:
            prefix = source[sb : sb + 16].strip()
            if prefix.startswith(b"pub(crate)"):
                return "pub(crate)"
            if prefix.startswith(b"pub(super)"):
                return "pub(super)"
            if prefix.startswith(b"pub"):
                return "pub"
            return "private"

        def extract_calls(sb: int, eb: int) -> List[str]:
            text = source[sb:eb].decode("utf-8", errors="replace")
            calls = []
            seen = set()
            for m in CALL_PATTERN.finditer(text):
                c = m.group(1).strip()
                if c and c not in RUST_KEYWORDS and c not in seen and len(c) < 80:
                    seen.add(c)
                    calls.append(c)
            for m in MACRO_PATTERN.finditer(text):
                c = m.group(1).strip()
                if c and c not in seen and len(c) < 40:
                    seen.add(c)
                    calls.append(c)
            return calls

        tree = self.parser.parse(source)
        symbols: List[Symbol] = []
        imports: List[str] = []
        modules: List[str] = []

        queue = [(tree.root_node, "")]
        while queue:
            parent, container = queue.pop(0)
            for item in parent.named_children:
                itype = item.type

                if itype == "use_declaration":
                    arg = item.child_by_field_name("argument")
                    if arg:
                        imports.append(source[arg.start_byte:arg.end_byte].decode("utf-8", errors="replace"))

                elif itype == "mod_item":
                    n_node = item.child_by_field_name("name")
                    mname = source[n_node.start_byte:n_node.end_byte].decode("utf-8") if n_node else ""
                    if mname:
                        modules.append(mname)
                    body = item.child_by_field_name("body")
                    if body:
                        queue.append((body, f"mod {mname}"))

                elif itype == "function_item":
                    n_node = item.child_by_field_name("name")
                    if n_node:
                        name = source[n_node.start_byte:n_node.end_byte].decode("utf-8")
                        s_line = byte_to_line(item.start_byte)
                        e_line = byte_to_line(item.end_byte)
                        vis = get_vis(item.start_byte)
                        body = item.child_by_field_name("body")
                        header = source[item.start_byte : (body.start_byte if body else item.end_byte)]
                        sig = " ".join(header.decode("utf-8", errors="replace").split())
                        doc = extract_leading_rust_docs(lines, s_line - 1)
                        is_async = b"async" in header
                        kind = "async fn" if is_async else "fn"
                        calls = extract_calls(body.start_byte, body.end_byte) if body else []

                        symbols.append(Symbol(
                            name=name,
                            kind=kind,
                            visibility=vis,
                            container=container,
                            signature=sig,
                            docstring=doc,
                            start_line=s_line,
                            end_line=e_line,
                            callees=calls,
                            file_path=file_path
                        ))

                elif itype in ("struct_item", "enum_item", "trait_item", "type_item"):
                    n_node = item.child_by_field_name("name")
                    if n_node:
                        name = source[n_node.start_byte:n_node.end_byte].decode("utf-8")
                        s_line = byte_to_line(item.start_byte)
                        e_line = byte_to_line(item.end_byte)
                        vis = get_vis(item.start_byte)
                        body = item.child_by_field_name("body")
                        header = source[item.start_byte : (body.start_byte if body else item.end_byte)]
                        sig = " ".join(header.decode("utf-8", errors="replace").split())
                        doc = extract_leading_rust_docs(lines, s_line - 1)
                        kind = itype.replace("_item", "")

                        symbols.append(Symbol(
                            name=name,
                            kind=kind,
                            visibility=vis,
                            container=container,
                            signature=sig if sig else f"{kind} {name}",
                            docstring=doc,
                            start_line=s_line,
                            end_line=e_line,
                            callees=[],
                            file_path=file_path
                        ))

                        if itype == "trait_item" and body:
                            queue.append((body, f"trait {name}"))

                elif itype == "impl_item":
                    t_node = item.child_by_field_name("type")
                    tr_node = item.child_by_field_name("trait")
                    type_str = source[t_node.start_byte:t_node.end_byte].decode("utf-8") if t_node else ""
                    trait_str = source[tr_node.start_byte:tr_node.end_byte].decode("utf-8") if tr_node else ""
                    cname = f"{trait_str} for {type_str}" if trait_str else type_str
                    body = item.child_by_field_name("body")
                    if body:
                        queue.append((body, cname))

                elif itype == "macro_definition":
                    n_node = item.child_by_field_name("name")
                    if n_node:
                        mname = source[n_node.start_byte:n_node.end_byte].decode("utf-8")
                        s_line = byte_to_line(item.start_byte)
                        e_line = byte_to_line(item.end_byte)
                        doc = extract_leading_rust_docs(lines, s_line - 1)
                        symbols.append(Symbol(
                            name=mname + "!",
                            kind="macro",
                            visibility="pub",
                            container=container,
                            signature=f"macro_rules! {mname}",
                            docstring=doc,
                            start_line=s_line,
                            end_line=e_line,
                            callees=[],
                            file_path=file_path
                        ))

        return ParsedFile(
            file_path=file_path,
            language="rust",
            symbols=symbols,
            imports=imports,
            modules=modules,
            line_count=len(lines),
            size_bytes=len(source)
        )
