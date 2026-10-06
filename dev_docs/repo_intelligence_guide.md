# Hướng Dẫn Phát Triển: Repo Intelligence Subsystem (Base Implementation)

> **Mục tiêu:** Cung cấp tài liệu kỹ thuật chi tiết về hệ thống Repo Intelligence trong Custos, cấu trúc các module đã triển khai, nguyên tắc an toàn, và định hướng nghiên cứu mở rộng trong tương lai.

---

## 1. Tổng Quan Kiến Trúc 4 Lớp (Four-Tier Architecture)

Repo Intelligence trong Custos được tổ chức theo 4 tầng độc lập để đảm bảo **Source of Truth là Git Worktree**, **chống hallucination cho LLM**, và **phân tách rõ ràng giữa Đọc (Read) và Ghi (Write)**:

```
┌────────────────────────────────────────────────────────┐
│  Tier 4: Engineering Agent & Skills                   │
│  (LexicalSearchSkill, ImpactAnalysis, RepoExplain)     │
└────────────────────────▲───────────────────────────────┘
                         │
┌────────────────────────┴───────────────────────────────┐
│  Tier 3: Context Compiler & Coordinator                │
│  (RepoCoordinator: Stale Check, Token Budget, Ranking) │
└────────────────────────▲───────────────────────────────┘
                         │
┌────────────────────────┴───────────────────────────────┐
│  Tier 2: Security & Scope Filter                      │
│  (RepoSecurityPolicy: Path Scope, Secrets, Traversal)  │
└────────────────────────▲───────────────────────────────┘
                         │
┌────────────────────────┴───────────────────────────────┐
│  Tier 1: Repo Index & Snapshot Identity                │
│  (RepoSnapshotRef, CodeGraph, Blue/Green Nexus SQLite) │
└────────────────────────────────────────────────────────┘
```

---

## 2. Bản Đồ Codebase & Các Thành Phần Đã Triển Khai (Base Implementation)

### 2.1. Crate `custos-domain` (`crates/custos-domain/src/repo/`)
- [snapshot.rs](file:///Users/mac/Project/AgentHub/Custos/crates/custos-domain/src/repo/snapshot.rs):
  - Định nghĩa [`RepoSnapshotRef`](file:///Users/mac/Project/AgentHub/Custos/crates/custos-domain/src/repo/snapshot.rs): Quản lý `repo_id`, `worktree_id`, `head_commit`, và bảng băm `dirty_hashes: BTreeMap<String, String>` (rel_path -> sha256) cùng `index_generation`.
  - Cung cấp hàm `is_clean()` và `is_file_dirty(rel_path, current_hash)`.
- [span.rs](file:///Users/mac/Project/AgentHub/Custos/crates/custos-domain/src/repo/span.rs):
  - Định nghĩa [`SourceSpan`](file:///Users/mac/Project/AgentHub/Custos/crates/custos-domain/src/repo/span.rs): Định vị chính xác đoạn mã với `path`, `start_byte`, `end_byte`, `start_line`, `end_line`, và `source_digest`.
- [query.rs](file:///Users/mac/Project/AgentHub/Custos/crates/custos-domain/src/repo/query.rs):
  - [`ConfidenceLevel`](file:///Users/mac/Project/AgentHub/Custos/crates/custos-domain/src/repo/query.rs): Minh bạch nguồn gốc kết quả (`TextMatch`, `SyntacticCandidate`, `ResolvedCompiler`).
  - [`MissingReason`](file:///Users/mac/Project/AgentHub/Custos/crates/custos-domain/src/repo/query.rs): Giải thích lý do loại trừ (`FileIgnored`, `ParserError`, `TokenBudgetExceeded`, `StaleSource`, `AccessDenied`).
  - [`CoverageMetrics`](file:///Users/mac/Project/AgentHub/Custos/crates/custos-domain/src/repo/query.rs): Định lượng độ hoàn thiện của index (`coverage_ratio`, `has_unindexed_changes`).
  - [`QueryResult<T>`](file:///Users/mac/Project/AgentHub/Custos/crates/custos-domain/src/repo/query.rs): Đóng gói dữ liệu kết quả kèm snapshot và bằng chứng độ tin cậy.

### 2.2. Crate `custos-core` (`crates/custos-core/src/repo/`)
- [security.rs](file:///Users/mac/Project/AgentHub/Custos/crates/custos-core/src/repo/security.rs):
  - Định nghĩa [`RepoSecurityPolicy`](file:///Users/mac/Project/AgentHub/Custos/crates/custos-core/src/repo/security.rs): Chặn đứng mọi hành vi path traversal (`..`), truy cập file nhạy cảm (`.env`, `.git`, `credentials.json`, `id_rsa`), và kiểm soát giới hạn egress bytes/lines trên mỗi `SourceSpan`.

### 2.3. Crate `custos-runtime` (`crates/custos-runtime/src/context/repo_intelligence/`)
- [graph.rs](file:///Users/mac/Project/AgentHub/Custos/crates/custos-runtime/src/context/repo_intelligence/graph.rs):
  - Bổ sung enum [`RelationType`](file:///Users/mac/Project/AgentHub/Custos/crates/custos-runtime/src/context/repo_intelligence/graph.rs) (`TextMatch`, `SyntacticRelation`, `ResolvedRelation`).
  - Cập nhật [`GraphEdge`](file:///Users/mac/Project/AgentHub/Custos/crates/custos-runtime/src/context/repo_intelligence/graph.rs) và [`CodeGraph`](file:///Users/mac/Project/AgentHub/Custos/crates/custos-runtime/src/context/repo_intelligence/graph.rs): Gỡ bỏ các tuyên bố "authoritative index" sai lệch, khẳng định đồ thị chỉ là cache phái sinh (derived heuristic cache).
- [coordinator.rs](file:///Users/mac/Project/AgentHub/Custos/crates/custos-runtime/src/context/repo_intelligence/coordinator.rs):
  - [`RepoCoordinator`](file:///Users/mac/Project/AgentHub/Custos/crates/custos-runtime/src/context/repo_intelligence/coordinator.rs): Điều phối truy vấn đa tầng, bắt `current_snapshot` thời gian thực với `blake3` dirty hash, kiểm tra `is_stale()`, và đóng gói kết quả vào `QueryResult`.

### 2.4. Crate `custos-packs` (`crates/custos-packs/src/engineering/skills/`)
- [ast_search.rs](file:///Users/mac/Project/AgentHub/Custos/crates/custos-packs/src/engineering/skills/ast/ast_search.rs):
  - Refactor thành [`LexicalSearchSkill`](file:///Users/mac/Project/AgentHub/Custos/crates/custos-packs/src/engineering/skills/ast/ast_search.rs) (kèm alias [`AstSearchSkill`](file:///Users/mac/Project/AgentHub/Custos/crates/custos-packs/src/engineering/skills/ast/ast_search.rs) tương thích ngược).
  - Gán nhãn minh bạch `assurance: text_match` và `is_heuristic: true`.

### 2.5. Python Nexus Subsystem (`tools/repo_intelligent/`)
- [workspace.py](file:///Users/mac/Project/AgentHub/Custos/tools/repo_intelligent/src/workspace.py):
  - Chuẩn hoá `Path.resolve()` và kiểm tra `relative_to(base)`. Ngăn chặn hoàn toàn các cuộc tấn công symlink và path traversal ra ngoài thư mục repo.
- [index_store.py](file:///Users/mac/Project/AgentHub/Custos/tools/repo_intelligent/src/indexer/index_store.py):
  - Bổ sung hàm `swap_from_staging()` sử dụng SQLite `ATTACH DATABASE` và single transaction để hoán đổi dữ liệu staging sang production trong 1 thao tác atomic.
- [bootstrap.py](file:///Users/mac/Project/AgentHub/Custos/tools/repo_intelligent/src/indexer/bootstrap.py):
  - Áp dụng cơ chế **Blue/Green Indexing**: Ghi index mới vào cơ sở dữ liệu tạm thời `.staging_{repo}_{uuid}.db`. Chỉ khi index hoàn tất 100% mới swap vào `nexus_index.db`. Nếu có lỗi hoặc bị dừng đột ngột, index chính vẫn nguyên vẹn 100%.
- [mcp_server.py](file:///Users/mac/Project/AgentHub/Custos/tools/repo_intelligent/src/mcp_server.py):
  - Hỗ trợ thương lượng protocol version linh hoạt (`SUPPORTED_PROTOCOL_VERSIONS`).
  - Định dạng kết quả trả về với cấu trúc chặt chẽ gồm `snapshot_id`, `locator`, và `stale_status`.

---

## 3. Quy Trình Mở Rộng & Nghiên Cứu Tiếp Theo (Next Research Topics)

Hệ thống đã có khung Base vững chắc. Khi bạn nghiên cứu thêm, có thể mở rộng vào các module sau:

1. **LSP / SCIP Integration (Tier 1 & 2):**
   - Nghiên cứu cơ chế đọc file `.scip` hoặc kết nối với Rust Analyzer / Pyright qua LSP JSON-RPC.
   - Khi có dữ liệu từ LSP, đánh nhãn `ConfidenceLevel::ResolvedCompiler` thay vì `SyntacticCandidate`.
2. **Tree-sitter WASM Parser (Rust Native):**
   - Đưa parser cú pháp dạng Tree-sitter WASM vào `custos-runtime` để trích xuất AST chính xác hơn regex mà không cần chạy Python backend.
3. **Graph Database & Vector DB (Kuzu / Qdrant):**
   - Kết nối `IndexStore` với Kuzu DB hoặc Vector Database để bổ sung tầng `Semantic Rerank` (Layer 4 trong query pipeline).
   - *Lưu ý:* Bất kể dùng Vector DB nào, đầu ra trả về cho Agent vẫn phải tuân thủ format [`QueryResult<Vec<SourceSpan>>`](file:///Users/mac/Project/AgentHub/Custos/crates/custos-domain/src/repo/query.rs) đã thiết lập ở tầng Domain.
