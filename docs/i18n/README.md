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
| **日本語** | `ja` | [README.ja.md](README.ja.md) | Fully Synchronized | Root `README.md` |
| **한국어** | `ko` | [README.ko.md](README.ko.md) | Fully Synchronized | Root `README.md` |
| **Español** | `es` | [README.es.md](README.es.md) | Fully Synchronized | Root `README.md` |

---

## Canonical Terminology Glossary

To ensure cross-language precision and avoid semantic drift across translations, contributors must adhere to the standardized domain terminology below:

| English (Canonical) | Tiếng Việt | Deutsch | 简体中文 | 日本語 | 한국어 | Español |
|---|---|---|---|---|---|---|
| **Durable Task** | Tác vụ bền vững | Persistente Aufgabe | 持久化任务 | 永続タスク | 지속성 태스크 | Tarea persistente |
| **Task Kernel** | Kernel tác vụ | Task Kernel | 任务内核 | タスクカーネル | 태스크 커널 | Núcleo de tareas |
| **Capability Gateway** | Cổng năng lực | Capability Gateway | 能力网关 | ケイパビリティゲートウェイ | 기능 게이트웨이 | Pasarela de capacidad |
| **Model Gateway** | Cổng mô hình | Model Gateway | 模型网关 | モデルゲートウェイ | 모델 게이트웨이 | Pasarela de modelos |
| **Human Principal** | Chủ thể con người | Menschlicher Prinzipal | 人类主权者 | 人間オペレータ | 인간 주권자 | Autoridad humana |
| **Action Intent** | Ý định hành động | Handlungsabsicht | 动作意图 | アクション意図 | 작업 제안 | Intención de acción |
| **Execution Permit** | Giấy phép thực thi | Ausführungsschein | 执行许可凭证 | 実行許可証 | 실행 허가증 | Permiso de ejecución |
| **Evidence** | Bằng chứng kiểm chứng | Evidenz / Nachweis | 客观实证 | 客観的実証 | 객관적 증거 | Evidencia objetiva |
| **Domain Pack** | Gói miền chuyên biệt | Domänenpaket | 领域工作流包 | ドメインパック | 도메인 팩 | Paquete de dominio |
| **Local-First** | Ưu tiên cục bộ | Local-First | 本地优先 | ローカルファースト | 로컬 우선 | Prioridad local |
| **Human-Governed** | Do con người làm chủ | Vom Menschen gesteuert | 人类主导 | 人間主導 | 인간 주도 | Gobernado por humanos |

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
   [ EN ](../../README.md) · [ VI ](README.vi.md) · [ DE ](README.de.md) · [ ZH ](README.zh.md) · [ JA ](README.ja.md) · [ KO ](README.ko.md) · [ ES ](README.es.md)
   ```
