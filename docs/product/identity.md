# Bản Sắc Sản Phẩm & Định Vị (Product Identity & Thesis)

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần I (§0-1) Canonical Specification

---

## 1. Custos Là Gì?

**Custos là một Human-Centered Agentic Work Runtime — local-first, model-agnostic — biến ý định con người thành công việc có trạng thái bền vững, có bằng chứng, có thể resume và đổi provider.**

Custos **không phải** là:
- Không phải "siêu agent" tự xưng toàn năng.
- Không phải chat wrapper gom nhiều API model.
- Không phải một thư viện framework trừu tượng hóa cồng kềnh.

Custos là **lớp runtime chuyên biệt** đứng giữa:
- **Con người (Human Operator):** Người nắm giữ ý định, giá trị và quyền phê duyệt.
- **AI Providers:** OpenAI Codex, Anthropic Claude, Google Antigravity, Local SLM/LLM.
- **Không gian tri thức & Mã nguồn:** Git repositories, workspace, tài liệu cá nhân.
- **Công cụ & Môi trường thực thi:** Shell, linters, test runners, API dịch vụ.
- **Judgment Fabric (System One):** Hệ thống phản xạ phán đoán nhanh, kiểm soát rủi ro.

---

## 2. Product Thesis & Công Thức Hệ Thống

> ### 🎯 Tuyên ngôn giá trị (Product Thesis)
> **Custos turns human intent into controlled, resumable, cross-provider and evidence-backed work.**  
> *(Custos biến ý định của con người thành công việc được kiểm soát, có khả năng phục hồi, độc lập nhà cung cấp và được bảo đảm bằng chứng cứ).*

### Đơn vị trung tâm: Task
Đơn vị trung tâm là **Task**, không phải là phiên chat, agent, model hay tool call.

```text
Human intent
→ Task contract
→ Decision cases and workflow
→ Workers + tools + models
→ Artifacts + evidence
→ Human-governed outcome
```

### Công thức hệ thống
$$	ext{Custos} = 	ext{Durable Task Control} + 	ext{Cognitive Control Fabric} + 	ext{Capability-Governed Execution} + 	ext{Knowledge/Evidence Fabric} + 	ext{Human Sovereignty}$$

---

## 3. Ba Trụ Cột Nền Tảng (Three Pillars)

| Trụ cột | Bản chất kỹ thuật | Giá trị mang lại cho người dùng |
|---|---|---|
| **Model-Agnostic** | Thay não mà không mất hồn (*Task memory & state isolation*) | Đổi từ Claude sang Codex hoặc mô hình Local mà không làm đứt gãy tiến trình hay mất lịch sử ngữ cảnh. |
| **Evidence-Carrying** | Mọi hành động mang theo bằng chứng (*Evidence-Carrying Action*) | Không tin vào lời khẳng định vô căn cứ của AI; chỉ chấp nhận kết quả khi có test pass, diff clean và receipt. |
| **Judgment Infrastructure** | Phán đoán là hạ tầng hạng nhất (*Pluggable System One*) | Tách riêng việc phán đoán nhanh (kiểm tra rủi ro, phân loại) khỏi suy luận sâu của LLM, giảm 60-80% chi phí và độ trễ. |

---

## 4. Sáu Câu Tóm Tắt Kiến Trúc (Architecture in Six Sentences)

1. **Kernel sở hữu sự thật:** Kernel sở hữu state, authority, budget và commit; model chỉ là bộ tính toán suy luận tạm thời.
2. **Workers sinh diệt linh hoạt:** Workers theo vai trò được sinh ra theo nhu cầu của subtask và kết thúc ngay khi hoàn thành scope, không giữ state vĩnh viễn.
3. **Phán đoán kiểm soát suy luận:** System One (Judgment Fabric) bao quanh và kiểm soát System Two (Deliberation LLMs) bằng các hợp đồng quyết định tường minh.
4. **Không có quyền hạn ngầm định:** Không một dòng lệnh shell hay API call nào được thực thi nếu không có giấy phép `ExecutionPermit` hợp lệ qua Capability Gateway.
5. **Nghiệm thu bằng chứng cứ:** Một Task chỉ chuyển sang `Completed` khi vượt qua cổng kiểm tra chứng cứ khách quan (*Completion Gate*) quy định trong Task Contract.
6. **Chủ quyền tuyệt đối của con người:** Con người nắm giữ ngân sách chú ý (*Human Attention Budget*), phê duyệt các quyết định có rủi ro cao thông qua diff và payload cụ thể (*Exact-Payload Approval*).

---

## 5. Đối Tượng Người Dùng & Jobs-To-Be-Done (JTBD)

### Đối tượng mục tiêu (Target Personas)
- **Staff+ Software Engineer / Tech Lead:** Cần giải quyết các refactoring phức tạp kéo dài nhiều giờ trên repo lớn, đòi hỏi cô lập worktree, test kỹ lưỡng và không làm hỏng branch đang làm việc.
- **Deep Researcher / Phân tích viên:** Cần tổng hợp hàng chục tài liệu kỹ thuật, xây dựng bảng đối chiếu claim-evidence có nguồn gốc rõ ràng, không chấp nhận ảo giác.
- **Technical Operator / Power User:** Cần tự động hóa các quy trình hàng ngày (lọc email, tổng hợp lịch, cập nhật ticket) với sự an tâm rằng AI không tự ý gửi thư hoặc xóa dữ liệu ngoài ý muốn.

### Jobs-To-Be-Done (JTBD)
- Khi tôi bắt đầu một tác vụ kỹ thuật phức tạp kéo dài qua đêm, tôi muốn runtime tự động chạy, gặp lỗi thì thử lại hoặc tạm dừng chờ tôi mà không bị mất dấu hay biến mất tiến trình.
- Khi tôi giao việc sửa lỗi mã nguồn cho AI, tôi muốn nhận lại một bản vá (*patch*) sạch kèm báo cáo test pass và lint pass, chứ không phải một đoạn giải thích suông trong khung chat.
- Khi chi phí API tăng cao, tôi muốn chuyển tác vụ sang model rẻ hơn hoặc local SLM cho các bước cơ bản mà không phải thiết lập lại từ đầu.

---

## 6. Những Gì Custos Không Hướng Tới (Non-Goals)

Để tập trung nguồn lực xuất sắc vào giá trị cốt lõi, Custos tuyên bố rõ ràng các Non-goals:
- **Không làm Chatbot giải trí:** Custos không thiết kế cho hội thoại vu vơ, tâm sự hay tìm kiếm thông tin chung chung không dẫn đến hành động có kết quả.
- **Không làm No-Code Drag-and-Drop builder cho người không chuyên:** Custos hướng tới người làm kỹ thuật, lập trình viên và chuyên gia tri thức.
- **Không trao quyền tự trị không giới hạn (No Unconstrained Autonomy):** Custos kiên quyết từ chối triết lý "thả rông" cho agent tự do gọi thẻ tín dụng hay tự ý deploy lên production mà không có sự kiểm soát của con người.
- **Không phụ thuộc đám mây kín:** Custos không phải SaaS độc quyền ép buộc gửi toàn bộ mã nguồn về máy chủ trung tâm.
