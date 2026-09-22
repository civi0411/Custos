# Custos Documentation Internationalization (i18n)

This directory hosts localized editions of the Custos project documentation and repository overviews.

Custos maintains a strict standard of engineering and academic rigor across all localized documentation: translations must preserve architectural fidelity, mermaid diagrams, contract tables, and cross-reference links without visual clutter or loose embellishments.

---

## Available Translations

| Language | Code | Document Link | Status | Canonical Source |
|---|---|---|---|---|
| **English** | `en` | [Main README](../../README.md) | Canonical Source of Truth | Root `README.md` |
| **Tiếng Việt** | `vi` | [README.vi.md](README.vi.md) | Fully Synchronized | `README.md` v4.0 |
| **Deutsch** | `de` | [README.de.md](README.de.md) | Fully Synchronized | `README.md` v4.0 |
| **简体中文** | `zh-CN` | [README.zh.md](README.zh.md) | Fully Synchronized | `README.md` v4.0 |

---

## Canonical Terminology Glossary

To ensure cross-language precision and avoid semantic drift across translations, contributors must adhere to the standardized domain terminology below:

| English (Canonical) | Tiếng Việt | Deutsch | 简体中文 |
|---|---|---|---|
| **Durable Task** | Tác vụ bền vững | Persistente Aufgabe (Durable Task) | 持久化任务 (Durable Task) |
| **Task Kernel** | Kernel tác vụ | Task Kernel | 任务内核 (Task Kernel) |
| **Capability Gateway** | Cổng năng lực | Capability Gateway | 能力网关 (Capability Gateway) |
| **Model Gateway** | Cổng mô hình | Model Gateway | 模型网关 (Model Gateway) |
| **Protocol Gateway** | Cổng giao thức | Protocol Gateway | 协议网关 (Protocol Gateway) |
| **Human Principal** | Chủ thể con người | Menschlicher Prinzipal (Human Principal) | 人类主权者 (Human Principal) |
| **System One (S1)** | Hệ thống một (Phán đoán nhanh) | System Eins (Schnelle Beurteilung) | 系统一 (快速决策/初筛) |
| **System Two (S2)** | Hệ thống hai (Tư duy sâu) | System Zwei (Tiefes Schließen) | 系统二 (深度逻辑推演) |
| **Action Intent** | Ý định hành động | Handlungsabsicht (Action Intent) | 动作意图 (Action Intent) |
| **Execution Permit** | Giấy phép thực thi | Ausführungsschein (Execution Permit) | 执行许可凭证 (Execution Permit) |
| **Receipt** | Biên nhận thực thi | Ausführungsbeleg (Receipt) | 执行回执 (Receipt) |
| **Artifact** | Tạo phẩm | Artefakt (Artifact) | 产物 (Artifact) |
| **Evidence** | Bằng chứng kiểm chứng | Evidenz / Nachweis | 客观实证 (Evidence) |
| **Continuation Packet**| Gói tiếp nối | Fortsetzungspaket (Continuation Packet)| 续行报文 (Continuation Packet) |
| **Domain Pack** | Gói miền chuyên biệt | Domänenpaket (Domain Pack) | 领域总成 (Domain Pack) |
| **Outcome Bundle** | Gói kết quả tổng thể | Ergebnisbündel (Outcome Bundle) | 成果总成 (Outcome Bundle) |
| **Local-First** | Ưu tiên cục bộ | Local-First (Lokal als Standard) | 本地优先 (Local-First) |
| **Human-Governed** | Do con người làm chủ | Vom Menschen gesteuert | 人类主导 / 受人治理 |

---

## Translation Guidelines

When adding or revising a translation:

1. **Source of Truth:** All translations derive from the canonical English root `README.md` and `docs/canonical-specification.md`.
2. **Diagram Integrity:** Mermaid diagrams must remain syntactically identical; only node labels and participant names may be translated. Never remove or alter sequence flows.
3. **No Decorative Clutter:** Maintain a professional and academic engineering tone. Do not introduce decorative emojis, non-standard badge styles, or informal prose.
4. **Relative Links:** Update relative paths appropriately:
   - Root document links (`LICENSE`, `crates/`, `apps/`) point back to `../../`.
   - Documentation links (`docs/architecture/`, etc.) point back to `../`.
5. **Language Switcher Bar:** Ensure the top language switcher bar includes all currently available languages in consistent order:
   ```markdown
   [ English ](../../README.md) | [ Tiếng Việt ](README.vi.md) | [ Deutsch ](README.de.md) | [ 简体中文 ](README.zh.md)
   ```
6. **Contribution:** Open a pull request with the file named `README.<lang_code>.md` and register the new language in the [Available Translations](#available-translations) table above.
