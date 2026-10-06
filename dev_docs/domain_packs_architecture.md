# Kiến trúc Domain Packs: Research, Engineering (Coding), và Assistant

Trong Custos, hệ thống được phân chia trách nhiệm rạch ròi thành 3 domain packs (`custos-packs`) chính: **Research**, **Engineering** (Coding), và **Assistant**. Việc phân tách này giúp đảm bảo nguyên tắc đặc quyền tối thiểu (Least Privilege), phân định rõ giới hạn (Boundary), và ngăn chặn những tác vụ không lường trước (Side-effects). 

Dưới đây là nghiên cứu và phân tích sâu sắc về 3 thành phần này:

---

## 1. Engineering Pack (Coding)

**Vai trò chính:** 
Thao tác, phân tích, và thay đổi trực tiếp trên mã nguồn (Codebase). Đây là pack duy nhất có quyền tạo ra các thay đổi đối với kho lưu trữ (Write Access) thông qua các công cụ quản lý phiên bản và hệ thống kiểm thử.

**Đặc điểm & Nhiệm vụ:**
- **Codebase Intelligence:** Index toàn bộ AST (Abstract Syntax Tree), phân giải Symbol, và phân tích đồ thị gọi hàm (Call Graph).
- **Autonomous Git Operations:** Tạo patch, chuyển đổi nhánh, tự động áp dụng (apply patch), và tạo git diff.
- **Test Runner & Regression Engine:** Thực thi các unit/integration test để tự động kiểm chứng xem bản vá (patch) có giải quyết được lỗi và không phá vỡ logic cũ hay không.
- **Refactoring:** Chuyển đổi an toàn trên AST và đảm bảo sự tuân thủ các quy tắc lint/code quality.

**Skills tiêu biểu (`custos-packs/src/engineering/skills.rs`):**
- `AstSearchSkill`, `LexicalSearchSkill`: Đọc và tìm kiếm ngữ cảnh mã nguồn.
- `GitPatchSkill`: Thay đổi trực tiếp trên files thông qua git diff/patch.
- `CargoTestSkill`: Đánh giá trạng thái thành công/thất bại sau khi build/test.

---

## 2. Research Pack

**Vai trò chính:**
Thu thập, trích xuất, và xác thực dữ liệu từ các nguồn tài liệu, thông tin tri thức (như tài liệu học thuật, articles). Pack này mang tính chất **Read-only** và hướng tới sự đảm bảo độ tin cậy của thông tin (chống ảo giác).

**Đặc điểm & Nhiệm vụ:**
- **Literature Fetch:** Kết nối có cấu trúc với các APIs học thuật và nghiên cứu bên ngoài (ArXiv, OpenAlex, Europe PMC...).
- **Claim & Fact Extraction:** Trích xuất các khẳng định (claims), bằng chứng, và định vị dữ liệu từ bên trong văn bản dài.
- **Citation Verification:** Xác thực chéo các trích dẫn để đảm bảo AI không sinh ra thông tin giả tạo (hallucination).
- **ReadingCard Synthesis:** Tổng hợp các bản báo cáo tài liệu (ReadingCard) bằng Markdown được đính kèm các điểm gốc bằng chứng (Evidence Anchors).

**Skills tiêu biểu (`custos-packs/src/research/skills.rs`):**
- `ClaimExtractSkill`: Trích xuất thông tin trọng tâm.
- `DoiVerifySkill`: Xác thực DOI (Digital Object Identifier) trên các nền tảng khoa học.

---

## 3. Assistant Pack

**Vai trò chính:**
Đóng vai trò là lớp Gateway, xử lý giao tiếp, định tuyến người dùng và quản trị quy trình làm việc (Workflow/Triage). Đóng gói và chuẩn bị thông tin để hiển thị cho con người nhưng không thực hiện thay đổi cấu trúc mã nguồn.

**Đặc điểm & Nhiệm vụ:**
- **Workspace Triage:** Tóm tắt thông báo và email, không có bất kỳ side-effects nào mà chưa được cấp phép.
- **Draft Response Generation:** Soạn thảo các email/tin nhắn nháp, đề xuất hướng xử lý.
- **Meeting & Agenda Management:** Trích xuất các Action Item từ lịch trình và ghi chú cuộc họp.
- **Contextual Linking:** Liên kết tài liệu văn bản (docs) và ngữ cảnh công việc.

**Skills tiêu biểu (`custos-packs/src/assistant/skills.rs`):**
- `PayloadLockSkill`: Khóa các payload trước khi chia sẻ/xuất phát nhằm đảm bảo an toàn.
- `ContactResolveSkill`: Nhận diện và phân giải danh bạ/định danh giao tiếp.

---

## So sánh Đối chiếu (Boundary Mapping)

| Thuộc tính | Research Pack | Engineering (Coding) Pack | Assistant Pack |
|---|---|---|---|
| **Input** | Tài liệu học thuật, Khẳng định (Claims), URL, DOI. | Source code, Lỗi (Errors), Issues, Ast Trees. | Lịch họp, Email, Tin nhắn, Tasks. |
| **Output** | Facts đã được xác thực, Báo cáo (ReadingCards), Trích dẫn đúng. | Pull Requests, Git Patches, AST Transformations, Kết quả Test. | Draft emails, Action Items, Tóm tắt ngữ cảnh. |
| **Quyền (Privilege)** | Chỉ Đọc (Read-only), fetch dữ liệu internet (HTTP Fetch). | Đọc/Ghi (Read/Write) vào Source Code, Chạy mã (Execution/Tests). | Đọc/Ghi nháp (Draft), Truy xuất Workspace Data, Định tuyến. |
| **Mục tiêu cốt lõi** | Chống Ảo giác (Anti-Hallucination). | Đảm bảo Code hoạt động (Verification & Fix). | Tối ưu hóa Giao tiếp & An toàn Workflow (Safety Triage). |

## Tích hợp vào Custos Architecture

Sự phân chia 3 Pack này hoạt động chặt chẽ với:
1. **Repo Intelligence (Đã triển khai):** Cung cấp dữ liệu tĩnh an toàn, Blue/Green index. *Engineering Pack* sử dụng để tra cứu AST, trong khi *Assistant* dùng để trả lời câu hỏi ngữ cảnh dự án.
2. **Context Compiler & OI (Orchestration Intelligence):** Dựa vào `pack_id`, OI Planner (trong `Benchmark`) sẽ cấp phát *Topology* thực thi tương ứng:
   - Với Engineering: Sẽ dùng topology *T4Worktree* hoặc *T6RepairLoop* để cách ly và sửa chữa mã lỗi.
   - Với Research/Assistant: Có thể chỉ dùng *T3ReadFanOut* hoặc *T0Direct* vì tính chất song song, không thay đổi gốc (zero side-effect). 
