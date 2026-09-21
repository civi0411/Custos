# Kiến Trúc Kiểm Thử & Ma Trận Độ Bền (Testing Architecture)

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần VI (§36) Canonical Specification

Hệ thống kiểm thử của Custos được xây dựng nhằm bảo đảm tính đúng đắn tuyệt đối của một runtime quản lý công việc và tự động sửa đổi mã nguồn.

---

## 1. Kim Tự Tháp Kiểm Thử (Test Pyramid)

```text
              ▲
             / \     [ 1. Live Provider Canaries ] (~2%)
            /   \    [ 2. E2E Repo Fixture Tests ] (~8%)
           /     \   [ 3. Chaos & Crash Matrix   ] (~15%)
          /       \  [ 4. Contract & Integration ] (~25%)
         /         \ [ 5. Pure Unit & Property   ] (~50%)
        ─────────────
```

1. **Pure Unit & Property Tests:** Kiểm tra logic chuyển đổi trạng thái của Task, chuẩn hóa đường dẫn, tính toán ngân sách bằng thư viện `proptest`.
2. **Contract & Integration Tests:** Kiểm tra tính tương thích ngược của schema, di chuyển dữ liệu (migrations), và các adapter với mock server.
3. **Chaos & Crash Matrix:** Giả lập tắt nguồn đột ngột, ngắt socket, và disk full để kiểm tra khả năng phục hồi của SQLite WAL và Git worktree.
4. **E2E Repo Fixture Tests:** Chạy các bài toán sửa lỗi mã nguồn trọn vẹn trên các repository mẫu ngoại tuyến.
5. **Live Provider Canaries:** Các bài kiểm tra định kỳ có giới hạn với API OpenAI/Anthropic thật để phát hiện sớm các thay đổi giao thức từ nhà cung cấp.

---

## 2. Các Ma Trận Kiểm Thử Quan Trọng (Critical Test Matrices)

### Ma Trận Xử Lý Sự Cố (Crash Matrix)
Mọi điểm chuyển dịch trạng thái đều được kiểm thử với kịch bản tiến trình bị ngắt đột ngột:
- Ngay trước khi commit SQLite transaction.
- Ngay sau khi lệnh trong sandbox chạy xong nhưng trước khi ghi receipt.
- Khi provider ngắt kết nối giữa chừng lúc đang stream token.
- Khi dung lượng ổ đĩa chạm ngưỡng 100%.

### Ma Trận Đường Dẫn Tệp Tin (Path Normalization Matrix)
Kiểm tra khả năng chống tấn công leo thư mục (*Path Traversal*):
- Kiểm tra các đường dẫn chứa symlink trỏ ra ngoài workspace.
- Kiểm tra chuỗi chứa `../`, Unicode normalization lệch chuẩn, hoặc tệp tin bị xóa trong lúc worker đang đọc.
