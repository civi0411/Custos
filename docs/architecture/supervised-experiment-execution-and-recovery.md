# Supervised Experiment Execution & Recovery (Custos F10 Canonical Specification)

> **Phân hệ**: Research Studio — Compute, Experiment Lifecycle & Checkpoint Recovery  
> **Trạng thái**: Đặc tả kiến trúc chính thức (Canonical Architecture Specification)  
> **Nguyên tắc**: Bền vững (Durable), Giám sát có kiểm soát (Supervised), Minh bạch về mức độ phục hồi (Honest Recovery).

---

## 1. Bản chất sản phẩm và Tôn chỉ điều hành

### 1.1 Bản chất sản phẩm
**Supervised Experiment Execution & Recovery** là hệ thống giúp Custos:
1. Chuẩn bị (Preflight & Environment).
2. Khởi chạy và phân bổ (Submit & Allocate).
3. Quan sát và đối soát (Observe & Reconcile).
4. Bảo vệ tiến trình bền vững (Durable Checkpoint Commitment).
5. Phục hồi và chuyển tiếp (Recovery & Handoff).

Hệ thống phục vụ 3 luồng công việc độc lập:
- **Interactive Notebook**: Đọc dữ liệu, thực thi cell, gỡ lỗi, vẽ biểu đồ.
- **Background Experiment / Training**: Tác vụ dài hạn có danh tính công việc (`job_id`), số liệu đo lường (`metrics`), tạo tác phẩm (`artifacts`) và đánh giá kiểm định (`held-out evaluation`).
- **Recovery / Handoff**: Tạo lượt thử mới (`Attempt`) từ checkpoint hợp lệ khi máy nguồn bị gián đoạn hoặc đổi target tính toán.

> [!IMPORTANT]
> Một notebook có thể kích hoạt tiến trình huấn luyện, nhưng tiến trình huấn luyện **tuyệt đối không được sống/chết theo tab notebook hoặc kết nối của trình duyệt/laptop**.

### 1.2 Bỏ các giả định sai và cam kết quá mạnh
- **Không hứa hẹn Live Migration VRAM**: Không cam kết di chuyển tiến trình hay bộ nhớ RAM/VRAM giữa hai GPU khác kiến trúc mà không mất bước. Custos cam kết: *Phục hồi an toàn và chính xác từ checkpoint bền vững gần nhất (`latest_protected_checkpoint_step`).*
- **Không kiểm tra Optimizer Norm = 0**: Một số optimizer hoặc tham số đặc thù (như SGD không momentum) không lưu tensors trong state dict; từ chối dựa vào norm = 0 sẽ làm sai lệch kiểm tra tính hợp lệ.
- **Không coi Batch đầu trùng là lỗi**: Hash batch đầu của attempt mới có thể trùng hoặc khác tuỳ theo data cursor và epoch; phải kiểm tra con trỏ dữ liệu (`dataset_cursor`) thực sự của engine.
- **Không coi Loss tăng > 5x là hỏng**: Đây chỉ là heuristic cảnh báo, không phải oracle toán học chung cho mọi domain/loss landscape.
- **Fencing thực sự**: Tăng epoch/generation trong SQLite local chỉ có ý nghĩa khi nơi tiếp nhận commit và lưu trữ kiểm tra generation đó. Fencing phải được thực thi ở tầng Storage (Namespace / Prefix Isolation), không chỉ trên SQLite cục bộ.
- **SHA-256 là toàn vẹn, không phải chữ ký**: Hash chứng minh bytes không bị suy suyển (integrity), không xác thực ai tạo ra dữ liệu (authentication/provenance).
- **Tensors không trung chuyển qua Desktop**: Tuyệt đối không kéo các file checkpoint nặng nhiều GB qua RAM/băng thông của Desktop app.

---

## 2. Phân định Trách nhiệm (Separation of Concerns)

```
Research UI / Notebook / Agent
            │
      SDK + Local API
            │
   Application Use Cases
            │
Authority / Budget / Admission
            │
Runtime Experiment Coordinator
 ├── Compute Target Adapter   (Colab, Kaggle, Local Process, Dedicated SSH)
 ├── Training Engine Adapter  (PyTorch Trainer, Unsloth, HuggingFace Accelerate)
 ├── Checkpoint Storage Port  (S3, Cloudflare R2, HuggingFace Private, Local CAS)
 └── Shared Observation Pipeline
            │
 SQLite Records + Artifact References
```

- **custos-app**: Là native desktop shell, hiển thị số liệu đo lường trung thực, **không** là training controller thứ hai.
- **Giám sát (Supervision)**: Là máy trạng thái (State Machine) và mã chính sách (Policy Code), **không** phải là một LLM ngồi polling liên tục. Agent/S1 chỉ giải thích log hoặc đề xuất recipe khi có sự cố, không tự ý cấp thêm ngân sách hay tuyên bố job đã hoàn tất.

---

## 3. Bảy Thao tác Vòng đời Vận hành (Discrete Operations)

Thay vì gộp chung vào một hàm chạy lớn, mọi compute adapter bắt buộc phải phân định 7 thao tác:

| Thao tác | Ý nghĩa kỹ thuật |
|---|---|
| **1. Preflight** | Kiểm tra engine, credentials, đường dẫn dữ liệu, storage, compute target và hạn mức ngân sách. |
| **2. Submit** | Yêu cầu tạo execution attempt trên backend; thao tác này thành công **chưa** đồng nghĩa công việc đang chạy (`running`). |
| **3. Observe / Reconcile** | Xác định trạng thái thực tế của đúng `job_id` / `version_id`; đối soát khi mất kết nối. |
| **4. Request Checkpoint** | Yêu cầu engine lưu trạng thái tại biên optimizer step an toàn kế tiếp. |
| **5. Stop Job** | Dừng tiến trình huấn luyện của job cụ thể. |
| **6. Release Compute** | Giải phóng máy ảo/pod/runtime để dừng tính tiền. |
| **7. Collect** | Thu thập các tham chiếu artifact, metrics và checkpoint manifest đã được kiểm định. |

> [!WARNING]
> **Stop Job và Release Compute là hai thao tác tách biệt.** Dừng tiến trình training không đồng nghĩa máy ảo/pod đã được trả; mất kết nối stream cũng không chứng minh tài nguyên từ xa đã được giải phóng.

---

## 4. Hai Vòng đời và Ba Con số Tiến độ

### 4.1 Hai vòng đời độc lập
1. **Invocation**: Lần gọi CLI hoặc HTTP request tới backend.
2. **Execution Attempt**: Công việc thực tế đang chạy trên hạ tầng tính toán. Mỗi attempt có `attempt_id` riêng, liên kết chặt chẽ với account, target, recipe revision và parent checkpoint.

### 4.2 Ba con số tiến độ trên UI
Giao diện người dùng phải hiển thị minh bạch 3 con số, không dùng một thanh progress chung để che giấu:
- **`Latest Observed Step`**: Bước huấn luyện gần nhất quan sát được qua telemetry/log stream.
- **`Latest Locally Saved Step`**: Bước huấn luyện mà engine đã ghi thành công vào đĩa cục bộ của node tính toán.
- **`Latest Protected Step`**: Bước huấn luyện đã được tải lên kho lưu trữ bền vững bên ngoài, đã kiểm tra hash toàn vẹn và commit thành công.

*Ví dụ*: Observed 920, Local 900, Protected 850. Nếu node tính toán bị kill đột ngột, điểm phục hồi duy nhất có bằng chứng vững chắc là **850**, phần 851–920 phải được tính toán lại.

---

## 5. Quy trình Cam kết Checkpoint (Checkpoint Commit Protocol)

1. **Snapshot**: Engine chụp snapshot nhất quán tại biên optimizer step (tránh nửa chừng micro-batch).
2. **Unique Namespace**: Lưu vào namespace/prefix duy nhất theo cấu trúc: `experiments/{experiment_id}/attempts/{attempt_id}/checkpoints/step_{N}/`.
3. **Local Completeness**: Kiểm tra đủ các file bắt buộc (`weights`, `optimizer.pt`, `scheduler.pt`, `dataset_cursor.json`).
4. **Target-to-Storage Upload**: Tiến trình trên máy tính toán trực tiếp đẩy blobs lên storage bền vững với số luồng có giới hạn (`bounded concurrency`).
5. **Remote Integrity Check**: Kiểm tra digest, kích thước file và phiên bản trên storage từ xa.
6. **Publish Manifest**: Ghi `TrainingCheckpointManifest` bất biến chứa đầy đủ danh mục file và metadata.
7. **Canonical Promotion**: Thăng cấp checkpoint thành điểm phục hồi chính thức (`latest_protected_step`) thông qua atomic update/conditional write.
8. **Pre-Load Verification**: Node đích tải và thẩm định hash trước khi engine bắt đầu nạp weights/state.

---

## 6. Bốn Cấp độ Phục hồi (Resume Tiers)

Giao diện và domain model bắt buộc phân định 4 kết quả khi người dùng hoặc hệ thống muốn tiếp tục huấn luyện:

| Cấp độ | Tên gọi | Điều kiện dữ liệu | Ý nghĩa |
|---|---|---|---|
| **Tier 1** | **State Resume Supported** | Có đủ Model Weights + Optimizer + Scheduler + Engine State + Data Cursor + RNG. | Tiếp tục huấn luyện nguyên trạng trên profile phần cứng đã kiểm định. |
| **Tier 2** | **Adapted Continuation** | Recipe hoặc phần cứng thay đổi (ví dụ VRAM thấp hơn cần đổi gradient accumulation/micro-batch). | Tiến trình tiếp tục nhưng động lực học (optimizer dynamics) có thể khác biệt; ghi nhận là adapted lineage. |
| **Tier 3** | **Weights-Only Warm Start** | Chỉ có file trọng số (model weights / LoRA adapter), thiếu optimizer/scheduler/data state. | Khởi tạo một lượt huấn luyện mới từ weights đã học; reset learning rate schedule và data epoch. |
| **Tier 4** | **Unsupported / Unknown** | Thiếu file bắt buộc, format không tương thích, hoặc engine không hỗ trợ. | Từ chối tiếp tục để tránh lãng phí chi phí tính toán hoặc làm hỏng mô hình. |

---

---

## 7. Quy trình Chuyển giao Target (Target Handoff Protocol)

### 7.1 Planned Handoff (Chuyển target có kế hoạch)
1. **Preflight Destination**: Kiểm tra quota, credentials, environment và budget trên compute target đích.
2. **Safe Boundary Snapshot**: Ra lệnh cho source engine chụp snapshot tại optimizer-step boundary tiếp theo.
3. **Target-to-Storage Commit**: Đẩy blobs lên storage và hoàn tất canonical manifest.
4. **Quiesce Source**: Tạm dừng (quiesce) tiến trình source để ngăn tạo ra bước tính toán mới.
5. **Fence Promotion Rights**: Thu hồi quyền thăng cấp canonical checkpoint của source attempt.
6. **Spawn Destination Attempt**: Khởi tạo attempt mới trên target đích liên kết với parent checkpoint ID.
7. **Pre-load Validation**: Node đích tải và kiểm tra bytes/digests trước khi engine nạp weights và optimizer.
8. **Observe Running**: Giám sát cho đến khi destination báo cáo telemetry tiến độ bình thường.
9. **Release Source Compute**: Trả tài nguyên source (pod/VM) theo chính sách tiết kiệm chi phí.

### 7.2 Unplanned Interruption (Gián đoạn bất ngờ / Mất kết nối)
1. **Uncertain State Detection**: Mất heartbeat hoặc log stream vượt quá ngưỡng timeout; đánh dấu trạng thái là `Uncertain`.
2. **Reconciliation**: Đối soát chính xác với provider API xem process thực sự chết hay chỉ mất kết nối mạng.
3. **Chọn Checkpoint Committed Tương Thích**: Chỉ lấy checkpoint cuối cùng có trạng thái `CommittedProtected` (`latest_protected_step`).
4. **Ba lựa chọn minh bạch nếu Source chưa thể xác nhận đã dừng**:
   - *Chờ đối soát (Wait for reconciliation)*: Tiếp tục quan sát có giới hạn thời gian.
   - *Dừng dứt điểm (Stop source then verify)*: Gửi lệnh stop và release tài nguyên cũ trước khi tiếp tục.
   - *Tách nhánh cô lập (Isolated recovery branch)*: Tạo attempt phục hồi độc lập trên namespace mới kèm cảnh báo rõ về nguy cơ duplicate compute spend.
5. **Tuyệt đối không ghi đè**: Không bao giờ ghi đè lên thư mục output cũ và không công bố "đã chuyển giao thành công" chỉ vì đích vừa nhận lệnh submit.

---

## 8. Ranh giới Nền tảng và An toàn Bảo mật (Platform Boundaries & Security)

### 8.1 Giới hạn nền tảng thực tế
- **Google Colab**: Giới hạn tài nguyên và thời gian sống biến động; các hành vi remote-control headless bị hạn chế nghiêm ngặt trên runtime miễn phí không có số dư compute units. Sự tồn tại của Colab CLI không thay thế việc kiểm tra điều kiện tài khoản và điều khoản dịch vụ (ToS).
- **Kaggle Kernels**: Push thành công chỉ là submission batch job. Muốn sao lưu giữa chừng (mid-run backup) thì notebook bắt buộc phải có network access và cloud storage secrets được cho phép. Nếu không, Custos chỉ cung cấp cơ chế versioned outputs/import hoặc manual export sau khi batch hoàn tất.

### 8.2 An toàn Credentials và Phân quyền
- **Phân tách Custos Permit và Cloud Credentials**: Quyền cho phép nội bộ của Custos không tự động khiến cloud provider từ chối worker cũ. Cần kết hợp fencing ở tầng storage prefix và generation checks.
- **Quyền hạn tối thiểu (Principle of Least Privilege)**:
  - Cấp credentials theo từng attempt với thời gian sống ngắn (short-lived) và prefix-scoped access.
  - Tuyệt đối không truyền root storage keys hoặc quyền ghi toàn bucket cho code training của người dùng.
  - Tránh in secrets vào `argv`, log streams hoặc output cells của notebook.

---

## 9. Phân bổ Trách nhiệm trong 11 Crates của Custos

Tuân thủ kiến trúc phân tầng Inviolable 11 Crates của Custos (`Custos/AGENTS.md`):

| Trách nhiệm | Vị trí Crate | Nội dung hiện thực |
|---|---|---|
| **Recipe, Checkpoint & Attempt Records** | `custos-domain` | `TrainingCheckpointManifest`, `ResumabilityLevel`, `CheckpointCommitStatus`, `ExecutionRecord` với 3 bước tiến độ (`latest_observed_step`, `latest_local_checkpoint_step`, `latest_protected_checkpoint_step`). Pure domain, 0 I/O. |
| **Authority, Budget, Admission & Contracts** | `custos-core` | `ComputePort` (7 thao tác), `HardwareAdaptationPlan`, `evaluate_compatibility`, `plan_failover`. Zero-I/O contracts. |
| **Experiment & Recovery Coordinator** | `custos-runtime/src/experiment/` (module mới, chưa tồn tại) | Máy trạng thái điều phối vòng đời (State Machine), policy reconciliation, phát hiện timeout, điều phối commit/fencing. |
| **Kernel & Notebook Coordination** | `custos-runtime/notebook` | Điều phối interactive cells, kernel sessions độc lập với background training runs. |
| **Target CLI/API I/O Adapters** | `custos-adapters/src/compute/` (module mới, chưa tồn tại) | CLI/API adapters cho Colab, Kaggle, Dedicated SSH, Local Subprocess. Parsing structured outputs, capability detection. |
| **Engine Save/Load & Validation** | `custos-adapters/src/training/` (module mới, chưa tồn tại) | Engine integration (Unsloth, HuggingFace Trainer, PyTorch DCP) thẩm định cấu trúc checkpoint theo từng recipe. |
| **Remote Blob Transfer & Storage** | `custos-adapters/src/storage/` (module mới, chưa tồn tại) | Tương tác S3/R2/HF để verify digest, conditional writes, prefix fencing. |
| **Canonical Records, Outbox & Migrations** | `custos-persistence` | SQLite tables cho execution records, attempts, checkpoints, storage manifests và outbox events. |
| **Application Use Cases & Composition** | `custos-daemon` | Gắn kết các ports và handlers, phục vụ HTTP/TCP RPC APIs cho Desktop App. |
| **Giám sát Minh bạch & UI Controls** | `custos-app-desktop` | Native UI hiển thị 3 con số tiến độ, 4 resume tiers, nút bấm can thiệp và cảnh báo rủi ro chi phí. |

---

## 10. Lộ trình Triển khai Chuẩn (Execution Roadmap)

1. **Bước 1 — CPU Deterministic Fixture**: Xây dựng kịch bản kiểm thử offline trên CPU: so sánh lượt chạy liên tục (`uninterrupted`) với lượt chạy bị ngắt giữa chừng rồi phục hồi (`interrupted -> resumed`), chứng minh data cursor và model state nối khớp chính xác.
2. **Bước 2 — Single Engine & Local Process Runner**: Xây dựng `process_runner.rs` và `manifest_verifier.rs` để chạy recipe thực tế, thu thập telemetry và commit checkpoint nội bộ.
3. **Bước 3 — Single Remote Target**: Hiện thực hóa 7 thao tác trên một target cụ thể (Colab hoặc Kaggle), xử lý đối soát mất kết nối.
4. **Bước 4 — Durable Storage Direct Upload**: Cơ chế đẩy checkpoint trực tiếp từ compute node lên S3/R2/HF mà không qua Desktop app.
5. **Bước 5 — Hardware Handoff Verified Pair**: Thực hiện chuyển tiếp thành công giữa một cặp backend đã được kiểm định.
6. **Bước 6 — Full Target Matrix & Adaptation**: Mở rộng Colab, Kaggle, Dedicated SSH với công thức thích ứng phần cứng.
7. **Bước 7 — OI Measured Optimization**: Dùng dữ liệu đo lường thực tế để tối ưu hóa tần suất checkpoint và lựa chọn backend kinh tế nhất.

### Bộ Test Quyết định D1–D10 (chưa có test nào được hiện thực; xem f10_implementation_plan §6)
Mọi tầng tích hợp phải vượt qua các ca kiểm thử then chốt:
- [ ] **Duplicate Submit**: Gửi 2 lần cùng một yêu cầu không sinh ra 2 running instances.
- [ ] **Source Alive after Disconnect**: Khi stream ngắt nhưng remote process vẫn sống, hệ thống phát hiện và reconcile đúng trạng thái.
- [ ] **Incomplete / Corrupt Upload**: Upload dở dang hoặc sai hash bị từ chối commit `CommittedProtected`.
- [ ] **Stale Writer / Generation Fencing**: Worker cũ cố tình upload checkpoint muộn bị từ chối promote.
- [ ] **Valid Zero-Valued Optimizer State**: Checkpoint của optimizer không có tensor momentum (ví dụ SGD) vẫn được chấp nhận hợp lệ.
- [ ] **Data Cursor Continuity**: Kiểm tra con trỏ batch và shuffle state nối tiếp chính xác sau khi resume.
- [ ] **Canary Non-Mutating Validation**: Bước kiểm tra canary trước khi tiếp tục không làm biến đổi weights/optimizer state.
- [ ] **Destination Unavailable**: Target đích lỗi kết nối thì hủy bỏ quy trình an toàn mà không làm mất trạng thái source.
- [ ] **Release Compute Failure Handling**: Lỗi giải phóng VM/pod cảnh báo người dùng ngay lập tức để tránh phát sinh chi phí ngầm.
- [ ] **Controller Daemon Offline Resilience**: Daemon trên laptop tắt không làm gián đoạn worker đã cấp policy đang chạy trên target.

