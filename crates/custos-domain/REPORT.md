# Báo cáo backend custos-domain

## Trạng thái

`custos-domain` là crate miền nghiệp vụ thuộc Layer 0 của Custos. Crate này chứa các giá trị miền có thể tuần tự hóa, máy trạng thái vòng đời, các thành phần bảo đảm tính toàn vẹn và các quy tắc kiểm tra hợp lệ. Crate tiếp tục tuân thủ nguyên tắc Zero-I/O: không thực hiện thao tác với hệ thống tệp, cơ sở dữ liệu, mạng, tiến trình, nhà cung cấp mô hình hoặc runtime bất đồng bộ.

Đợt triển khai này gia cố các hợp đồng hiện có mà không tạo crate mới, không di chuyển ranh giới module, không thêm dependency bên ngoài và không đưa logic persistence hoặc transport vào domain.

## Phạm vi hiện tại

Crate hiện có 37 tệp mã nguồn Rust với tổng cộng 5.105 dòng. Bề mặt public được tổ chức theo các nhóm sau:

| Nhóm chức năng | Module chính | Trách nhiệm |
|---|---|---|
| Vòng đời tác vụ | `task`, `run`, `span`, `session` | Quản lý phiên bản Task, chuyển trạng thái, lần chạy, retry có giới hạn, span và session |
| Hiệu ứng có kiểm soát | `action`, `authority`, `approval` | Action intent, effect attempt, grant, permit dùng một lần, receipt và phê duyệt của con người |
| Xác minh | `evidence`, `claim`, `artifact`, `continuation` | Trạng thái bằng chứng, research claim, tham chiếu artifact, phát hiện dữ liệu stale và tính toàn vẹn của continuation |
| Kiểm soát tài nguyên | `budget`, `workspace` | Hạch toán reservation token/span, mức headroom trừu tượng và vòng đời execution workspace |
| Lập kế hoạch | `workflow`, `oi`, `decision` | Workflow plan/revision và các giá trị quyết định của orchestration intelligence |
| Ngữ cảnh và bộ nhớ | `context`, `memory`, `fact`, `packet` | Context pack, đề xuất bộ nhớ, fact và các export tương thích cho continuation |
| Repo Intelligence | `repo` | Các giá trị thuần túy cho snapshot, source span, coverage, confidence và query result |
| Thành phần dùng chung | `ids`, `error`, `types`, `capability` | ID, digest xác định, lỗi miền có kiểu, alias và capability manifest |

## Các phần đã triển khai và gia cố

### Tính toàn vẹn của approval

- Một `ApprovalDecision` chỉ có thể xử lý `ApprovalRequest` có ID trùng khớp.
- Request đã hết hạn được chuyển sang `TimedOut` và không thể được phê duyệt hoặc từ chối sau đó.
- Request đã được xử lý không thể nhận quyết định lần thứ hai.

Các kiểm tra này ngăn một quyết định hợp lệ bị phát lại cho request khác, đồng thời ngăn quyết định đến muộn cấp quyền cho action đã hết hạn.

### Hạch toán budget

- Phép tính khi reserve hiện sử dụng số học có kiểm tra tràn số.
- Settlement bị từ chối nếu mức sử dụng thực tế vượt quá reservation đang có.
- Refund bị từ chối nếu giá trị hoàn trả vượt quá lượng đã reserve.
- Khi kiểm tra settlement hoặc refund thất bại, toàn bộ counter được giữ nguyên.
- Counter đã settle và phép tính headroom được bảo vệ khỏi lỗi tràn số nguyên.

Trước đây, phép trừ bão hòa có thể che giấu lỗi underflow trong hạch toán. Domain hiện trả về vi phạm một cách rõ ràng thông qua `DomainError`.

### An toàn khi tạo Task revision

- Việc tăng Task epoch, revision number và state version hiện sử dụng số học có kiểm tra.
- Lỗi tràn counter không thể làm thay đổi một phần trạng thái Task hoặc revision history.
- `Task::create_revision` hiện trả về `Result<&TaskRevision, DomainError>`.
- Đã loại bỏ `unwrap` trong production path từng được dùng để trả về active revision vừa chèn.

Không có consumer ngoài test nào trong workspace hiện gọi `Task::create_revision`, vì vậy chữ ký fallible mới không yêu cầu migration downstream trong checkout hiện tại.

### An toàn khi cập nhật Action

- `Action::with_assurance` không còn mặc định index mọi JSON value như một object.
- Với object parameters, trường `assurance` chuẩn vẫn được cập nhật.
- Với payload dạng scalar, array hoặc null, giá trị gốc được giữ nguyên trong khi trường assurance có kiểu vẫn được cập nhật, qua đó loại bỏ một panic path.

### Task spine và continuity

- `ConversationTurn` giữ turn ID, actor, lens, Task revision, attempt attribution, resource refs và privacy class mà không nhân bản journal theo workbench.
- `SessionTaskBinding` gắn một khoảng turn với đúng Task revision và chỉ được đóng một lần.
- `ContinuationManifest` giữ checkpoint cùng selected turn/source/artifact refs; không chứa grant, permit, secret hoặc native checkpoint.
- `ContextReceipt` ghi rõ refs được include/omit, redaction digest, token estimate và trạng thái delivery. `Delivered` chỉ là transport acknowledgement, không phải model comprehension.

### Execution truth, usage và outcome

- `LaunchAttempt` tách requested mode khỏi actual mode và giữ `Unknown` là trạng thái không được retry mù.
- `UsageRecord` phân biệt model/native harness/tool/compute và dùng `UsageMeasurement::Known|Estimated|Unknown`; telemetry thiếu không còn bị biểu diễn thành chi phí hoặc token bằng 0.
- `CriterionVerificationRecord` biểu diễn `Pass|Fail|Unknown|Stale` theo criterion, method/version và source revision.
- `TaskOutcome` bắt buộc có lineage tới ít nhất một verifier record; core vẫn là owner của completion decision.

### Proposal và handoff không tự cấp trust

- `ResearchClaimProposal` materialize claim ở đúng `L0Ungrounded`, confidence 0, không evidence hoặc sealed proof do client tự gắn.
- `SourceProposal` và `PassageAnchorProposal` tách metadata/locator/selection do client đề xuất khỏi source/passage hash và verification do backend sở hữu.
- `ArtifactHandoff` giữ producer/consumer, version/digest, selected/redacted refs, privacy, consent và caveats nhưng cố ý không mang authority.

## Phạm vi kiểm thử hồi quy

Sau đợt triển khai này, crate có tổng cộng 36 unit test. Các test hồi quy mới bao phủ:

- Approval decision và request không trùng ID.
- Approval request đã hết hạn.
- Request hợp lệ chỉ được xử lý một lần.
- Settlement và refund vượt reservation, đồng thời bảo đảm counter không bị thay đổi một phần.
- Tràn counter khi reserve.
- Tràn Task epoch và revision state version mà không gây mutation một phần.
- Cập nhật assurance với JSON parameters không phải object.
- Turn attribution và session–Task binding.
- Continuation/context state transitions và delivery semantics.
- Cross-pack artifact handoff và consent một lần.
- Research claim proposal không thể tự gắn trusted status.
- Unknown launch/usage, per-criterion verification và outcome lineage.

## Kết quả xác thực

Các kiểm tra đã hoàn thành:

- Chạy trực tiếp `rustfmt --check` cho các tệp Rust đã sửa: đạt.
- Chạy `git diff --check` trong phạm vi `crates/custos-domain` và tài liệu liên quan: đạt; working tree UI có trailing whitespace tồn tại ngoài phạm vi.
- Quét các import I/O bị cấm gồm `tokio`, `rusqlite`, `reqwest`, `std::fs`, `std::net` và `std::process`: không có kết quả khớp.
- Quét toàn bộ call site trong workspace đối với hành vi thay đổi và Task revision API mới: không có production call site cần sửa.
- Không chỉnh sửa các thay đổi UI không liên quan đang tồn tại trong working tree.

Ban đầu môi trường repository không có Rust toolchain trong `PATH`. Một toolchain tối thiểu, cô lập trong thư mục tạm đã cung cấp `rustc 1.99.0` và `rustfmt`, nhưng `cargo check -p custos-domain --offline` không thể biên dịch các dependency build script vì máy chưa cài MSVC linker và Windows SDK libraries (`link.exe`, `kernel32.lib`). Lỗi xảy ra trước khi `custos-domain` được type-check; vì vậy không được diễn giải kết quả format thành compile pass. Không có cài đặt toàn hệ thống hoặc thay đổi dependency của dự án. Cần chạy lại các lệnh sau trong môi trường có Visual C++ Build Tools và Windows SDK:

```text
cargo fmt --check -- crates/custos-domain
cargo check -p custos-domain
cargo test -p custos-domain
```

## Công việc cần tiếp tục

- `Budget` đã có trường `max_cost_cents`, nhưng thao tác reserve/settle hiện chỉ hạch toán span và token. Việc reserve và settle chi phí tiền tệ một cách bền vững cần thay đổi phối hợp giữa `custos-core` và persistence, nên không được tự ý bổ sung riêng trong Layer 0 ở đợt này.
- Continuation packet bảo đảm tính toàn vẹn cho canonical payload hiện tại. Các semantics mở rộng như pending effect, event cursor và tái xác minh tài nguyên vẫn là công việc xuyên layer đã được mô tả trong workspace restructuring plan.
- Báo cáo này phản ánh trạng thái đã triển khai của `custos-domain`; không khẳng định runtime orchestration, persistence, adapter hoặc daemon composition đã hoàn thiện.
