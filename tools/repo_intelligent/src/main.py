import typer
from rich.console import Console
from rich.panel import Panel
from rich.markdown import Markdown
from rich.table import Table

from src.indexer.bootstrap import CodebaseBootstrapper
from src.indexer.index_store import IndexStore
from src.graph.query_engine import QueryEngine
from src.llm_interface.context_pack import ContextPacker
from src.llm_interface.semantic_qa import SemanticQA
from src.tools import tool_read_file
from src.evidence import export_findings_markdown
from src.investigator import Investigator

app = typer.Typer(
    help="Nexus Lens: local AST and call-graph intelligence for Custos and upstream references",
    rich_markup_mode="rich"
)
console = Console()

@app.command()
def index(
    repo: str = typer.Option("custos", "--repo", "-r", help="Repo to index: 'custos', 'goose', or 'all'")
):
    """
    Kích hoạt Bootstrap Indexing AST (Tree-sitter), Call Graph, và Chunks vào SQLite.
    """
    bootstrapper = CodebaseBootstrapper()
    target = repo.lower().strip()
    if target in ("custos", "all"):
        bootstrapper.index_repo("custos")
    if target in ("goose", "all"):
        bootstrapper.index_repo("goose")
    if target not in ("custos", "goose", "all"):
        raise typer.BadParameter("repo must be 'custos', 'goose', or 'all'")


@app.command()
def overview(
    repo: str = typer.Option("custos", "--repo", "-r", help="Tên repo: 'custos' hoặc 'goose'")
):
    """
    Xuất Architectural Overview Context Pack (kích thước, crates, topology, entry points).
    """
    packer = ContextPacker()
    md = packer.pack_overview(repo)
    console.print(Markdown(md))

@app.command()
def symbol(
    name: str = typer.Argument(..., help="Tên struct, function, trait cần tra cứu"),
    repo: str = typer.Option("custos", "--repo", "-r", help="Tên repo: 'custos' hoặc 'goose'")
):
    """
    Đào sâu 360 độ một symbol: code thật, AST signature, docstring, danh sách callers và callees.
    """
    packer = ContextPacker()
    pack = packer.pack_function(repo, name)
    console.print(Markdown(pack))

@app.command()
def callers(
    symbol: str = typer.Argument(..., help="Tên function/method cần tìm ai gọi đến"),
    repo: str = typer.Option("custos", "--repo", "-r", help="Tên repo: 'custos' hoặc 'goose'")
):
    """
    Truy vết toàn bộ các hàm đang GỌI đến symbol này (Inbound Call Graph).
    """
    qe = QueryEngine()
    caller_list = qe.get_callers(symbol, repo)
    if not caller_list:
        console.print(f"[yellow]Không tìm thấy caller nào cho '{symbol}' trong repo '{repo}'.[/]")
        return

    table = Table(title=f"Callers of '{symbol}' in {repo.upper()}", border_style="cyan")
    table.add_column("Caller Function", style="bold green")
    table.add_column("File Location", style="cyan")
    table.add_column("Line", style="yellow")

    for c in caller_list:
        table.add_row(c["caller"], c["file"], str(c.get("start_line", "-")))
    console.print(table)

@app.command()
def callees(
    symbol: str = typer.Argument(..., help="Tên function/method cần tìm nó gọi những hàm nào"),
    repo: str = typer.Option("custos", "--repo", "-r", help="Tên repo: 'custos' hoặc 'goose'")
):
    """
    Truy vết toàn bộ các hàm được GỌI BÊN TRONG symbol này (Outbound Call Graph).
    """
    qe = QueryEngine()
    callee_list = qe.get_callees(symbol, repo)
    if not callee_list:
        console.print(f"[yellow]Không tìm thấy callee nào được gọi bởi '{symbol}' trong repo '{repo}'.[/]")
        return

    table = Table(title=f"Callees invoked by '{symbol}' in {repo.upper()}", border_style="magenta")
    table.add_column("Callee Symbol", style="bold magenta")
    table.add_column("Defined/Referenced in", style="cyan")

    for c in callee_list:
        table.add_row(c["callee"], c.get("file", ""))
    console.print(table)

@app.command()
def flow(
    from_sym: str = typer.Argument(..., help="Entry symbol bắt đầu"),
    to_sym: str = typer.Argument(..., help="Target symbol kết thúc"),
    repo: str = typer.Option("custos", "--repo", "-r", help="Tên repo: 'custos' hoặc 'goose'")
):
    """
    Tìm chuỗi đường dẫn gọi hàm ngắn nhất (Shortest Execution Flow Path) từ A đến B.
    """
    packer = ContextPacker()
    res = packer.pack_flow(repo, from_sym, to_sym)
    console.print(Markdown(res))

@app.command(name="file")
def inspect_file(
    path: str = typer.Argument(..., help="Đường dẫn tương đối của file"),
    repo: str = typer.Option("custos", "--repo", "-r", help="Tên repo: 'custos' hoặc 'goose'")
):
    """
    Mổ xẻ chi tiết từng hàm, struct, trait, dòng code trong file.
    """
    packer = ContextPacker()
    res = packer.pack_file_deep(repo, path)
    console.print(Markdown(res))

@app.command()
def crate(
    name: str = typer.Argument(..., help="Tên crate trong workspace"),
    repo: str = typer.Option("custos", "--repo", "-r", help="Tên repo: 'custos' hoặc 'goose'")
):
    """
    Kiểm tra phụ thuộc, vai trò và symbols xuất xưởng của một Crate.
    """
    packer = ContextPacker()
    res = packer.pack_crate(repo, name)
    console.print(Markdown(res))

@app.command()
def search(
    query: str = typer.Argument(..., help="Từ khóa tìm kiếm"),
    repo: str = typer.Option("custos", "--repo", "-r", help="Tên repo: 'custos' hoặc 'goose'")
):
    """
    Tìm kiếm AST & FTS5 full-text search theo tên symbol, signature, docstring.
    """
    qe = QueryEngine()
    hits = qe.search_symbols(query, repo=repo, limit=25)
    if not hits:
        console.print(f"[yellow]Không tìm thấy symbol nào khớp với '{query}' trong '{repo}'.[/]")
        return

    table = Table(title=f"Search Results for '{query}' in {repo.upper()}", border_style="green")
    table.add_column("Kind", style="yellow")
    table.add_column("Symbol", style="bold green")
    table.add_column("File", style="cyan")
    table.add_column("Lines", style="dim")
    table.add_column("Signature", style="white")

    for h in hits:
        container = f"{h['container']}::" if h.get("container") else ""
        table.add_row(
            h["kind"],
            f"{container}{h['name']}",
            h["file_path"],
            f"L{h['start_line']}-L{h['end_line']}",
            h.get("signature", "")[:50]
        )
    console.print(table)

@app.command()
def stats():
    """
    Hiển thị số liệu thống kê index của cả hai repository.
    """
    store = IndexStore()
    s = store.get_stats()

    table = Table(title="Custos Nexus Index Statistics", border_style="bold cyan")
    table.add_column("Repository", style="bold yellow")
    table.add_column("Files", style="cyan")
    table.add_column("AST Symbols", style="green")
    table.add_column("Call Graph Edges", style="magenta")

    for r, data in s["repos"].items():
        table.add_row(r.upper(), str(data["files"]), str(data["symbols"]), str(data["calls"]))

    table.add_section()
    table.add_row("TOTAL", str(s["total_files"]), str(s["total_symbols"]), str(s["total_calls"]))
    console.print(table)
    console.print(f"[dim]Total semantic chunks stored: {s['total_chunks']:,}[/]")

@app.command()
def read(
    path: str = typer.Argument(..., help="Đường dẫn file"),
    repo: str = typer.Option("custos", "--repo", "-r", help="Tên repo (custos hoặc goose)"),
    start: int = typer.Option(1, "--start", "-s", help="Dòng bắt đầu"),
    end: int = typer.Option(100, "--end", "-e", help="Dòng kết thúc")
):
    """
    Đọc một đoạn mã nguồn trong file.
    """
    res = tool_read_file(repo, path, start, end)
    console.print(res)

@app.command()
def ask(
    question: str = typer.Argument(..., help="Câu hỏi điều tra kiến trúc"),
    repo: str = typer.Option("custos", "--repo", "-r", help="Tên repo cần hỏi"),
    ai: bool = typer.Option(False, "--ai", help="Sử dụng model OpenRouter nếu có API key")
):
    """
    Hỏi đáp điều tra kiến trúc — tự động tổng hợp bằng chứng AST & Call Graph.
    """
    if ai:
        from src.providers import OpenRouterFreeProvider
        try:
            provider = OpenRouterFreeProvider()
            investigator = Investigator(provider=provider)
            final_answer = investigator.investigate(question)
            console.print(Markdown(final_answer))
            return
        except Exception as e:
            console.print(f"[yellow]OpenRouter không khả dụng ({str(e)}), chuyển sang tổng hợp cục bộ...[/]")

    qa = SemanticQA()
    ans = qa.answer_query(question, repo=repo)
    console.print(Markdown(ans))

@app.command()
def findings():
    """
    Xuất danh sách các bằng chứng đã thu thập vào SQLite.
    """
    md = export_findings_markdown()
    console.print(Markdown(md))

@app.command()
def mcp():
    """
    Khởi động Nexus dưới dạng Model Context Protocol (MCP) Server qua stdio (cho Cursor, Claude Desktop, Custos, Goose).
    """
    from src.mcp_server import NexusMcpServer
    server = NexusMcpServer()
    server.run()

if __name__ == "__main__":
    app()
