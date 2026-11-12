# So sánh cách dựng binary macOS cho dự án Rust trên GitHub Actions

Mục tiêu: chọn cách viết `.github/workflows/macos.yml` cho claude-usage (tiny_http, ureq + rustls/ring, rusqlite `bundled` nên có mã C dịch bằng `cc`): chạy `cargo test`, dựng binary macOS (universal hoặc tách arm64 + x86_64), đóng gói và upload làm workflow artifact. Chưa ký, chưa notarize. Dưới đây là cách 5 dự án mã nguồn mở đang làm, đọc trực tiếp từ file workflow của họ (ngày 2026-11-12).

## Bảng so sánh

| Dự án | Phương pháp | Mức phù hợp với claude-usage | Chi phí/độ phức tạp |
|---|---|---|---|
| [ripgrep](https://github.com/BurntSushi/ripgrep) `release.yml` | Hai mục matrix đều `os: macos-latest` (arm64), target `x86_64-apple-darwin` và `aarch64-apple-darwin`, tức là **dịch chéo x86_64 trên runner arm**, kể cả mã C của pcre2 (`PCRE2_SYS_STATIC: 1`). `dtolnay/rust-toolchain@master` + `targets`. `strip`, rồi `tar czf` + `shasum -a 256`, đẩy lên release bằng `gh release upload`. Chạy luôn binary x86_64 để sinh man page (Rosetta). Không lipo, không ký, không cache. `actions/checkout@v4` | Cao: chứng minh crate có mã C qua `cc` dịch chéo x86_64 trên `macos-latest` được | Thấp; 2 job song song, 2 tarball riêng |
| [bat](https://github.com/sharkdp/bat) `CICD.yml` | Mỗi kiến trúc một runner gốc: `x86_64-apple-darwin` trên `macos-15-intel`, `aarch64-apple-darwin` trên `macos-latest`. `dtolnay/rust-toolchain@stable` + `targets`. `cargo build --locked --release --target=…`, test trên cả hai. `tar czf` → `.tar.gz`, `actions/upload-artifact@master`. `actions/checkout@v6`. Không lipo, không ký, không cache | Trung bình: test x86_64 chạy trên máy Intel thật, nhưng tốn 2 runner và phụ thuộc nhãn Intel sắp hết | Trung bình; 2 runner macOS |
| [cargo-dist](https://github.com/axodotdev/cargo-dist) (`release.yml` sinh ra) | Matrix do `dist plan` tính; mặc định trong `github.rs`: `aarch64-apple-darwin` → `macos-15`, `x86_64-apple-darwin` → `macos-15-intel`. Không dùng action cài Rust (dist tự lo). Action ghim theo SHA commit. Upload `target/distrib/` bằng upload-artifact, có `actions/attest`. Không ký, không cache build | Thấp: kéo cả công cụ dist và quy trình release cho một exe nhỏ | Cao; thêm `dist-workspace.toml`, file YAML tự sinh |
| [Alacritty](https://github.com/alacritty/alacritty) `release.yml` + `Makefile` | Một job `macos-latest`: `rustup target add aarch64-apple-darwin` + `x86_64-apple-darwin`; `cargo test --release --target=x86_64-apple-darwin` (chạy qua Rosetta trên arm); Makefile đặt `MACOSX_DEPLOYMENT_TARGET="10.12"`, build cả hai, rồi `lipo target/{x86_64,aarch64}-apple-darwin/release/… -create -output …`; ký ad-hoc `codesign --force --deep --sign -`; đóng DMG. `actions/checkout@v4` | Cao: đúng mô hình 1 runner + lipo | Thấp; không cần runner Intel |
| [taiki-e/upload-rust-binary-action](https://github.com/taiki-e/upload-rust-binary-action) `main.sh` | Target ảo `universal-apple-darwin`: `rustup target add aarch64-apple-darwin x86_64-apple-darwin`, một lệnh `build --target aarch64-apple-darwin --target x86_64-apple-darwin`, rồi `lipo -create -output`. Tar bằng GNU `gtar` (cài qua Homebrew nếu thiếu); `codesign` tuỳ chọn. Không đặt `MACOSX_DEPLOYMENT_TARGET`. README: `macos-latest` đã đổi kiến trúc từ macos-14 nên nên ghi `target` rõ | Trung bình: đúng cách làm nhưng action này upload lên **release**, ta mới cần artifact | Thấp nếu dùng action; ta chỉ mượn các lệnh |

## Dữ kiện về runner và action (đọc ngày 2026-11-12)

- `macos-latest` = macOS 26 **arm64** (bảng trong `actions/runner-images`). Intel còn `macos-15-intel`, `macos-26-intel`. Nhãn `-latest` chuyển dần trong 1-2 tháng khi có bản mới; muốn cố định thì ghi rõ (vd. `macos-26`).
- `macos-13` đã nghỉ hưu "by December 4th, 2025" (changelog GitHub); GitHub gợi ý `macos-15-intel` cho Intel. macOS 14 nghỉ hưu ngày 2026-11-02.
- Bản mới nhất: `actions/checkout@v7` (v7.0.1 đánh dấu Latest), `actions/upload-artifact@v7` (README), `Swatinem/rust-cache@v2` (README; tự né lỗi hỏng cache trên macOS cargo#8603), `dtolnay/rust-toolchain@stable` (tham chiếu theo tên toolchain, không có số major; `targets` là chuỗi phân cách bằng dấu phẩy).
- upload-artifact nén zip và **làm mất quyền file** (file thành 644). Cách README chỉ: tự `tar` trước, rồi upload với `archive: false` (chỉ được 1 file; tên artifact = tên file, bỏ qua `name`).
- Rust tối thiểu: macOS 10.12 cho x86_64, 11.0 cho arm64; `rustc` đọc `MACOSX_DEPLOYMENT_TARGET`; xem mặc định bằng `rustc --print=deployment-target`.

## Chọn

Một job `macos-latest` (arm64), giống Alacritty và taiki-e:
1. `actions/checkout@v7`, `dtolnay/rust-toolchain@stable` với `targets: aarch64-apple-darwin,x86_64-apple-darwin`, `Swatinem/rust-cache@v2`.
2. `cargo test --locked` trên kiến trúc gốc (arm64). Tuỳ chọn: thêm `cargo test --target x86_64-apple-darwin` chạy qua Rosetta như Alacritty.
3. Đặt `MACOSX_DEPLOYMENT_TARGET: "11.0"` ở `env` của job để rustc và `cc` (sqlite bundled, ring) dùng cùng mức tối thiểu; bản universal cần 11.0 vì arm64 không xuống thấp hơn được.
4. `cargo build --release --locked --target aarch64-apple-darwin --target x86_64-apple-darwin`, rồi `lipo -create -output claude-usage target/{aarch64,x86_64}-apple-darwin/release/claude-usage`, kiểm tra bằng `lipo -archs` (phải ra `x86_64 arm64`).
5. `tar czf claude-usage-macos-universal.tar.gz claude-usage` rồi `actions/upload-artifact@v7` với `archive: false` để giữ bit thực thi.

Lý do: 1 runner, không phụ thuộc nhãn Intel đang bị thu hẹp dần; ripgrep chứng minh mã C qua `cc` dịch chéo x86_64 trên runner arm được; tar.gz giữ quyền file; lipo có sẵn trên runner.

Chưa kiểm chứng (cần chạy thật để đo): (a) `ring` dịch chéo x86_64 trên runner arm. BUILDING.md của ring không nói về macOS; ta chỉ dựa vào việc pcre2 của ripgrep dịch được. (b) Crate `cc` có tự lấy deployment target của rustc khi không đặt biến môi trường không; vì vậy nên đặt biến rõ ràng. (c) Binary universal sau `lipo` có còn chữ ký ad-hoc của linker cho lát arm64 không. Alacritty ký lại bằng `codesign --force --sign -` sau lipo; nên thêm bước này nếu binary bị macOS giết khi chạy.

## Nguồn

- https://raw.githubusercontent.com/BurntSushi/ripgrep/master/.github/workflows/release.yml
- https://raw.githubusercontent.com/sharkdp/bat/master/.github/workflows/CICD.yml
- https://raw.githubusercontent.com/axodotdev/cargo-dist/main/.github/workflows/release.yml
- https://raw.githubusercontent.com/axodotdev/cargo-dist/main/cargo-dist/src/backend/ci/github.rs
- https://raw.githubusercontent.com/alacritty/alacritty/master/.github/workflows/release.yml
- https://raw.githubusercontent.com/alacritty/alacritty/master/Makefile
- https://raw.githubusercontent.com/taiki-e/upload-rust-binary-action/main/main.sh
- https://github.com/taiki-e/upload-rust-binary-action
- https://github.com/actions/runner-images
- https://github.blog/changelog/2025-09-19-github-actions-macos-13-runner-image-is-closing-down/
- https://github.com/actions/checkout/releases
- https://github.com/actions/upload-artifact
- https://github.com/Swatinem/rust-cache
- https://github.com/dtolnay/rust-toolchain
- https://doc.rust-lang.org/nightly/rustc/platform-support/apple-darwin.html
- https://raw.githubusercontent.com/briansmith/ring/main/BUILDING.md
