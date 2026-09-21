# 🧑‍💻 Không Gian Làm Việc Của Vĩ (AI Engineer)

> **Vai trò:** Core Architect & AI Engineer  
> **Lãnh địa phụ trách:** `crates/core-domain`, `crates/cognitive-runtime`, `sidecars/python-judgment`, `sidecars/ts-claude-agent`.

---

## 🎯 Trọng Tâm Công Việc
1. **Core Domain:** Thiết kế data models, traits, state transitions và JSON schemas. Đảm bảo `crates/core-domain` luôn là Single Source of Truth, Zero I/O, không phụ thuộc crate nội bộ nào.
2. **Cognitive Control Fabric:** Xây dựng cơ chế ra quyết định 2 tầng:
   - *System 1:* Fast heuristic & risk triage qua Python sidecar.
   - *System 2:* Deep reasoning qua LLM APIs và TypeScript sidecar.
3. **Prompt & Context:** Context compiler, token budgeting, prompt versioning.

---

## 📁 Cấu Trúc Thư Mục Cá Nhân
- `notes/`: Ghi chú kỹ thuật, bản phác thảo schema, ý tưởng prompt, nghiên cứu paper/mô hình AI.
- `reports/`: Báo cáo tiến độ cá nhân theo ngày/tuần để chia sẻ cho Trường review trên nhánh `report`.

---

## 📋 Checklist Hiện Tại
- [ ] Bổ sung các fields cần thiết cho `Task` và `TaskStatus` trong `crates/core-domain`.
- [ ] Định nghĩa payload interface cho Action Request/Response.
