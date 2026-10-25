# GPU thuê và host riêng trong Custos

**Trạng thái:** target design đã hiệu đính, chưa allocate GPU/kiểm chứng billing live. Đối chiếu ngày 18-10-2026. Reuse [Colab/Kaggle](remote-compute-colab-kaggle.md), [checkpoint relay](compute-relay-and-heterogeneous-failover.md) và contracts/RemoteFleet hiện có; không thêm crate/runtime/DB riêng.

## 1. Phạm vi và sửa các giả định quan trọng

Host riêng (máy lab/workstation/VM đã thuê) và provider-managed Pod đều là compute resources, không ModelPort và không reasoning agent. Cần tách provider control plane, host connection, execution engine và durable storage.

Các bản nháp trước dùng ví dụ giá/hardware và vòng đời quá đồng nhất; không coi chúng là product promises. Không giữ lại giả định kernel sống qua terminate nhờ network volume: chỉ saved bytes có thể sống, RAM/VRAM/process không được bảo toàn. Không coi terminate pod là hết mọi charge nếu volumes/storage/network còn tồn tại.

| Profile | Quyền mặc định | Lifecycle |
|---|---|---|
| Dedicated SSH host | Probe/read sau connect consent; execute/upload cần scope | Disconnect không shutdown host; lease chỉ cho workspace/job được cấp |
| Adopted rented instance | Account/resource ID + explicit attach scope | Không auto-terminate resource của người khác hoặc đang có workloads ngoài Custos |
| Custos-created on-demand Pod | Allocation có quote/budget/image/storage preview | Track allocation/ready/observation/release riêng, exact provider identity |
| Interruptible/spot capacity | Thêm interruption risk/backup policy | Có thể mất compute bất ngờ; không dựa vào chắc nhận grace signal |
| Serverless execution | Operation-specific timeout/retry/storage contract | Không mặc định phù hợp long training hoặc durable interactive kernel |

MVP đề xuất Dedicated SSH + một provider Pods (Runpod trước, Vast profile sau), không hỗ trợ mọi cloud ngay. Đây là sequencing dựa scope, không xếp hạng giá/chất lượng nhà cung cấp.

## 2. Bốn loại identity không trộn

1. **Connection/account:** auth subject/credentials refs/provider API version.
2. **Compute resource/lease:** instance/pod ID, ownership, region, quote, volume refs, expiry/cleanup policy.
3. **Host/runtime/kernel:** SSH host key/execution host identity, process supervisor/kernel ID và epoch.
4. **Experiment attempt:** recipe/input/config/checkpoint digest, exact job/process identity, metrics/artifacts.

Không dùng IP public hoặc label làm durable identity; IP có thể đổi/reuse. PID cần host+start/boot identity hoặc supervisor job ID, không đủ một số nguyên. RemoteFleet node giúp biết host, không thay cloud allocation ledger hoặc experiment lineage.

## 3. Placement theo các crate hiện hữu

| Owner | Trách nhiệm |
|---|---|
| Domain `remote_fleet.rs`, recipe/execution/notebook/artifact models | Pure identity/state và hardware/environment observations; secret references |
| Core contracts/authority | Provision/execute/upload/release scopes, budget/admission, lease/compute/storage ports tối thiểu |
| Runtime workflow/notebook/experiment | Owned process lifecycle, recovery/handoff, checkpoint coordination qua ports |
| Adapters compute/SSH | Provider calls, SSH/tunnel/files, structured telemetry; một adapter per concrete protocol/profile |
| Adapters storage/training | Immutable checkpoint transfer + engine save/load; không để cloud allocation code tự quyết resume |
| Persistence | Resource/attempt/cleanup observations và projections qua existing single writer |
| Daemon/API/SDK | Composition, typed commands/events/capabilities; no desktop provider keys |
| Research pack | Recipe/eval/reproducibility obligations; không cấp quyền thuê GPU |

Paths target: `custos-adapters/src/compute/{external_ssh,runpod,vastai}.rs`; tận dụng host/SSH mechanisms hiện có sau inventory. Python helper chỉ khi vendor/engine thư viện Python tạo lợi ích thật. Không tạo `runpod_vast` universal module che mọi khác biệt vendor.

Current source có [RemoteHostNode/SshAuthMethod](../../crates/custos-domain/src/remote_fleet.rs); không mặc định `ComputeBackendKind` hoặc `ComputePort` đã tồn tại. Interface cụ thể phải derive từ first vertical slice và update schema fixtures, không paste enum/example trait chưa kiểm caller.

## 4. Dedicated SSH host: luồng an toàn

1. User khai báo host/user/auth và xác nhận host fingerprint qua trust policy; no `StrictHostKeyChecking=no` mặc định.
2. Probe OS, environment, disk, actual accelerator/capabilities bằng structured command/output. SSH online không chứng minh GPU idle hoặc được phép dùng.
3. Chọn explicit workspace root và process/kernel scope; không chọn `~` làm write allowlist toàn host.
4. Upload selected immutable inputs qua existing bounded file transport; confirm hashes trước execute, no shell interpolation raw model content.
5. Batch job chạy dưới supervisor có stable job ID, logs/status records và lifecycle độc lập SSH session.
6. Notebook dùng authenticated loopback Jupyter server/bridge hoặc kernel transport đã conformance, qua SSH tunnel. Không public port mặc định, không serialize tokens vào URL/event log.
7. Disconnect chỉ ngắt control channels. Resume observation theo job/kernel IDs; source process mất phải checkpoint restore, không “reattach” giả namespace.
8. Cleanup chỉ process/files do lease owns; preserve user jobs, input files và artifacts. Shutting down whole host là separate explicit effect.

Telemetry ưu tiên NVML hoặc supported vendor tools; `nvidia-smi --query-gpu ... --format=csv,noheader,nounits` là structured probe khả thi nhưng vẫn validate types/units/timestamp/exit. Driver-reported “CUDA version” không đồng nghĩa installed PyTorch CUDA runtime. Unsupported metric giữ unknown, không fill zero. GPU utilization thấp không chứng minh không có job.

Credential gap cần xử lý khi implement: current SshAuthMethod model có optional passphrase field; remote execution/DB/API serializer phải audit để chuyển secret reference/redact, không copy wholesale RemoteHostNode vào prompts/logs.

## 5. Cloud Pod: control plane khác SSH

Cloud adapter cần operation fidelity: quote/discover capacity, provision, inspect/reconcile, resolve host endpoint, stop nếu supported, terminate, inspect storage và billing/resource state. API version/direction pin theo provider; không assume mọi vendor đều GraphQL hoặc mọi Pod đều SSH.

### Allocation sequence

- User/standing grant chọn account/provider, region/trust class, acceptable GPU/VRAM/count, image digest, data policy, max rate/duration, persistent storage/retention.
- Obtain timestamped quote/capacity snapshot; refresh theo price tolerance trước launch.
- Persist allocation intent và correlation strategy trước API request.
- Normalize returned external ID, reconcile ambiguous timeout trước create lại. Nếu API không idempotent và mapping không xác định thì uncertain, không spawn duplicate.
- Observe ready, actual hardware/network/image, then host identity/auth. Provider running không tự là environment ready.
- Stage exact inputs/checkpoint, validate environment và engine restore; tạo experiment attempt riêng.
- Keep control/billing observations visible kể cả SSH down hoặc Screen đóng.

Không chạy arbitrary setup script từ model với root key. Image mutable tag cần resolved digest khi available; otherwise mark env capture partial. Remote marketplace machine không mặc nhiên đáng tin cho sensitive data: account/region/host policy và encryption/credential restrictions phải pass.

## 6. Storage và lifecycle theo provider, không gom nhãn “persistent”

[Runpod storage docs](https://docs.runpod.io/pods/storage/types) phân biệt container disk, volume disk gắn Pod, network volumes độc lập và các storage profiles khác. Chọn exact supported profile: container disk không là backup; volume gắn Pod không sống qua terminate; independent volume không giữ kernel RAM. Attach/region/availability phải kiểm ở allocation, không giả volume di chuyển xuyên vendor.

[Vast lifecycle](https://github.com/vast-ai/docs/blob/main/guides/instances/manage-instances.mdx) và [pricing](https://github.com/vast-ai/docs/blob/main/guides/pricing.mdx) cũng phân biệt stopped/destroyed và continuing storage costs. Provider-specific capabilities/retention cần conformance; không normalize thành “Stop luôn an toàn, Terminate luôn $0”.

| Command | Requested / observed result cần giữ | Tránh |
|---|---|---|
| Disconnect | Channels detached; resource/job có thể đang chạy | UI ghi stopped |
| Stop job | Target supervisor process status; pending checkpoint state | Kill entire shared Pod |
| Stop instance | Provider-specific stop state, persisted data policy, remaining storage charges | Assume mọi instance cho stop/restart |
| Terminate compute | Exact resource termination confirmed, storage inventory còn gì | Delete attached persistent volume ngầm |
| Delete retained storage | Separate destructive action, dependency/retention checks | Gộp vào close pane |
| Cleanup unresolved | Last observation/error, manual provider link/reconcile action | Xóa local record để hết cảnh báo |

Lease lifecycle và scientific execution status tách riêng. Attempt hoàn thành không chứng minh GPU released. Independent storage phải có backup ngoài failure domain phù hợp giá trị dữ liệu; provider volume không substitute cross-provider checkpoint store.

## 7. Guardrails tài chính thực tế

Không hardcode giá GPU hoặc storage trong UI/spec. Tính compute + setup/idle + retained disk/object storage + transfer + checkpoint overhead; actual billed khác measured elapsed-time estimate. Quote có timestamp/currency/billing granularity/account scope.

Standing grant auto-cleanup cần ownership, max duration, rate/budget, checkpoint/retention policy và operations allowed. Kernel idle không đủ auto-stop: xem tất cả owned training/export/upload/process/kernel leases, grace period và user activity; unknown external workload thì không terminate host mặc định.

Local watchdog: reserve conservative spend, alert/block new work, request admitted stop khi gần threshold. Provider-side expiry/max job duration hoặc independently hosted reaper nếu available mới giảm rủi ro khi desktop offline; mỗi cơ chế phải disclose giới hạn. Không claim hard cap hóa đơn khi API/telemetry delayed hoặc controller không chạy.

At deadline: request checkpoint/commit nếu còn grace và policy cho phép; attempt/job stop/release sau quyết định retention. Nếu imminent budget stop có thể phải đánh đổi newest state để chặn spend: show chọn last committed checkpoint, không hứa always save-first. Không để upload bị kẹt giữ paid GPU vô hạn; bounded cleanup deadline và unresolved attention.

## 8. Handoff liên provider và retention

Dùng [checkpoint relay design](compute-relay-and-heterogeneous-failover.md), không một backup script riêng per cloud. Source safe-save → external committed manifest → target compatibility → new attempt restore/canary → bounded source cleanup.

Copy filesystem volume không tự resume optimizer/RNG/data state. New GPU ít VRAM/different precision/sharding phải compatibility gate và recipe revision nếu thay semantics. No target phù hợp thì wait/export/warm-start có explicit choice, không tự thuê máy đắt hơn.

Source unknown alive có thể tạo double billing/duplicate writes; destination dùng isolated output prefix/new writer generation. Authoritative lineage chọn branch, không xóa source evidence. Source resource release và storage retention có status riêng cho human inspect.

## 9. API và desktop projection

Reuse Local API command/event envelopes, existing resource host/Research Runs/Notebook/Artifacts/Connections. New concrete DTOs freeze bằng fixtures sau contract audit; no raw provider SDK trong renderer.

Expose capabilities theo resource: probe, allocate/adopt, execute/attach, checkpoint eligibility, reconcile, disconnect, stop-job, supported instance stop/terminate, collect artifacts và storage deletion. Unsupported có reason; provisioning/SSH offline/auth required/storage unresolved đều nhìn thấy.

Projection hiển thị resource/account/ownership/actual GPU, cost estimate timestamp, active jobs/last observation, last remotely protected checkpoint, release status và retained storage. Không dashboard mới hay permanent global cost gauge nếu không có resource liên quan; dùng existing inspector/attention. Status text cần nói rõ còn bị tính phí hay unknown, không chỉ xanh/đỏ.

No worktree giả cho remote research resource; Project/Task/notebook anchors và execution host refs theo [core plan](../development/sade-core-refactor-plan.md), hiệu đính worktree-only rule trong implementation packet.

## 10. Implementation packets

| Packet | Thay đổi | Exit gate |
|---|---|---|
| E0 Contract/source inventory | Existing SSH/fleet/storage APIs, credential serializers, provider version/license/capability map | No fake existing structs, no duplicate control plane |
| E1 Dedicated host supervised job | Trusted SSH identity, bounded upload, durable supervisor ID/status/logs | Reconnect/daemon restart không launch lại; no shared-host cleanup |
| E2 Portable state on host | Checkpoint manifest/relay + engine canary từ F1–F3 | Corrupt/partial denied, remote commit observed |
| E3 One paid provider | Quote/admission/provision/reconcile/inspect/teardown profile | Lost receipt không duplicate spend; actual state shown |
| E4 Durable cleanup | Resource deadlines/provider safeguards + retained storage inventory | Desktop offline/API errors không success giả; leaks visible |
| E5 Cross-provider handoff | Compatible single GPU recipe from source checkpoint | Explicit resume mode, measured rework/cost/quality, source cleanup verified |
| E6 Additional provider/interactive kernel | Vast/others và Jupyter tunnel khi cần | Provider-specific lifecycle + port/secret/epoch tests |

Chưa clone/audit toàn source Runpod/Vast SDK, chưa thuê máy hoặc xác thực giá/hardware/account. Tài liệu official hỗ trợ thiết kế lifecycle/storage, không thay adapter conformance/live smoke. Không add cloud dependency hoặc chạy paid integration từ build/tests thường.

## 11. Sources

- [Runpod storage profiles](https://docs.runpod.io/pods/storage/types), [manage Pods](https://docs.runpod.io/pods/manage-pods), [maintenance/data safety](https://docs.runpod.io/pods/maintenance-and-outages).
- [Vast instance lifecycle source docs](https://github.com/vast-ai/docs/blob/main/guides/instances/manage-instances.mdx), [pricing source docs](https://github.com/vast-ai/docs/blob/main/guides/pricing.mdx).
- Custos [RemoteFleet domain](../../crates/custos-domain/src/remote_fleet.rs), [Recipe](../../crates/custos-domain/src/recipe.rs), [ExecutionRecord](../../crates/custos-domain/src/execution_record.rs): implementation inputs, không proof cloud adapter hiện hữu.
- [Checkpoint relay](compute-relay-and-heterogeneous-failover.md) và [compute source study](../../../TRAINING_COMPUTE_SOURCE_STUDY.md) chốt resume/provenance/failure semantics.

**Chốt:** mua compute và bảo toàn training state là hai vấn đề khác. Có nhiều provider là useful flexibility chỉ khi checkpoint portable, permissions/cost boundaries và resource release được chứng minh.
