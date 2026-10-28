# Icon khay hệ thống và widget: so sánh trước khi viết

Yêu cầu (28/10/2026): không còn cửa sổ cmd hiện lên, không mở cả loạt tab trình duyệt.
Thay vào đó là một icon ở khay hệ thống, hover vào thì hiện số liệu, và thêm một widget.

| Dự án | Cách làm | Hover / click | Widget nổi | Phù hợp với dự án này | Chi phí |
|---|---|---|---|---|---|
| [jens-duttke/usage-monitor-for-claude](https://github.com/jens-duttke/usage-monitor-for-claude) | Python, pystray + pywebview (popup là HTML) | hover: tooltip chữ; click: popup chi tiết | ghim popup, kéo đi được | ý tưởng hay (popup ghim được), nhưng pywebview cần runtime Python | thêm Python + WebView |
| [ksmaster03/claude-usage-tray](https://github.com/ksmaster03/claude-usage-tray) | Python, tkinter + Pillow | icon vẽ động (màu theo quota còn lại, in số %); double-click: chi tiết | luôn nổi trên cùng, kéo đi được, chỉnh độ trong | icon vẽ động và widget nổi đúng thứ cần | thêm Python + tkinter |
| [Zrnik/claude-usage-windows-taskbar-widget](https://github.com/Zrnik/claude-usage-windows-taskbar-widget) | C# WPF, cửa sổ không viền cạnh khay | hover: tooltip đếm ngược | cửa sổ luôn ở góc taskbar | cách đặt cửa sổ hay, nhưng cần .NET 8 | thêm .NET 8 Desktop Runtime |
| [tauri-apps/tray-icon](https://github.com/tauri-apps/tray-icon) (crate Rust) | bọc Shell_NotifyIcon, có sự kiện Enter/Leave | tự viết | không có, phải ghép thêm tao/wry | dùng được, nhưng kéo theo `windows` crate và vòng lặp sự kiện riêng | nhiều phụ thuộc mới |
| **Win32 trực tiếp qua `windows-sys`** (đã có trong Cargo.toml) | `Shell_NotifyIconW` bản 4: Windows gửi `NIN_POPUPOPEN` khi hover để app tự vẽ popup thay tooltip; vẽ bằng GDI | hover: popup tự vẽ; click: mở dashboard | cùng cửa sổ, kéo đi được, luôn nổi | **chọn**: không thêm crate, exe vẫn nhỏ, không cần runtime | tự viết khoảng 400 dòng |

## Chọn

Win32 trực tiếp qua `windows-sys` (`src/tray.rs`). Lấy từ các dự án trên:
- icon vẽ động, màu theo mức dùng (ksmaster03);
- popup chi tiết hiện khi hover, thay cho tooltip chữ (NIN_POPUPOPEN);
- widget ghim được, kéo đi được, luôn nổi trên cùng, nhớ vị trí (jens-duttke, ksmaster03).

Cửa sổ cmd: exe chuyển sang `windows_subsystem = "windows"` (không tạo console); khi chạy
từ terminal thì gắn vào console của terminal để vẫn in được chữ. PowerShell mà desktop.rs
gọi chạy với `CREATE_NO_WINDOW`.
