# So sánh: nhận biết một kho cookie Chromium còn đăng nhập hay không

Bài toán: một account Claude Desktop đã lưu có còn cookie `sessionKey` hợp lệ của claude.ai không. Mục đích là không chuyển Desktop sang một account đã bị đăng xuất. Đã đọc mã nguồn từng dự án (12/11/2026).

| Dự án | Phương pháp | Mức phù hợp | Chi phí |
|---|---|---|---|
| plexxed/claude-desktop-account-switcher (`scripts/Private/Switch.ps1`) | Chỉ xem `config.json` (`Get-SignedInAccount`), không đọc cookie | Thấp: chính là cách app đang làm, không thấy cookie đã mất | Không có |
| thewh1teagle/rookie (`rookie-rs/src/browser/chromium.rs`) | Dùng rusqlite: `SELECT host_key, …, expires_utc, name … FROM cookies` | Cao: Rust, đúng bảng và đúng cột | Thêm crate rusqlite (bundled) |
| borisbabic/browser_cookie3 (`__init__.py`) | Dùng sqlite3: chép file sang chỗ tạm rồi `SELECT host_key, path, secure, expires_utc, name …` | Cao về cách mở: chép ra file tạm để không khóa file gốc | Không có (Python) |
| Quét byte thô tìm chữ "sessionKey" (đã thử trên máy này) | Đếm chuỗi trong file | Thấp: trang đã xóa có thể còn byte cũ, không biết hạn | Không có |

**Chọn:**
- rusqlite với tính năng `bundled`, truy vấn kiểu rookie.
- Chép file ra chỗ tạm trước khi mở, như browser_cookie3.
- So hạn bằng `has_expires` / `expires_utc` (micro giây tính từ 1601).

**Số đo trên máy này:**
- Quét byte tìm "sessionKey": 0080af61 có 8, 359a6468 có 8, d8828315 có 4, fb2bda4d có 0.
- Đọc bằng SQLite: chỉ fb2bda4d không có dòng `sessionKey`.
