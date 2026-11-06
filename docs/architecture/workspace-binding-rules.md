# Workspace Binding Rules & Physical Scope Policy

> **Authority:** Custos SADE Master Specification  
> **Topic:** Physical vs. Logical Workspace Bindings across Workbench Lenses

---

## 1. Nguyên Tắc Cốt Lõi (Core Principles)

Trước đây, quy tắc giao diện giả định rằng mọi session làm việc đều phải gắn chặt với một Git worktree. Tuy nhiên, trong mô hình đa workbench (Assistant, Coding, Research, Quant), giả định này không còn chính xác và gây ra xung đột khi người dùng chỉ cần chat hỗ trợ hoặc đọc tài liệu khoa học.

**Quy tắc bất biến mới:**
1. **Coding Workbench (`lens: coding`)**: Bắt buộc gắn với một physical Git repository / worktree khi có thao tác sửa đổi file, tạo patch hoặc commit.
2. **Assistant Workbench (`lens: assistant`)**: Hoạt động ở phạm vi Logical Project hoặc Global; **không bắt buộc** phải khởi tạo fake Git checkout. Thao tác trò chuyện, lập lịch, hoặc tra cứu không yêu cầu worktree.
3. **Research Workbench (`lens: research`)**: Gắn với `NotebookKernelBinding` và thư mục tài liệu (`~/.custos/data/research/papers/`); **không bắt buộc** có Git repository.
4. **Quant Workbench (`lens: quant`)**: Gắn với thư mục lưu trữ dữ liệu thị trường Parquet và cấu hình chiến lược; việc version control chiến lược là tùy chọn, không bắt buộc cho backtest.

---

## 2. Ma Trận Ràng Buộc Vật Lý Theo Workbench

| Workbench Lens | Loại Binding Yêu Cầu | Tài Nguyên Vật Lý Ràng Buộc | Bắt Buộc Git Worktree? |
|---|---|---|:---:|
| **Assistant** | Logical Project Scope | Session journal, user preferences | ❌ Không |
| **Coding** | Physical Worktree Binding | Git checkout, PTY terminal, worktree path | ✅ Có (khi sửa code) |
| **Research** | Compute & Document Binding | Jupyter/Python Kernel, PDF paper store | ❌ Không |
| **Quant** | Analytical Data Binding | Parquet store, DuckDB analytics file | ❌ Không |

---

## 3. Chuyển Đổi Biên Giới (Cross-Workbench Scope Transitions)

Khi người dùng hoặc Agent chuyển đổi giữa các lenses trong cùng một Session:
- **Từ Assistant sang Coding**: Host sẽ kích hoạt việc resolve hoặc yêu cầu người dùng chọn một Workspace/Worktree trước khi Coding Agent thực thi tool sửa file.
- **Từ Coding sang Research**: Research Agent kế thừa artifact code nhưng không bị khóa vào worktree; có thể chạy phân tích tài liệu độc lập.
- **Không nhân bản Transcript**: Toàn bộ lịch sử trao đổi trong Session vẫn duy trì một dòng thời gian duy nhất trong SQLite `session_journal`, không phụ thuộc vào việc workspace có phải là Git hay không.
