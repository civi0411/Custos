<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../assets/banner-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="../assets/banner.png">
  <img alt="Custos" src="../assets/banner.png" width="100%">
</picture>

[ EN ](../../README.md) · [ VI ](README.vi.md) · [ DE ](README.de.md) · [ ZH ](README.zh.md) · [ JA ](README.ja.md) · [ KO ](README.ko.md) · [ ES ](README.es.md)

</div>

# Custos SADE

> **감독형 에이전트 개발 환경 (SADE) — 로컬 우선, 증거 바인딩, 인간 주도.**

Custos는 코딩, 과학 연구 및 개인 워크플로 어시스턴스를 포괄하는 자율 에이전트를 위한 로컬 우선(Local-first) 개발 및 실행 환경입니다. 단순한 LLM 래퍼나 불투명한 멀티 에이전트 시스템과 달리, Custos는 엄격한 인간 감독, 결정론적 상태 지속성, 암호학적으로 감사되는 기능 게이트, 그리고 객관적인 실증 증거를 요구하여 작업 완료를 철저히 검증합니다.

---

## 타협할 수 없는 3가지 핵심 기둥

| 기둥 | 핵심 원칙 | 아키텍처 보증 |
|---|---|---|
| **P1 — 태스크의 1급 객체화** | 태스크는 세션이나 연결과 독립적으로 지속됨 | SQLite WAL 영구 저장소, 결정론적 충돌 복구, 멱등성 명령 원장을 통한 중복 실행 방지. |
| **P2 — 제안 ≠ 결정 ≠ 부작용** | 모델이 제안하고, 커널이 승인하며, 게이트웨이가 실행함 | AI 모델은 `ActionIntent`만 발행하고, 커널이 1회용 `ExecutionPermit`을 서명하며, 샌드박스가 기능 게이트를 강제함. |
| **P3 — 객관적 증거 기반 완료** | 실증 증거 없이는 완료로 인정하지 않음 | 검수 조건은 독립적인 아티팩트(Diff 패치, 테스트 실행 추적, 인용 검증)가 증명해야 통과하며, 허위 완료 선언을 원천 차단함. |

---

## 10가지 절대 불변 시스템 규칙 (Invariants)

1. **INV-01 (로컬 우선 주권):** 모든 상태, 계약, 태스크 및 원격 측정 로그는 로컬 디스크에만 저장됩니다.
2. **INV-02 (가짜 원자성 배제):** 모든 상태 변경은 외부 작업이 발송되기 전에 SQLite에 먼저 커밋되어야 합니다.
3. **INV-03 (부작용 전 허가증 발급):** 유효한 `ExecutionPermit` 없이는 어떠한 외부 변경도 실행될 수 없습니다.
4. **INV-04 (실행자 신원 강제 바인딩):** 행위자(Actor) 신원은 인증된 전송 계층에서 바인딩되며, 페이로드의 자체 주장을 신뢰하지 않습니다.
5. **INV-05 (모델 주장보다 실증 우선):** 태스크 완료는 검증 가능한 객관적 증거를 필요로 하며, 모델의 단순 완료 주장을 신뢰하지 않습니다.
6. **INV-06 (UI 기능 진실성):** 백엔드 기능이 누락되거나 성능이 저하된 경우, UI는 즉시 안전한 차단 상태(`unavailable`/`degraded`)로 전환됩니다.
7. **INV-07 (작업 트리 임대 격리):** 동일한 작업 트리(Worktree)에 대한 동시 쓰기 작업은 데이터 손상을 막기 위해 격리 펜싱됩니다.
8. **INV-08 (엄격한 리소스 한계):** 프로세스 실행, 줄 단위 버퍼링 및 토큰 정리는 엄격한 바이트 한도를 준수합니다.
9. **INV-09 (감사된 전송 계층):** 모든 시스템 변경은 다이제스트 해시 검증을 거치는 멱등성 명령 원장을 통과해야 합니다.
10. **INV-10 (인간의 최고 통제권):** 인간 운영자는 태스크 수명 주기의 어느 단계에서든 작업을 일시 중지, 조정 또는 중단할 수 있는 절대적 권한을 가집니다.

---

## 운영 메커니즘

```mermaid
flowchart LR
    Human[인간 운영자] --> Session[세션 / 워크벤치]
    Session --> Task[지속성 태스크]
    Task --> Context[컨텍스트 컴파일]
    Context --> Worker[모델 / 하네스 워커]
    Worker --> Intent[작업 제안 (Action Intent)]
    Intent --> Gate[권한 게이트 및 허가증]
    Gate --> Effect[샌드박스 부작용 실행]
    Effect --> Evidence[객관적 실증 증거]
    Evidence --> Outcome[검증된 완료 결과]
```

---

## 시스템 아키텍처: 11개 표준 Rust 크레이트

Custos는 11개의 계층화된 Rust 크레이트를 통해 순수 도메인 로직과 외부 부작용을 명확히 분리합니다:

| 계층 | 크레이트 | 핵심 책임 |
|---|---|---|
| **Layer 0 (Domain)** | `custos-domain` | 순수 도메인 엔티티, 상태 머신, Zero-I/O 시스템 불변 규칙. |
| **Layer 1 (Core)** | `custos-core` | 태스크 오케스트레이션 서비스, 권한 엔진, 완료 검증 정책. |
| | `custos-persistence` | SQLite WAL 저장소, Outbox 패턴, CAS 상태 전이 및 장애 복구. |
| | `custos-provider` | 벤더 중립적 모델 계약, SSE 스트리밍 파서 및 토크나이저. |
| **Layer 2 (Runtime & Bridge)** | `custos-bridge` | 세션을 태스크로 승격, RPC 라우팅 및 로컬 API 발송. |
| | `custos-runtime` | 에이전트 인지 루프, 컨텍스트 압축 (RTK), 메모리 계층 및 플래너. |
| | `custos-adapters` | OS 샌드박스, MCP 클라이언트 어댑터, 로컬 추론 사이드카 및 프로토콜 변환기. |
| **Layer 3 (Domain Packs)** | `custos-packs` | 전문 도메인 워크플로: 엔지니어링, 과학 연구, 어시스턴트 팩. |
| **Layer 4 (Entry & Distribution)** | `custos-daemon` | 유일한 구성 루트 (Composition Root), 백그라운드 데몬 및 로컬 HTTP/JSON-RPC. |
| | `custos-sdk` | 클라이언트 DTO, UniFFI 크로스 바인딩 및 전송 클라이언트. |
| | `custos-cli` | 터미널 TUI 조작 인터페이스 (`ratatui`). |

---

## 데스크톱 UI

Custos 데스크톱 UI는 **Vite, React 19, Tauri 2** 기반으로 제작되었습니다:

- **3계층 계층 구조:** `프로젝트 -> 워크스페이스 -> 작업 트리 (Project -> Workspace -> Worktree)`의 직관적 탐색.
- **Graphite JetBlack 디자인 시스템:** 장시간 개발에 최적화된 고대비 다크 테마.
- **전용 워크벤치:**
  - **엔지니어링:** 파일 및 변경 사항, 실증 증거 뷰어, 작업 트리 관리자.
  - **연구:** 합성 스튜디오, 과학적 발견 보드, 실행 이력 원장.
  - **어시스턴트 및 공유:** 내장 브라우저, 터미널, 원격 함대(Fleet), 노트.
- **모의 데이터 금지 정책 (Zero-Mock Policy):** UI의 모든 지표는 데몬의 실제 원격 측정값을 반영하며, 기능 비활성화 시 명확한 경고를 표시합니다.

---

## 품질 게이트 및 검증

모든 코드 기여는 다음의 엄격한 품질 게이트를 통과해야 합니다:

```bash
# 1. 무경고 Clippy 차단 게이트 (Rust)
cargo clippy --workspace --all-targets -- -D warnings

# 2. 전체 테스트 스위트 (>1,670개 테스트, 100% 통과)
cargo test --workspace --all-targets

# 3. 데스크톱 UI 타입 체크 및 번들 빌드
cd ui/desktop && npm run build
```

---

## 문서 및 참조

- **시스템 아키텍처 사양:** [Custos.md](../../Custos.md)
- **에이전트 및 기여자 규칙:** [AGENTS.md](../../AGENTS.md)
- **데스크톱 UI 표준:** [.agents/rules/desktop_ui_architecture_standards.md](../../.agents/rules/desktop_ui_architecture_standards.md)
- **Git PR 워크플로 규칙:** [.agents/rules/git_pr_workflow.md](../../.agents/rules/git_pr_workflow.md)
- **문서 디렉터리:** [docs/README.md](../README.md)

---

## 라이선스

Custos는 **GNU AGPL-3.0-or-later**에 따라 배포됩니다. 여기에는 Custos 브랜드를 보호하기 위한 엄격한 상표권 고지가 포함되어 있으며, 독점적 상업용 SaaS 래핑을 금지합니다. 자세한 내용은 [LICENSE](../../LICENSE)를 참조하십시오.
