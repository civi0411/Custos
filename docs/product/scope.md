# Phạm Vi Sản Phẩm & Định Nghĩa Hoàn Thành (Scope & MVP DoD)

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần I (§2) & Phần VI (§38) Canonical Specification

---

## 1. Ma Trận Phạm Vi (Scope Matrix)

| Lĩnh vực | Trong phạm vi (In-Scope) | Ngoài phạm vi (Out-of-Scope) |
|---|---|---|
| **Môi trường vận hành** | Máy trạm cục bộ (macOS, Linux), chạy như local daemon (`custosd`) | Nền tảng SaaS đa người dùng tập trung trên cloud trong giai đoạn đầu. |
| **Quản lý trạng thái** | Bền vững qua SQLite + WAL, Outbox pattern, hồi phục sau crash | Lưu trữ in-memory phân tán yêu cầu cụm Redis/Kafka phức tạp. |
| **Tương tác AI** | Tương thích đa nhà cung cấp (Codex, Claude, Local models) qua adapter | Huấn luyện từ đầu (pre-training) mô hình nền tảng riêng. |
| **Kiểm soát hành động** | Capability Gateway, Worktree cô lập, Exact-payload approvals | Tự động bypass các rào chắn bảo mật hoặc chạy lệnh với quyền root. |
| **Giao diện người dùng** | CLI mạnh mẽ và tiện ích mở rộng VS Code (*First-class clients*) | Ứng dụng di động (Mobile apps) hoặc giao diện web phức tạp. |

---

## 2. Các Chân Trời Phát Hành (Release Horizons)

```text
┌─────────────────────────────────────────────────────────────┐
│ Horizon 1 (H1): Local Engineering Core                     │
│ - Local daemon + SQLite + CLI                               │
│ - Engineering Domain Pack v1 (Bug-fix & Feature slice)     │
│ - Worktree isolation + Tree-sitter + Ripgrep                │
│ - Single ProviderPort (Codex hoặc Claude)                   │
│ - Capability Gateway + Exact-payload approval               │
└──────────────────────────────┬──────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────┐
│ Horizon 2 (H2): Local-First Multi-Domain & Portability      │
│ - Pluggable System One (Deterministic + Local SLM + Jev)    │
│ - Multi-provider switching (Codex <-> Claude <-> Local)     │
│ - VS Code Extension chính thức                              │
│ - Research Domain Pack (Claim-Evidence matrix, Obsidian)    │
│ - Personal Operations Domain Pack (Attention budget)        │
└──────────────────────────────┬──────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────┐
│ Horizon 3 (H3): Workstation Mesh & Team Federation          │
│ - Peer-to-peer workspace synchronization                    │
│ - Selective cross-machine artifact sharing                  │
│ - Remote federation via A2A protocol                        │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Tiêu Chí Hoàn Thành MVP (MVP Definition of Done)

Trước khi công bố phiên bản MVP sẵn sàng cho người dùng thực tế, hệ thống bắt buộc phải vượt qua 100% danh sách kiểm tra sau:

### Core MVP DoD (Bắt buộc cho Alpha)
- [ ] **Sống sót qua sự cố:** Task sống sót sau khi kill daemon (`kill -9`) ở mọi trạng thái có thể resume, tự động tiếp tục chính xác khi khởi động lại.
- [ ] **Quy trình hoàn chỉnh:** Một tác vụ sửa lỗi (*bug-fix*) đi trọn vẹn từ mục tiêu ban đầu đến bản vá (*patch*) được kiểm chứng tự động (test pass).
- [ ] **Kiểm soát 100% tác động:** Không có bất kỳ thay đổi tệp tin, lệnh shell hay kết nối mạng nào diễn ra ngoài Capability Gateway.
- [ ] **Phê duyệt chính xác:** Cơ chế phê duyệt Exact-payload hoạt động: thay đổi nội dung payload sẽ vô hiệu hóa giấy phép cũ ngay lập tức.
- [ ] **Cô lập Git tuyệt đối:** Mọi thay đổi mã nguồn diễn ra trên Git worktree độc lập; không bao giờ làm bẩn nhánh làm việc hiện tại của người dùng.
- [ ] **Truy vết Snapshot:** Trích xuất mã nguồn gắn liền với commit snapshot cụ thể; phát hiện và từ chối nếu snapshot bị cũ (*stale detection*).
- [ ] **ContextPack chuẩn mực:** Gói ngữ cảnh gửi cho model luôn có nhãn nguồn gốc (*provenance*), kiểm soát độ nhạy cảm và tuân thủ ngân sách token.
- [ ] **Độc lập lỗi Provider:** Lỗi mạng hoặc provider sập (500/rate limit) không bao giờ làm hỏng hoặc mất trạng thái Task.
- [ ] **OutcomeBundle đầy đủ:** Gói bàn giao đầu ra chứa toàn bộ diff tệp tin, test receipts, bằng chứng kiểm tra, tổng chi phí và rủi ro còn lại.
- [ ] **Kiểm thử khôi phục thảm họa:** Kịch bản backup, restore và migration cơ sở dữ liệu SQLite được kiểm thử tự động trong CI.
- [ ] **Tài liệu minh bạch:** Hoàn thành tài liệu mô hình đe dọa (*threat model*) và cẩm nang xử lý sự cố (*runbook*).
- [ ] **Trung thực kỹ thuật:** Không có bất kỳ tuyên bố marketing nào vượt quá bằng chứng đo lường thực tế.

### Cognitive Beta DoD (Bổ sung cho đợt thử nghiệm nhận thức)
- [ ] Ba Question Packs chuẩn chạy ở chế độ giám sát ngầm (*shadow/advisory*).
- [ ] Phiên bản model và Question Pack được ghim (*pinned*) và ghi log đầy đủ.
- [ ] Phân định và cân chuẩn riêng biệt giữa các loại phán đoán (Boolean, Choice, Score).
- [ ] Dữ liệu ngoài miền phân phối (OOD) hoặc bất đồng phán đoán được leo thang lên con người kịp thời.
- [ ] Dữ liệu gửi sang Jev hoặc cloud judgment được lọc và khử nhạy cảm (*redaction*) nghiêm ngặt.
- [ ] Tuyệt đối không có bất kỳ đường dẫn logic nào cho phép System One tự cấp giấy phép hành động (`ExecutionPermit`).
- [ ] Chi phí và tỉ lệ thành công của Task được so sánh minh bạch với baseline không có System One.
