# Gói Nghiệp Vụ Nghiên Cứu (Research Domain Pack)

> **Status:** Canonical Baseline v4.0 (Horizon 2)  
> **Source:** Phần VI (§19) & Phần VII (§55) Canonical Specification

Research Domain Pack hướng tới việc biến ý định nghiên cứu khoa học và tổng hợp tài liệu chuyên sâu thành các báo cáo có căn cứ vững chắc, loại bỏ hoàn toàn hiện tượng trích dẫn ảo (*hallucinated citations*).

---

## 1. Pipeline Nghiên Cứu Chuyên Sâu (Research Pipeline)

```text
[ 1. Câu hỏi & Phạm vi ]
         │
[ 2. Khám phá nguồn    ] ───> (arXiv, Semantic Scholar, Web API, Zotero)
         │
[ 3. Nạp & Băm nội dung] ───> (Chuẩn hóa PDF/HTML, băm CAS lưu trữ)
         │
[ 4. Trích xuất Claim  ] ───> (Mô hình hóa Khẳng định - Chứng cứ theo schema)
         │
[ 5. Phản biện & Đối chiếu] ─> (Phát hiện mâu thuẫn chéo giữa các nguồn)
         │
[ 6. Xuất Báo Cáo      ] ───> (Markdown Report, Obsidian Vault, BibTeX)
```

---

## 2. Mô Hình Khẳng Định - Chứng Cứ (Claim-Evidence Schema)

Mỗi luận điểm nghiên cứu được số hóa bằng cấu trúc dữ liệu tường minh:

```yaml
schema: "custos.claim.v1"
claim_id: "clm_01J8N8X1Y2Z3"
claim_text: "Fast local judgment models reduce multi-turn agent latency by up to 70%."
claim_type: "empirical"
epistemic_grade: "STRONG_SUPPORT"

evidence:
  - source_id: "src_arxiv_2406_18665"
    title: "RouteLLM: Learning to Route LLMs with Preference Data"
    locator:
      section: "Section 4: Empirical Results"
      page: 7
      char_span: [1240, 1580]
    excerpt_content: "...demonstrating a 70% latency drop when fast heuristics filter straightforward queries..."
    extraction_confidence: 0.94

cross_validation:
  contradictions_found: 0
  independent_sources_count: 3
```

---

## 3. Tích Hợp Markdown & Obsidian Vault

Toàn bộ kết quả nghiên cứu có thể được xuất trực tiếp thành một **Obsidian Vault** cục bộ:
- Các khái niệm được liên kết hai chiều qua cú pháp `[[WikiLinks]]`.
- Mỗi khẳng định liên kết trực tiếp tới file trích dẫn gốc trong thư mục CAS.
- Tương thích hoàn toàn với các phần mềm quản lý trích dẫn như Zotero.
