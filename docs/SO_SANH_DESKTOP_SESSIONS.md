# So sánh cách giữ session Code của Claude Desktop khi đổi account

Vấn đề: transcript của tab Code nằm chung ở `~/.claude/projects` cho mọi account, nhưng
danh sách ở sidebar thì riêng từng account:
`<thư mục dữ liệu Desktop>/claude-code-sessions/<accountUuid>/<orgUuid>/local_*.json`
(mỗi file là một "thẻ" session, trỏ tới transcript bằng `cliSessionId`). Đổi account thì
sidebar trống dù transcript còn nguyên. Anthropic đã có báo lỗi
[#48511](https://github.com/anthropics/claude-code/issues/48511), chưa có bản sửa chính
thức. Máy này (2026-10-30): 6 thư mục account, cùng một org, 115 thẻ, 13 MB; thư mục dữ
liệu là `%LOCALAPPDATA%\Claude-Data\claude_<id>` (bản Desktop mới).

| Dự án | Cách làm | Lúc chạy | An toàn | Mức phù hợp | Chi phí |
|---|---|---|---|---|---|
| [bdarbaz/claude-session-sync](https://github.com/bdarbaz/claude-session-sync) (Python, MIT) | Chép thẻ giữa mọi thư mục account/org; bỏ các trường riêng của account (`remoteMcpServersConfig`, `enabledMcpTools`, `toolSurfaceSnapshot`, `bridgeSessionIds`, `remoteControlAutoEligible`, `steeredByRemoteClient`, `error`, `errorAt`, `priorErrorMark`), lấy 3 trường connector từ thẻ gốc của thư mục đích; thẻ mới hơn (`max(lastActivityAt, lastFocusedAt)`) đè thẻ cũ; xoá ở một nơi thì chuyển bản ở nơi khác vào trash; ghi lại `archived-sessions.idx` | Tiến trình nền, 5 giây một lần, cả khi Desktop đang chạy (không sửa thư mục của account đang đăng nhập vì app giữ trong bộ nhớ và ghi đè) | Backup lần đầu, ghi qua file tạm, so mtime trước khi ghi | Cao về quy tắc; nhưng **không tìm thấy** thư mục `Claude-Data` của bản Desktop mới (chỉ tìm `%APPDATA%\Claude` và `Packages\Claude_*\LocalCache`) | Thêm một tiến trình nền + shortcut Startup |
| [sahol3/claude-code-session-restorer](https://github.com/sahol3/claude-code-session-restorer) (PowerShell, MIT) | Chép thẻ từ các account khác vào account đang đăng nhập; tạo thẻ mới cho transcript chưa có thẻ (lấy `aiTitle`) | Chạy tay, **Desktop phải tắt** ("app ghi lại thư mục này khi thoát") | Backup trước, chạy lại không tạo trùng | Trung bình: một chiều, chạy tay | Không |
| [realiti4/claude-swap](https://github.com/realiti4/claude-swap) | Chỉ cho CLI: `--share-history` để các profile thấy chung transcript | — | — | Thấp: không đụng tới sidebar của Desktop | — |

## Chọn

Làm trong Claude Usage, ngay trong lệnh chuyển Desktop, theo quy tắc của
claude-session-sync, vào lúc Desktop **đã tắt** (như session-restorer khuyên):

- Lệnh chuyển vốn đã tắt Desktop, lưu đăng nhập cũ, đặt đăng nhập mới rồi mở lại; chép thẻ
  ở giữa bước tắt và bước mở thì không có app nào giữ thẻ trong bộ nhớ, nên mọi thư mục
  đều ghi được, không cần tiến trình nền.
- Dùng đúng thư mục dữ liệu mà app đã tìm (`desktop_dir`, có cả `Claude-Data`).
- Giữ các quy tắc an toàn: bỏ trường riêng của account, thẻ mới hơn thắng, xoá thì vào
  trash, backup lần đầu, ghi qua file tạm, thẻ không đọc được thì để yên.
- Không tạo thẻ cho transcript chưa có thẻ (việc của session-restorer): ngoài phạm vi.
