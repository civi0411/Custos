import bisect
from dataclasses import dataclass, field
import re
from typing import List, Optional, Set
from tree_sitter import Language, Parser
import tree_sitter_python as tspy
from src.indexer.docstring_extractor import clean_docstring

PYTHON_LANGUAGE = Language(tspy.language())

PY_KEYWORDS = {
    "if", "while", "for", "match", "return", "yield", "raise", "try",
    "except", "finally", "with", "assert", "import", "from", "as",
    "pass", "break", "continue", "global", "nonlocal", "lambda"
}

CALL_PATTERN = re.compile(r'\b([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)*)\s*\(')

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

class PythonParser:
    def __init__(self):
        self.parser = Parser(PYTHON_LANGUAGE)

    def parse(self, source: bytes, file_path: str = "") -> ParsedFile:
        lines = source.decode("utf-8", errors="replace").splitlines()

        line_starts = [0]
        for i, b in enumerate(source):
            if b == 10:
                line_starts.append(i + 1)

        def byte_to_line(byte_offset: int) -> int:
            return bisect.bisect_right(line_starts, byte_offset)

        def extract_calls(sb: int, eb: int) -> List[str]:
            text = source[sb:eb].decode("utf-8", errors="replace")
            calls = []
            seen = set()
            for m in CALL_PATTERN.finditer(text):
                c = m.group(1).strip()
                if c and c not in PY_KEYWORDS and c not in seen and len(c) < 80:
                    seen.add(c)
                    calls.append(c)
            return calls

        def extract_python_doc(body_node) -> str:
            if not body_node:
                return ""
            for child in body_node.named_children:
                if child.type == "expression_statement":
                    for sub in child.named_children:
                        if sub.type == "string":
                            raw = source[sub.start_byte:sub.end_byte].decode("utf-8", errors="replace")
                            return clean_docstring(raw)
                elif child.type in ("comment", "pass_statement"):
                    continue
                else:
                    break
            return ""

        tree = self.parser.parse(source)
        symbols: List[Symbol] = []
        imports: List[str] = []

        queue = [(tree.root_node, "")]
        while queue:
            parent, container = queue.pop(0)
            for item in parent.named_children:
                itype = item.type

                if itype in ("import_statement", "import_from_statement"):
                    imp_text = source[item.start_byte:item.end_byte].decode("utf-8", errors="replace")
                    imports.append(imp_text.strip())

                elif itype == "class_definition":
                    n_node = item.child_by_field_name("name")
                    if n_node:
                        name = source[n_node.start_byte:n_node.end_byte].decode("utf-8")
                        s_line = byte_to_line(item.start_byte)
                        e_line = byte_to_line(item.end_byte)
                        body = item.child_by_field_name("body")
                        header = source[item.start_byte : (body.start_byte if body else item.end_byte)]
                        sig = " ".join(header.decode("utf-8", errors="replace").split())
                        doc = extract_python_doc(body)
                        vis = "private" if name.startswith("_") else "public"

                        symbols.append(Symbol(
                            name=name,
                            kind="class",
                            visibility=vis,
                            container=container,
                            signature=sig,
                            docstring=doc,
                            start_line=s_line,
                            end_line=e_line,
                            callees=[],
                            file_path=file_path
                        ))

                        if body:
                            queue.append((body, name))

                elif itype == "function_definition":
                    n_node = item.child_by_field_name("name")
                    if n_node:
                        name = source[n_node.start_byte:n_node.end_byte].decode("utf-8")
                        s_line = byte_to_line(item.start_byte)
                        e_line = byte_to_line(item.end_byte)
                        body = item.child_by_field_name("body")
                        header = source[item.start_byte : (body.start_byte if body else item.end_byte)]
                        sig = " ".join(header.decode("utf-8", errors="replace").split())
                        doc = extract_python_doc(body)
                        vis = "private" if name.startswith("_") else "public"
                        is_async = header.startswith(b"async")
                        kind = "async def" if is_async else "def"
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

        return ParsedFile(
            file_path=file_path,
            language="python",
            symbols=symbols,
            imports=imports,
            modules=[],
            line_count=len(lines),
            size_bytes=len(source)
        )
