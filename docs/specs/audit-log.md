# Audit log (ảnh chụp mỗi vòng poll)

## Vấn đề
`usage-log.jsonl` chỉ ghi figures từng account với trường `active` = **chỉ account Code CLI**
(`figures()` trong `usage_log.rs`, `active = active_id == id`). Nó KHÔNG ghi account Claude
Desktop đang đăng nhập, cũng không ghi `in_use` (Code HOẶC Desktop) hay `error` theo thời
điểm. Hậu quả: khi auto-switch "không chuyển", không tái dựng được *lúc đó account nào đang
được dùng* (nhất là ở Desktop) — một account dùng ở Desktop trông y hệt account nằm im
(`active:false`). Đây là lỗ hổng làm việc chẩn đoán bất khả thi.

## Mục tiêu
Mỗi vòng poll (khi có account được hỏi), ghi MỘT dòng ảnh chụp đủ để tái dựng quyết định
switch: ai đang dùng (Code + Desktop), và figures/error của từng account tại thời điểm đó.

## Đầu vào / đầu ra
- `snapshot(st: &State, desk: Option<&str>, t: i64) -> Value` — hàm thuần, không I/O:
  ```json
  {
    "t": <ms>,
    "codeActive": "<active_id|null>",   // account Code CLI đang dùng
    "desktop": "<uuid|null>",            // account Claude Desktop đang đăng nhập
    "accounts": [
      {"id", "inUse": <bool>, "error": "<text|null>", "five": <u%|null>, "week": <u%|null>}
    ]
  }
  ```
  `inUse` lấy từ `Account.in_use` (đã được `mark_in_use` đặt = Code-active HOẶC Desktop-signed-in).
- `Audit`: ghi ra `audit.jsonl` cạnh `state.json`; giữ 60 ngày (cắt lúc khởi động); mỗi dòng
  là một ảnh chụp. Best-effort: lỗi ghi không làm hỏng poll.

## Tích hợp
- `poll_all` (poll.rs): sau khi poll xong các account (đã `mark_in_use`), trước khi switch,
  gọi `audit.append(snapshot(...))`. Khi không có account nào đến hạn thì không ghi.
- `App` thêm field `audit: audit::Audit`; `main.rs` khởi động `audit.start(data_dir, now)`.

## Tiêu chí xong
- `snapshot` có test đỏ-trước-xanh: bắt đúng codeActive/desktop/inUse/error/five/week và xử lý
  null (không active, không desktop, account chưa có usage).
- `cargo test` xanh; `cargo build` sạch. File `audit.rs` ≤ 200 dòng, một trách nhiệm.
- Không đổi hành vi switch; chỉ thêm ghi log.
