<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../assets/banner-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="../assets/banner.png">
  <img alt="Custos" src="../assets/banner.png" width="100%">
</picture>

[ EN ](../../README.md) · [ VI ](README.vi.md) · [ DE ](README.de.md) · [ ZH ](README.zh.md) · [ JA ](README.ja.md) · [ KO ](README.ko.md) · [ ES ](README.es.md)

</div>

# Custos SADE

> **監視付きエージェント開発環境 (SADE) — ローカルファースト・客観的実証バインド・人間主導。**

Custos は、コーディング、科学的研究、および個人のワークフロー支援を網羅する自律型エージェント向けのローカルファースト開発・実行環境です。単純な LLM ラッパーや不透明なマルチエージェントオーケストレータとは異なり、Custos は厳格な人間による監督、決定論的な状態永続化、暗号学的に監査されたケイパビリティゲート、そして客観的な実証成果を要求し、タスクの完了を厳密に保証します。

---

## 妥協のない3つの基本原則

| 柱 | 中核方針 | アーキテクチャ上の保証 |
|---|---|---|
| **P1 — タスクのファーストクラス化** | タスクはセッションや接続から独立して永続する | SQLite WAL による永続化、決定論的なクラッシュ復旧、冪等なコマンド台帳による二重実行防止。 |
| **P2 — 提案 ≠ 決定 ≠ 副作用** | モデルが提案し、カーネルが許可し、ゲートウェイが実行する | AI モデルは `ActionIntent` のみを提示し、カーネルが単一利用の `ExecutionPermit` を発行。サンドボックスがケイパビリティを制限。 |
| **P3 — 客観的実証に基づく完了判定** | 実証成果なしに完了と認めない | 完了条件の合格には客観的なアーティファクト（差分、テスト実行トレース、文献照合）が不可欠であり、虚偽の完了宣言を排除。 |

---

## 10の不可侵システム不変条件

1. **INV-01 (ローカルファースト主権):** すべての状態、契約、タスク、およびテレメトリはローカルディスクに保存される。
2. **INV-02 (見せかけの原子性の排除):** すべての状態変更は、外部アクションが発行される前に SQLite にコミットされる。
3. **INV-03 (副作用前の許可証発行):** 有効な `ExecutionPermit` なしにいかなる外部変更も実行されない。
4. **INV-04 (実行者アイデンティティの結合):** アクターの識別情報は認証済みトランスポート層でバインドされ、ペイロードの自己申告を信用しない。
5. **INV-05 (モデルの主張より実証を優先):** タスクの検収には検証可能な実証成果が必要であり、モデルの口頭の完了報告を信用しない。
6. **INV-06 (UIケイパビリティの真実性):** バックエンドが利用不可または機能低下している場合、UI は直ちに安全側に倒れる (`unavailable`/`degraded`)。
7. **INV-07 (作業ツリーリースの分離):** 同一の作業ツリーに対する並行書き込みはフェンシングされ、データの破損を防ぐ。
8. **INV-08 (リソース上限の厳守):** プロセス実行、バッファリング、トークン削減は厳格なバイト制限を遵守する。
9. **INV-09 (監査済みトランスポート):** すべての変更操作はコマンド台帳を通過し、ダイジェスト検証によって保護される。
10. **INV-10 (人間の最高統治権):** 人間のオペレータは、タスクのライフサイクルのいかなる段階でも一時停止、方向修正、または中止する絶対的権限を保持する。

---

## 運用メカニズム

```mermaid
flowchart LR
    Human[人間オペレータ] --> Session[セッション / ワークベンチ]
    Session --> Task[永続タスク]
    Task --> Context[コンテキスト生成]
    Context --> Worker[モデル / Harness ワーカー]
    Worker --> Intent[アクション意図 (Action Intent)]
    Intent --> Gate[権限ゲート & 許可証発行]
    Gate --> Effect[サンドボックス副作用実行]
    Effect --> Evidence[客観的実証成果]
    Evidence --> Outcome[検証済み完了]
```

---

## システムアーキテクチャ: 11の規範的 Rust クレート

Custos は、11の階層化された Rust クレートによって純粋なドメインロジックと外部副作用を厳密に分離しています。

| レイヤ | クレート | 主要な責務 |
|---|---|---|
| **Layer 0 (Domain)** | `custos-domain` | 純粋なドメインエンティティ、状態機械、Zero-I/O 不変条件。 |
| **Layer 1 (Core)** | `custos-core` | タスクオーケストレーション、権限エンジン、完了検証ポリシー。 |
| | `custos-persistence` | SQLite WAL ストレージ、Outbox パターン、CAS 状態遷移、クラッシュリカバリ。 |
| | `custos-provider` | プロバイダ非依存のモデル契約、SSE ストリーミング解析、トークナイザ。 |
| **Layer 2 (Runtime & Bridge)** | `custos-bridge` | セッションからタスクへの昇格、RPC ルーティング、ローカル API 配信。 |
| | `custos-runtime` | エージェント認知ループ、コンテキスト圧縮 (RTK)、メモリ階層、実行プランナ。 |
| | `custos-adapters` | OS サンドボックス、MCP クライアント、ローカル推論サイドカー、ワイヤ変換器。 |
| **Layer 3 (Domain Packs)** | `custos-packs` | 専門業務ワークフロー: エンジニアリング、科学研究、アシスタントパック。 |
| **Layer 4 (Entry & Distribution)** | `custos-daemon` | 唯一の Composition Root、バックグラウンドデーモンサービス、ローカル HTTP/JSON-RPC。 |
| | `custos-sdk` | クライアント DTO、UniFFI バインディング、トランスポートクライアント。 |
| | `custos-cli` | ターミナル TUI 操作インターフェース (`ratatui`)。 |

---

## デスクトップ UI

Custos デスクトップ UI は **Vite、React 19、Tauri 2** で構築されています。

- **3層階層構造:** `Project -> Workspace -> Worktree` による直感的なナビゲーション。
- **Graphite JetBlack トークン:** 長時間の集中作業に最適化された高コントラストなダークテーマ。
- **専用ワークベンチ:**
  - **エンジニアリング:** ファイルと変更点、実証証拠ビューア、作業ツリーマネージャ。
  - **リサーチ:** 統合スタジオ、発見ダッシュボード、実行履歴レジャー。
  - **アシスタント & 共有:** 組み込みブラウザ、ターミナル、リモートフリート、ノート。
- **モック排除ポリシー (Zero-Mock Policy):** UI 上のすべてのメトリクスは実際のデーモンデータを反映し、機能不全時は明示的に警告を表示。

---

## 品質ゲートとテスト

すべてのコード変更は以下の厳格な品質ゲートを通過する必要があります。

```bash
# 1. 警告ゼロの Clippy ブロッキングゲート (Rust)
cargo clippy --workspace --all-targets -- -D warnings

# 2. 全テストスイート (>1,670 テスト、100% 合格)
cargo test --workspace --all-targets

# 3. デスクトップ UI の型チェックおよびビルド
cd ui/desktop && npm run build
```

---

## ドキュメントと参照先

- **アーキテクチャ標準仕様:** [Custos.md](../../Custos.md)
- **エージェントおよび開発者ルール:** [AGENTS.md](../../AGENTS.md)
- **デスクトップ UI 規約:** [.agents/rules/desktop_ui_architecture_standards.md](../../.agents/rules/desktop_ui_architecture_standards.md)
- **Git PR フロー:** [.agents/rules/git_pr_workflow.md](../../.agents/rules/git_pr_workflow.md)
- **ドキュメントディレクトリ:** [docs/README.md](../README.md)

---

## ライセンス

Custos は **GNU AGPL-3.0-or-later** の下で提供されています。商標保護条項が含まれており、プロプライエタリな商用 SaaS による再パッケージ化を禁じています。詳細は [LICENSE](../../LICENSE) をご参照ください。
