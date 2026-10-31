<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../assets/banner-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="../assets/banner.png">
  <img alt="Custos" src="../assets/banner.png" width="100%">
</picture>

[ EN ](../../README.md) · [ VI ](README.vi.md) · [ DE ](README.de.md) · [ ZH ](README.zh.md) · [ JA ](README.ja.md) · [ KO ](README.ko.md) · [ ES ](README.es.md)

</div>

> **감독형 에이전트 개발 환경 (SADE) — 로컬 우선, 증거 바인딩, 인간 주도.**

Custos는 소프트웨어 엔지니어링, 과학 연구 및 개인 워크플로 어시스턴스를 포괄하는 자율 에이전트를 위한 로컬 우선(Local-first) 개발 및 실행 환경입니다. 단순한 LLM 래퍼나 불투명한 멀티 에이전트 시스템과 달리, Custos는 결정론적 상태 지속성, 암호학적으로 감사되는 기능 게이트, 출처가 보장된 컨텍스트 컴파일, 그리고 객관적인 실증 증거를 요구하여 작업 완료를 철저히 검증합니다.

---

## 핵심 동기 및 해결 과제

현대의 에이전트 시스템은 강력한 생성 능력을 갖추었으나, 구조적으로 치명적인 결함을 내포하고 있습니다:

- **휘발성 컨텍스트 손실:** 컨텍스트가 특정 벤더의 폐쇄적인 저장소에 갇혀 도구 전환이나 세션 재시작 시 완전히 소실됩니다.
- **검증되지 않은 완료 선언:** 객관적인 검증이나 재현 가능한 증거 없이 모델 자체의 완료 주장만 맹목적으로 신뢰됩니다.
- **통제되지 않는 외부 부작용:** 파일 수정, 네트워크 요청, 시스템 명령과 같은 변형 작업이 엄격한 권한 제어나 감사 로그 없이 직접 실행됩니다.
- **취약한 상태 관리:** 시스템 충돌, 요청 제한, 컨텍스트 초과 발생 시 진행 중인 작업이 복구 수단 없이 파괴됩니다.
- **인간 주권의 침해:** 불투명한 멀티 에이전트 루프로 인해 인간의 감독권이 약화되고 중요한 의사결정 시점을 박탈당합니다.

Custos는 실증 증거에 기반한 실행 모델을 도입하여 인간의 주권을 최우선으로 보장하며 에이전트의 모든 외부 작업을 엄격하게 통제합니다.

---

## 타협할 수 없는 3가지 핵심 기둥

| 기둥 | 핵심 원칙 | 아키텍처 보증 |
|---|---|---|
| **P1 — 태스크의 1급 객체화** | 태스크는 세션이나 연결과 독립적으로 지속됨 | SQLite WAL 영구 저장소, 결정론적 충돌 복구, 멱등성 명령 원장을 통한 중복 실행 방지. |
| **P2 — 제안 ≠ 결정 ≠ 부작용** | 모델이 제안하고, 커널이 승인하며, 게이트웨이가 실행함 | 신뢰할 수 없는 AI 모델은 `ActionIntent`만 발행하고, 커널이 1회용 `ExecutionPermit`을 서명하며, 샌드박스가 Outbox를 통해 기능 게이트를 강제함. |
| **P3 — 객관적 증거 기반 완료** | 실증 증거 없이는 완료로 인정하지 않음 | 검수 조건은 독립적인 객관적 아티팩트(Diff 패치, 테스트 실행 추적, 인용 검증)가 증명해야 통과하며, 허위 완료를 원천 차단함. |

---

## 운영 메커니즘

```mermaid
flowchart LR
    Human[인간 운영자] --> Session[워크벤치 세션]
    Session --> Task[지속성 태스크]
    Task --> Context[컨텍스트 컴파일]
    Context --> Worker[모델 / Harness Worker]
    Worker --> Intent[동작 의도 ActionIntent]
    Intent --> Gate[권한 게이트 및 허가증]
    Gate --> Effect[샌드박스 내부 안전한 부작용]
    Effect --> Evidence[객관적 실증 Evidence]
    Evidence --> Outcome[검증된 완료 상태]
```

---

## 3가지 특화 도메인 렌즈

Custos는 서로 다른 인지 영역에 최적화된 상호작용 모델과 검증 엔진을 제공합니다:

### 1. 엔지니어링 렌즈 (Engineering Lens - 코딩 워크벤치)
- **3경로 실행 워크스페이스 (3-Path Workspaces):** 표준 저장소(canonical repo), 스테이징 포크(staging fork), 검증 샌드박스를 안전하게 격리.
- **원자적 패치 번들 (Atomic Patch Bundles):** 스테이징 전 구조화된 Diff에 대해 구문 검사 및 자동 Lint 검증을 강제.
- **작업 트리 임대 격리 (Worktree Lease Isolation):** 동일한 작업 트리에 대한 동시 쓰기 작업을 펜싱하여 충돌 및 데이터 손실 방지.

### 2. 리서치 렌즈 (Research Lens - 연구 워크벤치)
- **주장 근거 계층화 (Claim Grounding Hierarchy):** 4단계 엄밀성 체계 확립: `L0 미검증`, `L1 인용 뒷받침`, `L2 동료 평가 통과`, `L3 실험적 재현`.
- **인용 신뢰도 오라클 (Citation Coverage Oracle):** 기출판 논문의 DOI/문헌과 대조 검증하여 모델의 자의적 완료 주장 기각.
- **합성 스튜디오 (Synthesis Studio):** 다중 문헌 조사 결과를 검증 가능한 가설과 재현 가능한 실험 배치로 변환.

### 3. 어시스턴트 렌즈 (Assistant Lens - 개인 비서 및 공유 워크벤치)
- **발신함 승인 큐 (Outbox Approval Queue):** 외부 통신 및 주요 변경 작업을 지속성 대기 승인 큐 뒤에 안전하게 격리.
- **상시 위임 권한 (Standing Grants):** 정기적인 루틴 작업에 대해 정책 기반 위임을 허용하면서 고위험 작업에서는 인간 승인 보장.
- **통합 실행 환경:** 샌드박스 브라우저, PTY 터미널, 원격 서버 플릿, 지속성 노트에 대한 안전한 통합 접근.

---

## 10가지 절대 불변 시스템 규칙 (Invariants)

1. **INV-01 (로컬 우선 주권):** 모든 상태, 계약, 태스크 및 원격 측정 로그는 로컬 디스크에만 저장됩니다.
2. **INV-02 (가짜 원자성 배제):** 모든 상태 변경은 외부 작업이 발송되기 전에 SQLite에 먼저 커밋되어야 합니다.
3. **INV-03 (부작용 전 허가증 발급):** 암호학적으로 바인딩된 `ExecutionPermit` 없이는 어떠한 외부 변경도 실행될 수 없습니다.
4. **INV-04 (실행자 신원 강제 바인딩):** 행위자(Actor) 신원은 인증된 전송 계층에서 바인딩되며, 페이로드의 자체 주장을 신뢰하지 않습니다.
5. **INV-05 (모델 주장보다 실증 우선):** 태스크 완료는 검증 가능한 객관적 증거를 필요로 하며, 모델의 단순 완료 주장을 신뢰하지 않습니다.
6. **INV-06 (UI 기능 진실성):** 백엔드 기능이 누락되거나 성능이 저하된 경우, UI는 즉시 안전한 차단 상태(`unavailable`/`degraded`)로 전환됩니다.
7. **INV-07 (작업 트리 임대 격리):** 동일한 작업 트리(Worktree)에 대한 동시 쓰기 작업은 데이터 손상을 막기 위해 격리 펜싱됩니다.
8. **INV-08 (엄격한 리소스 한계):** 프로세스 실행, 줄 단위 버퍼링 및 토큰 정리는 엄격한 바이트 한도를 준수합니다.
9. **INV-09 (전체 경로 감사 전송):** 모든 상태 변경은 다이제스트 검증과 재생 공격 방지를 갖춘 멱등성 Command Ledger를 통과합니다.
10. **INV-10 (인간의 최고 통제권):** 인간 운영자는 태스크 수명 주기의 어느 단계에서든 무조건 일시 중지, 방향 전환 또는 중단할 수 있는 권한을 보유합니다.

---

## 워크스페이스 크레이트 및 아키텍처 토폴로지

Custos는 순수 도메인 로직과 외부 부작용을 계층화된 Cargo 패키지로 엄격히 분리합니다:

| 계층 | 크레이트 경로 | 패키지 이름 | 핵심 책임 |
|---|---|---|---|
| **Layer 0 (Domain)** | `crates/custos-domain` | `custos-domain` | 순수 도메인 엔티티, 데이터 모델, 상태 머신 및 Zero-I/O 불변 규칙. |
| **Layer 1 (Core)** | `crates/custos-core` | `custos-core` | 태스크 오케스트레이션 서비스, 권한 중재 엔진 및 완료 검증기. |
| | `crates/custos-persistence` | `custos-persistence` | SQLite WAL 영구 저장소, Outbox 패턴, CAS `state_version` 상태 전이 및 장애 복구. |
| | `crates/custos-provider` | `custos-provider` | 공급자 중립적 `ModelPort` 계약, 스트리밍 SSE 파서 및 토큰 계량기. |
| **Layer 2 (Runtime & Adapters)** | `crates/custos-bridge` | `custos-bridge` | 세션에서 태스크로의 승격(Session-to-Task), 실행 중 조향(Steering), IPC 라우팅. |
| | `crates/custos-runtime` | `custos-runtime` | S1/S2 인지 루프, 컨텍스트 압축(RTK), 계층형 메모리 시스템 및 실행 플래너. |
| | `crates/custos-adapters` | `custos-adapters` | OS 샌드박스, MCP 클라이언트 어댑터, 로컬 추론 Sidecar, Git 작업 트리 및 프로토콜 변환. |
| **Layer 3 (Domain Packs)** | `crates/custos-packs` | `custos-packs` | 전문 도메인 워크플로: 엔지니어링(Engineering), 리서치(Research), 어시스턴트(Assistant). |
| **Layer 4 (Composition & Hosts)** | `crates/custos-daemon` | `custos-daemon` | 유일한 조합 루트(Composition Root), 백그라운드 데몬 서버 및 로컬 HTTP/JSON-RPC. |
| | `crates/custos-sdk` | `custos-sdk` | 클라이언트 데이터 전송 객체(DTO), UniFFI 바인딩 및 ACP 전송 타입. |
| | `crates/custos-app/cli` | `custos-app-cli` | `ratatui` 기반의 터미널 운영자 인터페이스. |
| | `crates/custos-app/desktop` | `custos-app-desktop` | `ui/desktop` 프론트엔드를 통합하는 네이티브 Tauri 2 호스트 셸. |

---

## 데스크톱 사용자 인터페이스

Custos Desktop은 **Vite, React 19 및 Tauri 2** 기반으로 제작되었습니다:

- **3계층 계층 구조:** `프로젝트 -> 워크스페이스 -> 작업 트리 (Project -> Workspace -> Worktree)` 직관적 탐색.
- **Graphite JetBlack 디자인 토큰:** 장시간 집중 작업에 최적화된 고대비 모던 다크 테마.
- **전용 워크벤치 (Workbenches):**
  - **엔지니어링 (Engineering):** 파일 및 변경 사항, 객관적 증거, 작업 트리 관리자.
  - **리서치 (Research):** 합성 스튜디오, 연구 발견, 실행 원장.
  - **어시스턴트 및 공용 (Assistant & Shared):** 샌드박스 브라우저, 터미널, 원격 서버 플릿, 영구 메모.
- **Zero-Mock 정책:** UI의 모든 지표는 데몬의 실제 원격 측정을 반영하며 오류 발생 시 안전하게 차단 모드로 진입합니다.

---

## 프로토콜 허브 및 네이티브 하네스 감독

Custos 데몬은 6개의 전문 허브를 통해 시스템 간 상호 운용성을 중앙에서 제어합니다:

- **Model Hub:** 로컬 추론 엔진(GGUF, llama.cpp)과 클라우드 모델(Anthropic, OpenAI, Gemini, Bedrock) 간 라우팅 및 자동 장애 복구.
- **Tool Hub:** Model Context Protocol (MCP) 클라이언트 및 샌드박스 시스템 도구 거버넌스.
- **Agent Hub:** 에이전트 간 통신(A2A) 및 에이전트 인터랙션 프로토콜(ACP) 조정.
- **Storage Hub:** SQLite WAL 영구 저장, 대용량 Blob 파일 및 구조화된 인덱스 관리.
- **Event Hub:** 고처리량 감사 원격 측정 및 라이프사이클 이벤트 분배.
- **Operator Hub:** CLI 및 데스크톱 프론트엔드를 위한 인증된 로컬 보안 엔드포인트 제공.

### 네이티브 하네스 감독
Custos는 기존 코딩 하네스의 내부 로직을 침해하지 않고 외부에서 감시 및 보호합니다:
- **Claude Code:** Stdio 서브프로세스 격리 및 안전한 명령어 래핑.
- **Codex:** 샌드박스화된 코드 완성 및 테스트 자동 실행.
- **Cursor:** 편집기 버퍼 동기화 및 차분 패치 안전 적용.
- **Antigravity:** 에이전트 코딩 런타임 조율 및 태스크 상태 동기화.

---

## 검증 및 품질 게이트

모든 코드 기여는 반드시 다음 품질 게이트를 통과해야 합니다:

```bash
# 1. Rust Clippy 경고 0건, 에러 0건의 엄격한 게이트
cargo clippy --workspace --all-targets -- -D warnings

# 2. 전체 워크스페이스 테스트 스위트 (>1,670개 단위/통합 테스트, 100% 통과)
cargo test --workspace --all-targets

# 3. 데스크톱 UI 타입 검사 및 빌드 검증
cd ui/desktop && npm run build
```

---

## 문서 및 참조

- **표준 아키텍처 명세서:** [Custos.md](../../Custos.md)
- **에이전트 및 기여자 행동 강령:** [AGENTS.md](../../AGENTS.md)
- **데스크톱 UI 설계 표준:** [.agents/rules/desktop_ui_architecture_standards.md](../../.agents/rules/desktop_ui_architecture_standards.md)
- **Git PR 워크플로:** [.agents/rules/git_pr_workflow.md](../../.agents/rules/git_pr_workflow.md)
- **다국어 문서 목차:** [docs/i18n/README.md](README.md)

---

## 라이선스

Custos는 **GNU AGPL-3.0-or-later** 라이선스로 배포됩니다. Custos 브랜드를 보호하고 비공개 상업용 SaaS 래퍼로의 무단 전용을 금지하는 엄격한 상표권 조항을 포함합니다. 상세 내용은 [LICENSE](../../LICENSE)를 참조하십시오.
