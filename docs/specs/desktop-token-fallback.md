# Đọc usage qua token Claude Desktop khi refresh token bị xoay mất

## Vấn đề
Refresh token của Anthropic là dùng-một-lần (xoay vòng). Một account đang đăng nhập trong
Claude Desktop (vd account chính của người dùng) sẽ bị Desktop làm mới token định kỳ, **huỷ**
bản refresh token mà claude-usage đã lưu cho account đó. Khi claude-usage poll account parked
này, `fetch_usage` gọi `refresh()` → `ApiErr::Auth("refresh failed (400) ... /login again")`.

Hiện fallback đọc token Desktop (`desktop_usage`) CHỈ chạy cho account **Code-CLI-live** với
lỗi chứa "Claude Code renews" (poll.rs). Nên account dùng-ở-Desktop bị kẹt: usage đóng băng,
không bao giờ thấy chạm trần → không tự chuyển. (Xem [[auto-switch-rules-claude-usage]].)

`desktop::desktop_access_token(dir, uuid)` đọc token từ cache của Claude Desktop **theo uuid**,
độc lập `lastKnownAccountUuid` — nên dùng được cho mọi account có mặt trong cache Desktop.

## Thay đổi
Trong `poll_account` (poll.rs), khi `fetch_usage` lỗi, nếu lỗi là loại "token lưu không dùng
được nhưng Desktop có thể có token sống", thì fallback `desktop_usage(account_uuid)`:
- `ApiErr::Auth(_)` → true (refresh bị từ chối / 401: token đã bị xoay/huỷ — thử token Desktop).
- `ApiErr::Other(m)` khi `is_live && m` chứa "Claude Code renews" → true (giữ hành vi cũ).
- `ApiErr::Slow(_)` → false (đang bị 429, tôn trọng back-off).

Hàm thuần `wants_desktop_fallback(&ApiErr, is_live) -> bool` (test được, không cần mạng).
`desktop_usage` trả None nhanh (không gọi mạng) nếu Desktop không có token cho uuid đó → account
không ở Desktop vẫn giữ nguyên lỗi như cũ (không hồi quy). Fallback thành công → usage tươi,
`error` được xoá.

## Tiêu chí xong
- Test đỏ-trước-xanh cho `wants_desktop_fallback` (Auth→true; Other+live+"renews"→true;
  Other không-live→false; Slow→false; Other khác→false).
- `cargo test` xanh, `cargo build` sạch. Không đổi hành vi switch, chỉ sửa đường đọc usage.
- Không gọi mạng thừa: chỉ account có token trong cache Desktop mới phát sinh 1 call usage.
