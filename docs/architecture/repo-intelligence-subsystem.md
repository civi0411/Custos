# Codebase Architecture: Repo Intelligence Subsystem

> **Lưu ý:** Đây là tài liệu thiết kế nền tảng (Base Architecture). Tài liệu này sẽ tiếp tục được mở rộng và cập nhật thông qua quá trình research sâu hơn về các công nghệ Indexing, VectorDB, và RAG.

## 1. Triết lý Thiết kế (Design Philosophy)

Trong hệ thống Custos, **Repo Intelligence** không chỉ là một công cụ (Tool) hay một kỹ năng (Skill) đơn lẻ. Nó là một **Hệ thống con (Subsystem)** chuyên trách việc đọc, hiểu và truy vấn mã nguồn có nhận thức về **Snapshot (bản ghi trạng thái)** và **Provenance (nguồn gốc dữ liệu)**.

Các nguyên tắc cốt lõi:
1.  **Source of Truth:** Mã nguồn (Git snapshot) là nguồn sự thật duy nhất. Bất kỳ Index nào (AST, Call Graph, Vector) đều chỉ là **dữ liệu suy ra (derived data)** và có thể bị vứt bỏ/tái tạo bất cứ lúc nào.
2.  **RAG is a Consumer:** RAG (Retrieval-Augmented Generation) chỉ là một cách chọn ngữ cảnh (consumer) của hệ thống này. Không bắt buộc phải có Vector DB mới có Repo Intelligence.
3.  **MCP is an Adapter:** MCP (Model Context Protocol) chỉ là một màng lọc/bề mặt xuất (export surface) tùy chọn để cung cấp năng lực cho các Agent ngoại lai (như Claude Desktop, Goose). Agent nội bộ của Custos phải gọi trực tiếp interface nội bộ (không qua overhead của JSON-RPC).
4.  **Read/Write Segregation:** Repo Intelligence chỉ cung cấp năng lực ĐỌC (Read) và TRUY VẤN (Query). Các hành động sửa code, ghi file, push Git phải đi qua Authority Gateway của Custos.

---

## 2. Kiến trúc 4 Lớp (Four-Tier Architecture)

Subsystem này được phân tách nghiêm ngặt thành 4 lớp độc lập:

| Lớp (Tier) | Nhiệm vụ chính | Có dùng LLM không? |
| :--- | :--- | :--- |
| **1. Repo Index** | Ghi nhận snapshot, parse file, tạo bảng symbol, trích xuất quan hệ (relations), đo lường parser coverage. | Không |
| **2. Repo Query** | Tìm kiếm theo path/text/symbol/reference. Đọc chính xác toạ độ (span), mở rộng lân cận trong giới hạn. | Không bắt buộc |
| **3. Context Compiler**| Chọn lọc kết quả phù hợp, kiểm tra phân quyền (Scope/Privacy), đóng gói Spans vào Token Budget giới hạn. | Có thể (Dùng Reranker) nhưng mặc định là Không. |
| **4. Engineering Agent**| Dùng ContextPack đã được đóng gói để giải thích, điều tra lỗi, lập kế hoạch sửa code. | Có (Model API hoặc Native Agent) |

---

## 3. Bản đồ ánh xạ Crate (Crate Mapping)

Thiết kế này được rải đều và tận dụng tối đa hệ sinh thái Crate hiện tại của Custos:

*   **`custos-domain/`**: Nơi chứa các định nghĩa thuần túy (Metadata): `RepoSnapshotRef`, `SourceSpan`, `QueryResult`. Không chứa thuật toán.
*   **`custos-core/`**: Nơi đặt chốt chặn an ninh. Kiểm tra Scope của repository, Privacy, quyền đọc file trước khi dữ liệu đến tay Agent.
*   **`custos-persistence/`**: Nơi lưu trữ Index dẫn xuất (FTS, symbol/edge tables). Quản lý `index_generation`.
*   **`custos-runtime/` (tại `src/context/repo_intelligence/`)**: Trái tim điều phối. Kết nối luồng: `Snapshot → Index → Query → ContextPack`. Đánh giá các candidate (lexical/structural), xếp hạng (ranking), và phát hiện dữ liệu cũ (stale detection).
*   **`custos-adapters/`**: Chứa Git/Filesystem watcher, tích hợp LSP/SCIP/Tree-sitter. Nơi cấu hình MCP Client (gọi ra) hoặc MCP-facing adapter (phục vụ client ngoài).
*   **`custos-packs/src/engineering/`**: Khai báo các Workflows và Skills (như `repo_explain`, `diagnose`, `impact_analysis`) để Agent (qua 9-Router) có thể sử dụng.

---

## 4. Pipeline Truy vấn Đa tầng (Query Pipeline)

Không phải mọi truy vấn đều cần Vector/Semantic search. Pipeline chuẩn ưu tiên từ nhẹ đến nặng:

1.  **Scope + Snapshot**: Xác định không gian truy vấn an toàn và phiên bản mã nguồn.
2.  **Exact / Lexical Candidates**: Tìm kiếm chính xác (FTS, Regex) -> *Nhanh, rẻ, chính xác tuyệt đối với tên biến.*
3.  **Structural Expansion**: Mở rộng theo cấu trúc cú pháp (Tree-sitter / LSP) nếu Lexical không đủ ngữ cảnh.
4.  **Semantic Rerank (Optional)**: Đánh giá lại độ liên quan bằng mô hình ngữ nghĩa (nếu cần thiết).
5.  **Bounded ContextPack**: Đóng gói lại cho vừa Token Budget.
6.  **Agent Consumption**: Agent sử dụng và đối chiếu lại với Source Anchors.

---

## 5. Nguyên tắc Đảm bảo Độ chính xác (Accuracy Constraints)

Để ngăn chặn LLM ảo giác (Hallucination), mọi API kết quả trả về **BẮT BUỘC** phải đính kèm Metadata rõ ràng:

*   Phân biệt minh bạch độ tin cậy của kết quả:
    *   `text_match`: Chỉ là chuỗi xuất hiện trong file (Regex).
    *   `syntactic_relation`: Parser (Tree-sitter) thấy có vẻ giống.
    *   `resolved_relation`: Compiler/LSP đã xác nhận chính xác.
*   Bắt buộc đính kèm: `worktree_id`, `dirty-file hashes`, `index_generation`, `path + byte/line span`.
*   **Stale Detection:** Nếu file gốc bị sửa đổi (dirty hash thay đổi), các SourceSpan cũ liên quan phải bị đánh dấu là `stale` (hết hạn), không được phép cung cấp làm bằng chứng (evidence) mới cho Agent.

---
*(Phần dưới này dành cho các Research notes bổ sung trong tương lai về Kuzu, Neo4j, CodeRAG-bench, v.v.)*
