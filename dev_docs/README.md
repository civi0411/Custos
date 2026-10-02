# KHÔNG GIAN KỸ THUẬT NỘI BỘ (DEV DOCS)
## (Internal Team Workspace: Vĩ — Trường — Vinh)

> **Mục đích:** Khu vực làm việc nội bộ của 3 anh em core team (Vĩ, Trường, Vinh).  
> **Kiến trúc SSOT:** [`Custos.md`](../Custos.md)  
> **Tài liệu kỹ thuật chính thức:** [`docs/`](../docs/README.md)

Thư mục `dev_docs/` được giữ **tinh gọn tối đa**, chỉ phục vụ 2 nhu cầu thiết thực hàng ngày:
1. **Phân chia công việc:** Bảng phân chia 11 crate, ai làm gì, ranh giới rõ ràng để không dẫm chân lên nhau.
2. **Notes cá nhân:** Mỗi người có một thư mục riêng để lưu ghi chú kỹ thuật, kết quả spike, thử nghiệm hoặc báo cáo tiến độ cá nhân.

---

## 1. Cấu Trúc Thư Mục

```text
dev_docs/
├── README.md                  # Hướng dẫn quy ước nội bộ này
├── TEAM_WORK_ALLOCATION.md    # Phân chia 11 crates & trách nhiệm của 3 người
│
├── vi/                        # Notes, nghiên cứu AI & kiến trúc của Vĩ
│   ├── notes/                 # Ghi chú thuật toán, prompt, AST context
│   └── reports/               # Báo cáo đánh giá, evals
│
├── truong/                    # Notes, kịch bản test lưu trữ & sandbox của Trường
│   ├── notes/                 # Ghi chú SQLite WAL, sandbox macOS/Linux
│   └── reports/               # Báo cáo test crash, benchmark I/O
│
└── vinh/                      # Notes, luồng thực thi & giao diện của Vinh
    ├── notes/                 # Ghi chú Session/Task, CLI ratatui, MCP
    └── reports/               # Báo cáo test E2E client, IPC
```

---

## 2. Phân Chia Phụ Trách Nhanh

| Thành viên | Vai trò | Phụ trách chính (11 Crates) |
|---|---|---|
| **Vĩ** (~60%) | Chief Architect & AI/DS Lead | `custos-domain`, `custos-core`, `custos-provider`, `custos-daemon`, `custos-packs` |
| **Trường** (~20%) | Systems & Persistence Lead | `custos-persistence`, `crates/custos-adapters/src/sandbox`, test crash |
| **Vinh** (~20%) | Runtime & Client Lead | `custos-runtime`, `custos-bridge`, `custos-cli`, `custos-sdk`, MCP adapters |

Chi tiết cụ thể từng crate và cách phối hợp xem tại [**`TEAM_WORK_ALLOCATION.md`**](TEAM_WORK_ALLOCATION.md).

---

## 3. Quy Ước Đơn Giản Cho 3 Anh Em

1. **Không dẫm chân lên nhau:** Crate của ai người đó chủ động viết code và test. Thay đổi interface dùng chung thì hú nhau tiếng trước khi sửa.
2. **Composition Root:** File `crates/custos-daemon/src/main.rs` do Vĩ ráp nối cuối cùng. Trường và Vinh làm các library crates.
3. **Thoải mái ghi chép:** Thư mục cá nhân (`vi/`, `truong/`, `vinh/`) là nơi tự do lưu scratchpad, ghi chú nhanh, nhật ký bug.
4. **Tài liệu chính thức:** Khi tính năng đã chạy ổn định và chốt kiến trúc, hãy viết vào `docs/` để dùng chung lâu dài, không để tài liệu vĩnh cửu tồn đọng rải rác.
