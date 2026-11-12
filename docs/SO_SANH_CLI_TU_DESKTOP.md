> **Dropped on 2026-11-12** (docs/specs/no-foreign-credentials.md): making a Claude Code
> sign-in from Claude Desktop's looks like infostealer malware to Anthropic. Not to be built.

# Tạo đăng nhập Claude CLI từ đăng nhập Claude Desktop: so sánh cách làm

Hỏi 2026-11-12: "có cách nào chỉ cần đăng nhập ... nó tự tạo ra phiên đăng nhập cho claude
desktop không? tại tôi lười". Chiều CLI → Desktop không làm được: Desktop cần cookie
`sessionKey` của claude.ai, chỉ có khi đăng nhập trên claude.ai; mã OAuth của CLI không đổi
được thành cookie đó (không tìm thấy endpoint hay dự án nào làm được). Chiều Desktop → CLI
làm được: tài liệu này so sánh các cách.

## Đã đo trên máy này (chỉ đọc, không gửi gì đi)

- SCAS lưu phiên Desktop của mỗi tài khoản ở `~/.claude-usage-dashboard/desktop/profiles/<uuid>/`:
  `auth.json` (các khoá `oauth:*` của Desktop) và `Cookies` (có `sessionKey`).
- claude2 (`359a6468…`): đăng nhập CLI hỏng ("refresh failed (400)"); phiên Desktop đã lưu còn
  tới 9/12. Bộ đệm token đã lưu của Desktop có 2 mục cho client của Claude CLI (`9d1c250a…`):
  - scope kết thúc bằng `claude_code` (gọi model): hết hạn 12/11 16:03 (~8 h), có refresh token;
  - scope `user:profile`: hết hạn 11/11/2027, có refresh token (chỉ đọc hồ sơ/usage, không gọi model).
- SCAS đã có sẵn toàn bộ luồng OAuth PKCE của Claude Code (`src/login.rs`, `src/signin.rs`): tạo
  PKCE, URL cho phép, đổi `code` lấy token, lưu thành đăng nhập CLI. Chỉ thiếu bước tự qua
  trang "Authorize".

## Các cách

| Cách | Làm thế nào | Hợp với SCAS | Chi phí, rủi ro |
|---|---|---|---|
| A. [ccauth](https://github.com/synacktraa/ccauth) | Mở Chrome thật (patchright), nạp cookie claude.ai, tự bấm nút "Authorize", bắt `code` qua localhost | Kém: cần Chrome + patchright (Python), **bật cửa sổ** | Nặng; chờ Cloudflare Turnstile tới 60 s; gãy khi trang đổi nút |
| B. [claude-cookie-session](https://github.com/North-web-dev/claude-cookie-session) | HTTP thuần: `GET claude.ai/api/account` lấy org → `POST claude.ai/v1/oauth/{org}/authorize` với cookie `sessionKey` + PKCE → nhận `code` → `POST platform.claude.com/v1/oauth/token` | **Tốt**: ~60 dòng Rust (ureq), dùng lại `login.rs`; không cửa sổ; tạo một đăng nhập CLI **mới, riêng** (refresh token của riêng nó, không giành với Desktop) | Không chính thức: Anthropic đổi API là gãy; Cloudflare có thể chặn client không phải trình duyệt (chưa đo); scope `org:create_api_key` cần phiên "mới" (`session_stale_relogin`), lùi về scope chỉ cho Code |
| C. Dùng token Desktop đã lưu | Chép mục `claude_code` trong bộ đệm token của Desktop sang đăng nhập CLI | Vừa: không gọi mạng để lấy | Token gọi model chỉ sống ~8 h; làm mới thì **xoay mất** refresh token của bản Desktop đã lưu (Desktop chuyển về tài khoản đó có thể phải tự xin lại) |
| D. Như hiện nay | "Sign in again": mở trình duyệt mặc định, người dùng bấm "Authorize" | Có sẵn | Mỗi tài khoản một lần bấm, và trình duyệt phải đang đăng nhập đúng tài khoản claude.ai |

## Chọn

**B**, viết bằng Rust trong SCAS: khi đăng nhập CLI của một tài khoản bị từ chối và phiên
Desktop đã lưu của nó còn sống, SCAS tự lấy `sessionKey` từ `Cookies` đã lưu, xin `code`, đổi
lấy token và lưu thành đăng nhập CLI của tài khoản đó. Hết phiên Desktop (28 ngày không dùng,
hoặc bấm Log out trong Desktop) thì vẫn phải đăng nhập tay như cũ.

## Đã thử trên claude2 (2026-11-12 13:44)

- Lần 1: Cloudflare cho qua; `GET /api/account` 200 (đúng uuid claude2); `authorize` với scope có
  `org:create_api_key` bị 403 ("Account does not hold the permission … iam:claude-code-api-key:create"),
  scope chỉ cho Code được 200 kèm `code`; đổi lấy token 200: access 8 h + refresh token, scope
  `user:file_upload user:inference user:mcp_servers user:profile user:sessions:claude_code`; profile
  đúng uuid; usage 200.
- Lần 2 (4 phút sau, cùng cookie đọc lại từ hồ sơ đã lưu): `GET /api/account` 403
  `account_session_invalid`. Phiên Desktop đã lưu của claude2 không còn dùng được.
- Chưa rõ vì sao. Giả thuyết đầu: claude.ai **xoay** `sessionKey` khi được dùng (trả cookie mới qua
  `Set-Cookie`, huỷ cookie cũ); lần 1 không ghi lại cookie mới nên bản lưu thành cũ. Nếu đúng thì
  mọi lần tạo phải ghi cookie mới về hồ sơ Desktop đã lưu, nếu không sẽ làm hỏng đăng nhập
  Desktop. Cần đo `Set-Cookie` trên một tài khoản khác trước khi viết code.

Trước khi viết: thử một lần trên claude2 (tạo một đăng nhập CLI thật cho claude2) để đo xem
Cloudflare có cho qua không, và API trả `code` theo dạng nào.
