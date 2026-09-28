import ast
import re
import json
from typing import Dict, Any, List
from src.tools import (
    tool_list_files, tool_read_file, tool_search_text, 
    tool_find_symbol, tool_find_references, tool_get_repo_map
)
from src.evidence import save_finding
from src.repo_map import generate_repo_map
from src.providers import OpenRouterFreeProvider

SYSTEM_PROMPT = """Bạn là Nexus Investigator - một Kiến trúc sư Hệ thống chuyên nội soi mã nguồn của 2 repository Custos và Goose.
Mục tiêu của bạn là trả lời các câu hỏi về kiến trúc và cơ chế hoạt động của 2 repo DỰA TRÊN BẰNG CHỨNG THỰC TẾ TRONG CODE.

BẠN CÓ CÁC CÔNG CỤ (TAY CHÂN TỰ ĐỘNG KHÁM PHÁ):
1. get_repo_map(repo="custos", max_depth=3): Xem bản đồ cấu trúc cây thư mục và các symbols chính.
2. list_files(repo="custos", subpath=""): Liệt kê các file trong thư mục.
3. read_file(repo="custos", path="...", start_line=1, end_line=100): Đọc mã nguồn chi tiết.
4. search_text(repo="custos", query="...", path_filter=""): Tìm kiếm chuỗi/từ khóa.
5. find_symbol(repo="custos", name="..."): Định vị chính xác file và dòng định nghĩa struct, fn, trait, class.
6. find_references(repo="custos", symbol="..."): Truy vết tất cả các nơi đang GỌI hoặc SỬ DỤNG symbol này (truy vết luồng).
7. save_evidence(repo="custos", path="...", start_line=1, end_line=10, topic="...", fact="..."): Lưu bằng chứng vào DB.
8. finish(answer="..."): Trả lời kết luận cuối cùng khi đã đủ bằng chứng.

QUY TẮC BẮT BUỘC:
- KHÔNG ĐOÁN MÒ. Mọi nhận định đều phải đọc code thật.
- Khi tìm kiếm một struct/function/trait cụ thể, hãy dùng ngay `find_symbol(repo="...", name="...")` để định vị tức thì.
- Khi tìm thấy 1 struct/hàm quan trọng, hãy dùng find_references để lần ra ai đang gọi nó và nó đi về đâu!
- Trả lời theo định dạng ReAct chuẩn:
Thought: <suy nghĩ ngắn gọn về bước tiếp theo>
Action: <tên_hàm>(tham_số=giá_trị, ...)

Ví dụ:
Thought: Tôi cần xem bản đồ Goose để định hướng.
Action: get_repo_map(repo="goose", max_depth=3)

Thought: Tôi thấy ExtensionManager, giờ tôi sẽ tìm xem nó được gọi ở đâu.
Action: find_references(repo="goose", symbol="ExtensionManager")
"""

def parse_action(action_str: str) -> tuple[str, list, dict]:
    """Parse linh hoạt cả tham số vị trí lẫn tham số từ khóa."""
    action_str = action_str.strip()
    try:
        tree = ast.parse(action_str)
        expr = tree.body[0].value
        func_name = expr.func.id
        args = [ast.literal_eval(a) for a in expr.args]
        kwargs = {kw.arg: ast.literal_eval(kw.value) for kw in expr.keywords}
        return func_name, args, kwargs
    except Exception:
        # Fallback to regex nếu cú pháp hơi lệch
        match = re.match(r"(\w+)\((.*)\)", action_str, re.DOTALL)
        if match:
            tool_name, raw_args = match.groups()
            kwargs = {}
            for m in re.finditer(r'(\w+)\s*=\s*(?:"([^"]*)"|\'([^\']*)\'|(\d+))', raw_args):
                k, v1, v2, v3 = m.groups()
                kwargs[k] = v1 if v1 is not None else (v2 if v2 is not None else int(v3))
            return tool_name, [], kwargs
        raise ValueError(f"Không thể parse Action: {action_str}")

def execute_action(action_str: str) -> str:
    """Parse và thực thi Action từ chuỗi text của LLM."""
    try:
        tool_name, args, kwargs = parse_action(action_str)
    except Exception as e:
        return f"Error: Cannot parse action '{action_str}': {str(e)}"

    def get_arg(name: str, idx: int, default=None):
        if name in kwargs:
            return kwargs[name]
        if idx < len(args):
            return args[idx]
        return default

    try:
        if tool_name == "get_repo_map":
            return tool_get_repo_map(
                repo=get_arg("repo", 0, "custos"),
                max_depth=get_arg("max_depth", 1, 3)
            )
        elif tool_name == "list_files":
            return tool_list_files(
                repo=get_arg("repo", 0, "custos"),
                subpath=get_arg("subpath", 1, "")
            )
        elif tool_name == "read_file":
            return tool_read_file(
                repo=get_arg("repo", 0, "custos"),
                path=get_arg("path", 1, ""),
                start_line=get_arg("start_line", 2, 1),
                end_line=get_arg("end_line", 3, 100)
            )
        elif tool_name == "search_text":
            return tool_search_text(
                repo=get_arg("repo", 0, "custos"),
                query=get_arg("query", 1, ""),
                path_filter=get_arg("path_filter", 2, "")
            )
        elif tool_name == "find_symbol":
            return tool_find_symbol(
                repo=get_arg("repo", 0, "custos"),
                name=get_arg("name", 1, "")
            )
        elif tool_name == "find_references":
            return tool_find_references(
                repo=get_arg("repo", 0, "custos"),
                symbol=get_arg("symbol", 1, "")
            )
        elif tool_name == "save_evidence":
            finding_id = save_finding(
                repo=get_arg("repo", 0, "custos"),
                file_path=get_arg("path", 1, ""),
                start_line=get_arg("start_line", 2, 1),
                end_line=get_arg("end_line", 3, 1),
                topic=get_arg("topic", 4, "General"),
                fact=get_arg("fact", 5, "")
            )
            return f"Evidence saved successfully with ID #{finding_id}."
        elif tool_name == "finish":
            return get_arg("answer", 0, "Analysis finished.")
        else:
            return f"Error: Unknown tool '{tool_name}'."
    except Exception as e:
        return f"Error executing tool '{tool_name}': {str(e)}"

class Investigator:
    def __init__(self, provider: OpenRouterFreeProvider = None):
        self.provider = provider or OpenRouterFreeProvider()

    def investigate(self, question: str, max_iterations: int = 8) -> str:
        messages = [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": f"Câu hỏi điều tra: {question}\n\nHãy bắt đầu bằng Thought và Action đầu tiên để khám phá mã nguồn."}
        ]

        print(f"\n[bold green]🔍 Bắt đầu điều tra tự động:[/] {question}")
        for step in range(1, max_iterations + 1):
            print(f"\n--- [Vòng {step}/{max_iterations}] ---")
            
            try:
                response = self.provider.chat(messages)
            except Exception as e:
                return f"Lỗi gọi LLM API: {str(e)}"

            print(response.strip())
            messages.append({"role": "assistant", "content": response})

            # Check if finish tool called
            if "finish(" in response:
                match = re.search(r"finish\((?:answer=)?[\"'](.*?)[\"']\)", response, re.DOTALL)
                if match:
                    return match.group(1)
                return response

            # Extract Action
            action_match = re.search(r"Action:\s*([^\n]+)", response)
            if not action_match:
                if len(response.strip()) > 80:
                    return response.strip()
                obs = "Observation: Vui lòng đưa ra Action theo định dạng `Action: tool_name(...)`."
            else:
                action_str = action_match.group(1).strip()
                obs = f"Observation:\n{execute_action(action_str)}"

            print(f"[dim]{obs[:350]}...[/]" if len(obs) > 350 else f"[dim]{obs}[/]")
            messages.append({"role": "user", "content": obs})

        return "Cuộc điều tra kết thúc do đạt giới hạn số vòng lặp tối đa."
