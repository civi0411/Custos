# Custos Documentation Internationalization (i18n)

This directory hosts localized editions of the Custos project documentation and repository overviews.

Custos maintains a strict standard of engineering and academic rigor across all localized documentation: translations must preserve architectural fidelity, mermaid diagrams, contract tables, and cross-reference links without visual clutter or loose embellishments.

---

## Available Translations

| Language | Code | Document Link | Status | Canonical Source |
|---|---|---|---|---|
| **English** | `en` | [Main README](../../README.md) | Canonical Source of Truth | Root `README.md` |
| **Tiếng Việt** | `vi` | [README.vi.md](README.vi.md) | Fully Synchronized | Root `README.md` |
| **Deutsch** | `de` | [README.de.md](README.de.md) | Fully Synchronized | Root `README.md` |
| **简体中文** | `zh-CN` | [README.zh.md](README.zh.md) | Fully Synchronized | Root `README.md` |

---

## Canonical Terminology Glossary

To ensure cross-language precision and avoid semantic drift across translations, contributors must adhere to the standardized domain terminology below:

| English (Canonical) | Tiếng Việt | Deutsch | 简体中文 |
|---|---|---|---|
| **Durable Task** | Tác vụ bền vững | Persistente Aufgabe | 持久化任务 |
| **Task Kernel** | Kernel tác vụ | Task Kernel | 任务内核 |
| **Capability Gateway** | Cổng năng lực | Capability Gateway | 能力网关 |
| **Model Gateway** | Cổng mô hình | Model Gateway | 模型网关 |
| **Protocol Gateway** | Cổng giao thức | Protocol Gateway | 协议网关 |
| **Human Principal** | Chủ thể con người | Menschlicher Prinzipal | 人类主权者 |
| **Action Intent** | Ý định hành động | Handlungsabsicht | 动作意图 |
| **Execution Permit** | Giấy phép thực thi | Ausführungsschein | 执行许可凭证 |
| **Receipt** | Biên nhận thực thi | Ausführungsbeleg | 执行回执 |
| **Artifact** | Tạo phẩm | Artefakt | 产物 |
| **Evidence** | Bằng chứng kiểm chứng | Evidenz / Nachweis | 客观实证 |
| **Continuation Packet**| Gói tiếp nối | Fortsetzungspaket | 续行报文 |
| **Domain Pack** | Gói miền chuyên biệt | Domänenpaket | 领域总成 |
| **Outcome Bundle** | Gói kết quả tổng thể | Ergebnisbündel | 成果总成 |
| **Local-First** | Ưu tiên cục bộ | Local-First (Lokal als Standard) | 本地优先 |
| **Human-Governed** | Do con người làm chủ | Vom Menschen gesteuert | 人类主导 / 受人治理 |

---

## Translation Guidelines

When adding or revising a translation:

1. **Source of Truth:** All translations derive from the canonical root `README.md` and `Custos.md`.
2. **Diagram Integrity:** Mermaid diagrams must remain syntactically identical; only node labels and participant names may be translated. Never alter sequence flows.
3. **No Decorative Clutter:** Maintain a professional engineering tone. Do not introduce decorative emojis, non-standard badge styles, version badges, or informal prose.
4. **Relative Links:** Update relative paths appropriately:
   - Root document links (`LICENSE`, `crates/`, `Custos.md`, `AGENTS.md`) point back to `../../`.
   - Documentation links (`docs/README.md`, etc.) point back to `../`.
5. **Language Switcher Bar:** Ensure the top language switcher bar includes all currently available languages in consistent order:
   ```markdown
   [ English ](../../README.md) · [ Tiếng Việt ](README.vi.md) · [ Deutsch ](README.de.md) · [ 简体中文 ](README.zh.md)
   ```
