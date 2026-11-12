# So sánh cách nhúng icon + thông tin phiên bản vào exe Windows (Rust)

Vấn đề (11/11/2026): `claude-usage.exe` build ra là exe "trần". `ExtractIconEx` đếm được 0 icon
(notepad.exe có 1), còn `ProductName` và `FileDescription` đều rỗng. Lý do là dự án không có
`build.rs`, `.rc` hay `.ico`. Hệ quả: Start, taskbar, mục Uninstall và Task Manager không có icon
và tên riêng của app.
Đã đọc thẳng `build.rs` / `Cargo.toml` của từng dự án (tháng 11/2026).

| Dự án | Phương pháp | Icon | VERSIONINFO | Mức phù hợp | Chi phí |
|---|---|---|---|---|---|
| Zed (`crates/windows_resources`) | `embed-resource` 3.0; build.rs **sinh file `.rc`** (icon theo kênh phát hành + VERSIONINFO lấy `CARGO_PKG_VERSION`) rồi `compile` | Có | Có, sinh tự động | Cao: version luôn khớp Cargo.toml, không phải truyền macro có dấu nháy cho rc.exe | Thấp: ~60 dòng |
| Alacritty (`alacritty/build.rs`) | `embed_resource::compile("./windows/alacritty.rc", NONE)` với `.rc` viết sẵn (ICON + manifest) | Có | Không | Trung bình: gọn, nhưng version phải sửa tay trong `.rc` | Rất thấp |
| RustDesk (`build.rs`) | `winres::WindowsResource::new().set_icon(..).compile()` | Có | Có (crate tự lấy từ Cargo) | Thấp: `winres` 0.1.12 đã bỏ, không còn ai bảo trì | Rất thấp |
| `winresource` 0.1.34 (bản fork còn bảo trì của winres) | API như winres, tự đọc metadata trong Cargo.toml | Có | Có | Trung bình: tiện, nhưng thêm một lớp ẩn | Rất thấp |

Máy build: toolchain `x86_64-pc-windows-msvc`, có `rc.exe` của Windows SDK (10.0.19041, 10.0.26100).
Cả 4 cách đều chạy được.

**Chọn cách của Zed:** `embed-resource` 3 (crate được dùng nhiều nhất, Zed và Alacritty đều dùng),
build.rs sinh `.rc` vào `OUT_DIR` với version lấy từ `CARGO_PKG_VERSION`. Cách này không cần
`#include <winver.h>` (khỏi phụ thuộc đường dẫn include của SDK) và không cần macro có dấu nháy.
Icon `assets/claude-usage.ico` được vẽ bằng chính hàm `icon_pixels` của icon khay
(`src/tray/icon.rs`): exe và khay cùng một hình, và có test giữ chúng khớp nhau.
