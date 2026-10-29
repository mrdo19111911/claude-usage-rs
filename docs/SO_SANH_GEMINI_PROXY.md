# So sánh proxy Anthropic → Gemini cho Claude Code

Cập nhật 2026-10-29. Mục tiêu: `cgemini.cmd` đặt `ANTHROPIC_BASE_URL=http://127.0.0.1:<port>` rồi chạy `claude`. App nhận `POST /v1/messages`, dịch sang `generativelanguage.googleapis.com/v1beta/models/<m>:streamGenerateContent?alt=sse` (key AI Studio), rồi stream ngược lại dạng Anthropic SSE.
Đã đọc mã nguồn thật của 5 dự án (dòng ghi theo nhánh `main`/`HEAD` hôm nay) và tài liệu chính thức của Claude Code và Gemini.

## Bảng so sánh

| Dự án | Ngôn ngữ | Tools/schema | Thought signature | Streaming | Mức phù hợp | Chi phí chuyển |
|---|---|---|---|---|---|---|
| [musistudio/claude-code-router](https://github.com/musistudio/claude-code-router), lõi dịch [musistudio/llms](https://github.com/musistudio/llms) | TS | Gửi nguyên schema vào `parametersJsonSchema` (`src/utils/gemini.util.ts` dòng 252). Hàm làm sạch `cleanupParameters` (dòng 4) chỉ chạy cho `parameters` | Đặt vào `signature` của khối `thinking`, Claude Code gửi lại khối đó. Gắn chữ ký vào functionCall **đầu tiên** (dòng 330–345) | Gemini → OpenAI chunk → Anthropic SSE (`anthropic.transformer.ts`). **Không gửi ping** | Cao, mẫu đầy đủ nhất | Trung bình, đi vòng qua định dạng OpenAI |
| [1rgs/claude-code-proxy](https://github.com/1rgs/claude-code-proxy) | Python + LiteLLM | `clean_gemini_schema` (`server.py` dòng 132): bỏ `additionalProperties`, `default`; `format` chỉ giữ `enum`/`date-time` (dòng 141) | Không xử lý | Tự sinh SSE: `message_start` (dòng 946), `ping` (dòng 952). Khối text luôn ở index 0, tool từ 1 | Vừa. Đơn giản, nhưng tool_result bị ép thành text | Thấp |
| [fuergaosi233/claude-code-proxy](https://github.com/fuergaosi233/claude-code-proxy) | Python | Chỉ dịch sang API kiểu OpenAI, không có nhánh riêng cho Gemini | Không xử lý | `src/conversion/response_converter.py`: start → ping → text 0 → tool 1..n. Lỗi gửi `event: error` | Thấp, chỉ tham khảo thứ tự SSE | Thấp |
| [BerriAI/litellm](https://github.com/BerriAI/litellm) `/v1/messages` adapter | Python (có thêm `litellm-rust`) | `build_vertex_schema` (`llms/vertex_ai/common_utils.py` dòng 840–885): gỡ `$defs`/`$ref`, `anyOf`+null → `nullable`, enum chỉ cho string, lọc theo danh sách khóa trắng (`filter_schema_fields` dòng 1004) | Nhét vào id: `call_x__thought__<sig>` (`factory.py` dòng 92, 1247). Khi trả về Anthropic thì id bị chuẩn hóa và mất chữ ký (`anthropic/common_utils.py` dòng 1894), nên phải dùng chữ ký giả base64 `skip_thought_signature_validator` (`factory.py` dòng 1327–1340), chỉ cho call đầu (dòng 1405–1416) | Adapter chung (`adapters/streaming_iterator.py`) | Cao về độ đúng schema | Cao, mã rất lớn |
| [skyline69/agcp](https://github.com/skyline69/agcp) | **Rust** | Lọc theo danh sách khóa trắng `type, description, properties, required, items, enum`. `const` → `enum`, `anyOf`/`oneOf` → nhánh khác null đầu tiên (`src/format/to_google.rs` dòng 325–460) | **Cache theo tool_use id** (`signature_cache.rs`, TTL 2h, tối đa 1000 mục). Gặp id lạ thì dùng `skip_thought_signature_validator` (`to_google.rs` dòng 176–196, `sse.rs` dòng 461) | `cloudcode/sse.rs`: một bộ đếm block, thinking → `signature_delta`, ánh xạ finish (dòng 474–477) | Cao, cùng ngôn ngữ. Nhưng upstream là Cloud Code chứ không phải AI Studio | Thấp, có thể chép logic |

## 1. Claude Code gọi gì tới `ANTHROPIC_BASE_URL`

Nguồn: [gateway compatibility guide](https://code.claude.com/docs/en/llm-gateway-protocol), [env-vars](https://code.claude.com/docs/en/env-vars).
- `POST /v1/messages?beta=true`. So khớp theo **path**, bỏ qua query. Có kèm header `anthropic-version`, `anthropic-beta`.
- `POST /v1/messages/count_tokens` là tùy chọn. Nếu thiếu, Claude Code tự ước lượng theo số ký tự. agcp tự đếm ký tự (`server.rs` dòng 278, 3133).
- `HEAD /api/hello`: probe làm ấm kết nối, từ chối cũng không sao.
- `GET /v1/models?limit=1000`: timeout 3 s. Chỉ chạy khi `CLAUDE_CODE_ENABLE_GATEWAY_MODEL_DISCOVERY=1`, và chỉ giữ id có chứa `claude`/`anthropic`. Với Gemini nên bỏ qua.
- agcp còn trả 200 cho `POST /api/event_logging/batch` và `POST /` (`server.rs` dòng 305–316). Khi đã tắt nonessential traffic thì hai đường này không xuất hiện.
- Luồng stream phải đủ chuỗi sự kiện đến `message_delta` và `message_stop`, nếu không Claude Code coi là rớt kết nối. Có watchdog idle, nên khi upstream im lặng proxy phải tự gửi `ping`.
- `retry-after` trả về dạng giây nguyên. Giá trị trên 60 khiến Claude Code dừng thử lại và báo lỗi ngay.
- Với model id lạ, Claude Code gửi thêm `role:"system"` **giữa** `messages`. Proxy phải nhận được.
- Về khóa: `ANTHROPIC_AUTH_TOKEN` đi thành `Authorization: Bearer …` và không cần duyệt. `ANTHROPIC_API_KEY` đi thành `x-api-key` và ở chế độ tương tác **sẽ hỏi duyệt một lần** (đó là lời nhắc "detected custom API key"). Khi có biến khóa gateway, khóa này thay cho đăng nhập claude.ai OAuth. Chỉ đặt `ANTHROPIC_BASE_URL` thì token OAuth vẫn được gửi tới proxy. CCR hiện dùng `apiKeyHelper` và tự đặt `CLAUDE_CODE_DISABLE_UNKNOWN_MODEL_WINDOW_ENFORCEMENT=1` (`packages/core/src/profiles/service.ts` dòng 2334).
- Biến model (CCR `agents/claude-code/environment.ts` dòng 3–10): `ANTHROPIC_MODEL`, `ANTHROPIC_DEFAULT_{OPUS,SONNET,HAIKU,FABLE}_MODEL`. `ANTHROPIC_SMALL_FAST_MODEL` đã bị đánh dấu deprecated.
- Biến khác: `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1`, `CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1` (bỏ các trường beta như `defer_loading`), `CLAUDE_CODE_ATTRIBUTION_HEADER=0` (bỏ khối fingerprint ở đầu system), `ENABLE_TOOL_SEARCH=false`.

## 2. Dịch request

- `system`: là string hoặc mảng block. Nối các block text thành `systemInstruction.parts[{text}]`, bỏ `cache_control`. Message `role:"system"` giữa hội thoại thì đổi thành text của lượt `user`.
- Role: `user` → `user`, `assistant` → `model`. Gộp các lượt liền nhau cùng role.
- Block:
  - `text` → `{text}`.
  - `image` base64 → `{inlineData:{mimeType,data}}` (agcp `to_google.rs` dòng 162).
  - `tool_use` → `{functionCall:{name,args}}`.
  - `tool_result` → `{functionResponse:{name,response:{result|error}}}`.
  - `thinking` → bỏ, hoặc lấy `signature` của nó làm `thoughtSignature` (CCR `gemini.util.ts` dòng 290).
- **Tên hàm cho functionResponse** lấy theo `tool_use_id`: dò ngược lượt assistant trước để tìm `tool_use` có cùng id. LiteLLM làm vậy (`factory.py` ≈ dòng 1708–1730), không thấy thì ném lỗi. CCR cũng vậy (`gemini.util.ts` dòng 361–366). agcp lấy luôn id làm tên (`to_google.rs` dòng 231), đó là điểm yếu.
- `generationConfig`: `maxOutputTokens` (agcp chặn ≤ 65536), `temperature`, `topP`, `topK`, `stopSequences`. Khi `thinking` bật: `thinkingConfig.includeThoughts=true`. Gemini 3 dùng `thinkingLevel` theo effort. Gemini 2.5 dùng `thinkingBudget` (CCR dòng 387–390; budget nằm trong [128, 32768] cho pro, [0, 24576] cho flash).
- Tools → `tools:[{functionDeclarations:[{name,description,parametersJsonSchema}]}]`. Từ 11/2025 Gemini nhận JSON Schema cho mọi model đang hỗ trợ ([blog Google](https://blog.google/technology/developers/gemini-api-structured-outputs/)), và CCR gửi nguyên schema. Nếu dùng trường `parameters` kiểu OpenAPI cũ thì phải lọc khóa như 3 dự án còn lại làm. Bỏ các tool không có `input_schema` (server tool như web_search, advisor).
- `tool_choice` → `toolConfig.functionCallingConfig`: `auto` → `AUTO`, `any` → `ANY`, `{type:tool,name}` → `ANY` + `allowedFunctionNames:[name]`, `none` → `NONE` (agcp `to_google.rs` dòng 301–322; [API ref](https://ai.google.dev/api/caching)).

## 3. Thought signature (Gemini 3)

Nguồn: [Vertex/Gemini docs](https://docs.cloud.google.com/gemini-enterprise-agent-platform/models/thinking/thought-signatures).
- Với Gemini 3, thiếu chữ ký bắt buộc trên functionCall của **lượt hiện tại** sẽ bị **400**. "Lượt hiện tại" bắt đầu từ user message gần nhất mà không phải functionResponse.
- Gọi song song thì chỉ functionCall **đầu tiên** có chữ ký. Gọi nối tiếp thì mỗi bước đều có.
- Chữ ký ở phần text cuối là tùy chọn. Khi stream, chữ ký có thể nằm trong một part có text rỗng.
- Thiếu thật thì dùng `skip_thought_signature_validator`, nhưng đây là "last resort" vì làm giảm chất lượng.
- FinishReason có thêm giá trị `MISSING_THOUGHT_SIGNATURE` ([generate-content ref](https://ai.google.dev/api/generate-content)).
- Claude Code chỉ giữ `id/name/input` của tool_use, và giữ khối `thinking` kèm `signature`. Ba cách mang chữ ký qua:
  1. cache theo id (agcp);
  2. nhét vào signature của khối thinking (CCR);
  3. nhét vào id (LiteLLM). Cách này hỏng với Anthropic vì id phải khớp `^[a-zA-Z0-9_-]+$`.

## 4. Streaming ngược

- Mỗi chunk Gemini: `data: {"candidates":[{"content":{"role":"model","parts":[…]},"finishReason":…}],"usageMetadata":{promptTokenCount,candidatesTokenCount,thoughtsTokenCount,cachedContentTokenCount,totalTokenCount}}`.
- Loại part:
  - `{text}`;
  - `{text,thought:true}`;
  - `{functionCall:{name,args},thoughtSignature?}`. functionCall đến **nguyên khối**, không chia mảnh.
- Thứ tự Anthropic (agcp `sse.rs` dòng 285–520; 1rgs dòng 946–1160):
  - `message_start`;
  - `ping`;
  - với mỗi khối: `content_block_start(i)` → `content_block_delta(i)` (`thinking_delta` / `text_delta` / `input_json_delta`) → (`signature_delta` cho thinking) → `content_block_stop(i)`;
  - `message_delta{stop_reason, usage}`;
  - `message_stop`.
- Index lấy từ **một bộ đếm** tăng dần theo thứ tự xuất hiện (CCR `assignContentBlockIndex`, agcp `block_index`). Đổi loại part thì đóng khối cũ.
- finishReason:
  - `STOP` → `end_turn`, nhưng thành `tool_use` nếu đã phát functionCall (agcp đặt `stop_reason="tool_use"` khi gặp call, dòng 422).
  - `MAX_TOKENS` → `max_tokens`.
  - `SAFETY`/`RECITATION`/`BLOCKLIST`/`OTHER` thì agcp trả sự kiện error (dòng 230).
- Lỗi:
  - Trước khi stream: trả mã HTTP kèm `{"type":"error","error":{"type":"rate_limit_error",…}}`.
  - Giữa stream: `event: error` + `data:{"type":"error","error":{"type":"api_error","message":…}}` (fuergaosi233). Riêng CCR viết sai thành `message:{…}`.
- Usage: `input_tokens = promptTokenCount − cachedContentTokenCount`, `cache_read_input_tokens = cached` (agcp, CCR).

## 5. Model Gemini (ai.google.dev, đọc ngày 2026-10-29)

- Theo [models](https://ai.google.dev/gemini-api/docs/models) và [pricing](https://ai.google.dev/gemini-api/docs/pricing):

  | Model | Trạng thái | Free tier | Giá trả phí (input/output, mỗi 1M token) |
  |---|---|---|---|
  | `gemini-4-argon` | stable | không | $2 / $10 (tăng sau 30/11/2026) |
  | `gemini-3.8-flash` | stable | có | $0.75 / $3.75 |
  | `gemini-3.6-flash` | stable | có | $0.75 / $3.75 |
  | `gemini-3.5-flash-lite` | stable | có | $0.30 / $2.50 |
  | `gemini-3.1-flash-lite` | stable | có | — |
  | `gemini-3-flash-preview` | preview | có | — |
  | `gemini-3.1-pro-preview` | preview | không | — |

- Họ `gemini-2.5-*` chỉ còn cho người đã dùng trước đây. `gemini-3-pro-preview` và `gemini-2.0-*` đã tắt.
- RPM/RPD của free tier chỉ xem được trong AI Studio ([rate-limits](https://ai.google.dev/gemini-api/docs/rate-limits)).
- Mặc định của các dự án:
  - 1rgs: `gpt-4.1`, danh sách Gemini là `gemini-2.5-flash`/`-pro` (`server.py` dòng 108, 128).
  - agcp: `flash` → `gemini-3-flash`, `pro` → `gemini-3-pro-high` (Cloud Code).
  - CCR, LiteLLM: không có mặc định.

## Chọn

1. **Route** (tiny_http, so khớp path, bỏ query; kiểm `Authorization: Bearer <token>` hoặc `x-api-key`):
   - `POST /v1/messages`: dịch rồi stream.
   - `POST /v1/messages/count_tokens`: trả `{"input_tokens": ký_tự/4}`.
   - `HEAD /api/hello`, `GET /`: trả 200.
   - `GET /v1/models`: trả `{"data":[]}`.
   - Path khác: 404 dạng error Anthropic.
2. **Schema**:
   - Gửi `parametersJsonSchema`.
   - Luôn bỏ `$schema`, `$id`, `$comment`, `examples`. Đổi `const` thành `enum:[v]`.
   - Gemini trả 400 nhắc tới schema → thử lại bằng `parameters` với danh sách trắng: `type, format(enum|date-time), description, nullable, enum(string), properties, required, items, anyOf, minItems, maxItems, minimum, maximum, minLength, maxLength, pattern`.
   - Ở đường này còn bỏ `additionalProperties, default, exclusiveMinimum/Maximum, oneOf→anyOf, allOf, not, $ref/$defs (inline trước), patternProperties, propertyNames, title`.
   - `type:[T,"null"]` → `T` + `nullable`. `required` chỉ giữ khóa có thật.
3. **Thought signature**:
   - Lưu `HashMap<tool_use_id,(sig,Instant)>`, TTL 2h, tối đa 1000 mục.
   - Khi dựng request: lấy chữ ký từ cache. Không có thì gắn `skip_thought_signature_validator` cho functionCall đầu mỗi lượt `model`.
   - Tự sinh id `toolu_<24hex>`. Không gửi `id` lên Gemini.
4. **Request**: làm như mục 2. Tên functionResponse tra theo id. Message `system` giữa hội thoại đổi thành text của lượt user. `maxOutputTokens ≤ 65536`.
5. **SSE**:
   - Ngay khi Gemini trả 200: `message_start` (usage 0) và `ping`.
   - Sau đó các khối đánh index bằng một bộ đếm. Mỗi functionCall là một khối `tool_use` với **một** `input_json_delta` chứa JSON đầy đủ.
   - Cuối: `message_delta` (usage đầy đủ) và `message_stop`.
   - Gửi `ping` mỗi 10 s khi im lặng.
   - Ánh xạ finish:
     - `STOP` → `end_turn`, hoặc `tool_use` nếu đã có call.
     - `MAX_TOKENS` → `max_tokens`.
     - Nhóm an toàn → `refusal`.
     - `MALFORMED_FUNCTION_CALL`/`MISSING_THOUGHT_SIGNATURE` → `event: error` api_error.
6. **Lỗi HTTP**:
   - 400 → `invalid_request_error`; 401/403 → `authentication_error`/`permission_error`.
   - 429 → `rate_limit_error`, kèm `retry-after` lấy từ `RetryInfo.retryDelay`.
   - 500 → `api_error`; 503 → 529 `overloaded_error`.
7. **cgemini.cmd** (app ghi lại file này mỗi khi đổi port):
   - Kết nối: `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN=<token>`, `set ANTHROPIC_API_KEY=` (xóa biến), `NO_PROXY=127.0.0.1,localhost`.
   - Model: `ANTHROPIC_MODEL` và `ANTHROPIC_DEFAULT_{OPUS,SONNET,FABLE}_MODEL` = `gemini-3.8-flash`. `ANTHROPIC_DEFAULT_HAIKU_MODEL` và `ANTHROPIC_SMALL_FAST_MODEL` = `gemini-3.5-flash-lite`.
   - Tắt bớt: `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1`, `CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1`, `CLAUDE_CODE_ATTRIBUTION_HEADER=0`, `CLAUDE_CODE_DISABLE_UNKNOWN_MODEL_WINDOW_ENFORCEMENT=1`, `ENABLE_TOOL_SEARCH=false`.
   - Cuối file: `claude %*`.
8. **Model mặc định**: `gemini-3.8-flash` (stable, có free tier). Model nhanh: `gemini-3.5-flash-lite`. Proxy nhận `claude-*`: tên chứa `haiku` → model nhanh, còn lại → model chính.

## Onboarding (dán API key)

Đọc ngày 2026-10-29, mã nguồn trên GitHub:

| Dự án (file) | Cách làm | Hợp với ta | Chi phí |
|---|---|---|---|
| Gemini CLI (`packages/cli/src/ui/auth/ApiAuthDialog.tsx`) | Hộp thoại "Enter Gemini API Key", ghi link `https://aistudio.google.com/app/apikey`, ô "Paste your API key here", lỗi hiện ngay dưới ô, Enter gửi / Esc hủy, key vào keychain. | Cao: cùng Google, cùng link, cùng chữ. | Thấp |
| Cline (`webview-ui/.../common/ApiKeyField.tsx`) | Một ô `type="password"`, nhãn "`<Provider>` API Key", link đăng ký bên dưới; lưu ngay khi gõ, không kiểm key. | Vừa: ô password + link; không kiểm key thì người dùng chỉ biết sai khi chạy. | Thấp |
| Continue (`gui/src/pages/AddNewModel/configs/providers.ts`) | Thẻ "Google Gemini": một câu "lấy key ở đây rồi dán vào", `apiKeyUrl` = trang AI Studio, ô nhập (text). | Vừa: một câu giải thích + link; ô text lộ key. | Thấp |
| LibreChat (`client/.../SetKeyDialog/SetKeyDialog.tsx`) | Dialog "key for Google": ô key, chọn hạn dùng (12 h mặc định), Submit, Revoke có hỏi lại; lỗi bằng toast. | Thấp: hạn dùng và toast thừa cho app một người. Lấy ý "Remove có hỏi lại". | Vừa |

Chọn: dialog kiểu Gemini CLI (link AI Studio, ô "Paste your API key here", lỗi ngay dưới ô,
Enter = Done, Esc = Cancel) + ô `password` có nút Show của Cline + Remove có hỏi lại của
LibreChat. Thêm: mở sẵn trang AI Studio ở tab mới, ba bước đánh số, app kiểm key với Google
trước khi lưu (Cline, Continue không kiểm) nên có trạng thái "Checking the key…".

## Proxy và PATH

Đọc mã nguồn ngày 2026-10-29. Hai việc: (a) stream SSE từ tiny_http mà không bị giữ lại, (b) thêm thư mục `bin` vào PATH của người dùng.

| Dự án | Cách làm | Mức phù hợp | Chi phí |
|---|---|---|---|
| [skyline69/agcp](https://github.com/skyline69/agcp) `src/server.rs` (Rust, hyper) | Body là một `mpsc` (64 mục), mỗi chunk SSE thành một frame hyper ghi ngay. Token: `Authorization: Bearer` hoặc `x-api-key`, so sánh `!=` thường (không hằng thời gian). Heartbeat `": ping"` mỗi 15 s; ngắt khi upstream im 300 s | Cao về ý: một luồng đọc upstream, một kênh, ping khi im | Cao nếu chép nguyên: cần tokio + hyper, app này dùng tiny_http đồng bộ |
| [musistudio/claude-code-router](https://github.com/musistudio/claude-code-router) (TS, Node) | Node ghi từng chunk của ReadableStream ra socket ngay. Khóa: `Authorization: Bearer` hoặc `x-api-key` (tài liệu extensions) | Vừa: chỉ xác nhận cách nhận token | Không dùng được mã (Node) |
| tiny_http 0.12 (`response.rs` `raw_print`, chunked_transfer 1.5 `Encoder::new` = 8192 byte) | `Response` với reader → `io::copy` vào `Encoder` chunk 8 KB, không flush giữa chừng. Đo bằng `sse_tests.rs`: 4 sự kiện cách nhau 500 ms đến cùng lúc ở 2002–2004 ms. Không có issue nào trên GitHub về chuyện này | Thấp: giữ SSE lại | — |
| tiny_http `Request::into_writer` + tự ghi head và chunk | Ghi `HTTP/1.1 200`, `Content-Type: text/event-stream`, `Cache-Control: no-cache`, `Connection: close`, `Transfer-Encoding: chunked`; mỗi sự kiện một chunk rồi `flush`; cuối `0\r\n\r\n`. Đo: sự kiện đến ở 501/1001/1502/2002 ms (sinh ở 500/1000/1500/2000), client đọc được EOF | **Chọn** | Thấp: ~60 dòng (`sse.rs`) |
| [rustup](https://github.com/rust-lang/rustup) `src/cli/self_update/windows.rs` | Crate `windows_registry`: đọc `PATH` ở HKCU\Environment (thiếu → rỗng, không phải chuỗi → bỏ qua), ghi `REG_EXPAND_SZ`, rỗng thì xóa giá trị. Thêm ở **đầu**, so khớp phân biệt hoa thường. Rồi `SendMessageTimeoutA(HWND_BROADCAST, WM_SETTINGCHANGE, 0, "Environment", SMTO_ABORTIFHUNG, 5000)` | Cao | Thấp: windows-sys đã có, chỉ thêm feature `Win32_System_Registry` |
| [ScoopInstaller/Scoop](https://github.com/ScoopInstaller/Scoop) `lib/system.ps1` | .NET registry, đọc với `DoNotExpandEnvironmentNames`; kiểu `ExpandString` nếu có `%`, không thì giữ kiểu cũ. `Add-Path` thêm ở đầu, chỉ khi chưa có (so `-like`, không phân biệt hoa thường). `Publish-EnvVar`: `SendMessageTimeout(0xffff, 0x1a, 0, "Environment", 2, 5000)`. Rỗng thì xóa giá trị | Cao | Không dùng được mã (PowerShell) |

Chọn:
- Stream: `into_writer`, tự chia chunk và flush từng sự kiện (`src/gemini/sse.rs`). Một luồng đọc dòng SSE của Google vào `sync_channel(16)`; phía ghi `recv_timeout(10 s)`, hết giờ thì gửi `ping` như agcp (`relay.rs`). Client đi mất thì ghi lỗi, luồng đọc gửi thất bại và tự dừng, kết nối tới Google đóng theo.
- Token: như agcp/CCR (Bearer hoặc `x-api-key`) nhưng so sánh hằng thời gian.
- PATH: như rustup (HKCU\Environment, `REG_EXPAND_SZ`, xóa giá trị khi rỗng, `SendMessageTimeoutW` … 5000 ms). Khác rustup: thêm ở **cuối** (không che chương trình khác) và so khớp không phân biệt hoa thường, bỏ `\` cuối, như Scoop. Hàm thuần `path_with`/`path_without` có test; phần registry không chạy trong test.

## Ghi chú khi viết code

Phần dịch (`request*.rs`, `schema*.rs`, `sigs.rs`, `stream*.rs`, `errors.rs`), viết ngày 2026-10-29 sau khi đọc lại mã nguồn agcp (`to_google.rs`, `cloudcode/sse.rs`, `format/signature_cache.rs`), CCR `llms` (`gemini.util.ts`), LiteLLM (`vertex_ai/common_utils.py`, `prompt_templates/common_utils.py` `unpack_defs`, `factory.py`). Những điểm bảng trên chưa ghi:
- **Chữ ký giả**: agcp và tài liệu Google gửi nguyên chuỗi `skip_thought_signature_validator`. LiteLLM gửi bản base64 của chuỗi đó (`factory.py` dòng 1340). Chuỗi gốc dài 32 ký tự và đều là ký tự base64 hợp lệ, nên cả hai cách đều được nhận. Code dùng chuỗi gốc. LiteLLM chỉ gắn chữ ký giả cho Gemini 3 trở lên. Code cũng vậy: model `gemini-1*`/`gemini-2*` không được gắn.
- **Thứ tự lấy chữ ký** cho functionCall đầu mỗi lượt `model`: cache theo id, rồi `signature` của khối `thinking` cùng lượt (CCR), cuối cùng mới là chữ ký giả. Các call sau trong cùng lượt (gọi song song) chỉ lấy từ cache.
- **Lúc stream**: chữ ký trên part không phải call (thường là text rỗng) được giữ lại, rồi gán cho call kế tiếp nếu call đó không có chữ ký riêng. Chữ ký trên part thought thành `signature_delta` của khối thinking, như agcp. Text rỗng không mở khối nào.
- **Usage**: `candidatesTokenCount` **không** gồm `thoughtsTokenCount` (tài liệu generate-content). agcp chỉ lấy candidates. Code lấy `output_tokens = candidates + thoughts`, vì cả hai đều tính tiền như output.
- **Danh sách finishReason** (generate-content ref) dài hơn bảng trên:
  - `refusal`: `SAFETY, RECITATION, BLOCKLIST, PROHIBITED_CONTENT, SPII, LANGUAGE, IMAGE_*`, và `promptFeedback.blockReason`.
  - `event: error`: `MALFORMED_FUNCTION_CALL, MISSING_THOUGHT_SIGNATURE, UNEXPECTED_TOOL_CALL, TOO_MANY_TOOL_CALLS, MALFORMED_RESPONSE`.
  - Không có finishReason (stream bị cắt): cũng là `event: error`, để câu trả lời dở dang không bị coi là xong.
- **tool_result mất tool_use** (lịch sử bị cắt khi compact): LiteLLM ném lỗi. Code đổi nó thành text `Tool result (<id>): …` để request vẫn đi được.
- **Ảnh trong tool_result** (Read một file ảnh): thành part `inlineData` riêng, đặt ngay sau `functionResponse` trong cùng lượt user. agcp bỏ ảnh, chỉ giữ text.
- **Schema `parameters`**: theo LiteLLM, khi có `anyOf` thì node chỉ giữ `anyOf` (Vertex từ chối khóa đi kèm, LiteLLM #11164). Mô tả của node được chép vào từng nhánh. Thêm vào danh sách chọn:
  - `enum` chỉ giữ trên string, `format` chỉ trên string;
  - array thiếu `items` thì thêm `{type:object}`;
  - `properties` rỗng thì bỏ;
  - không có `type` thì đặt `object` (hoặc `string` nếu có `enum`).
  - Riêng `allOf` thì **gộp** `properties`/`required` của các nhánh thay vì bỏ, vì bỏ sẽ mất hết thuộc tính của schema dựng bằng `allOf`.
  - `$ref` lặp vòng thì dừng ở `{type:object}`, như `unpack_defs` để nguyên ref vòng.
- **Lỗi**: Google trả key sai là 400 `INVALID_ARGUMENT` kèm `ErrorInfo.reason = API_KEY_INVALID`. Code đổi thành 401 `authentication_error`. `retryDelay` lẻ ("1.2s") được làm tròn lên. 404 → `not_found_error`, 413 → `request_too_large`.
- **Trang structured-output** của Gemini: nhóm khóa `minLength/maxLength/minProperties/maxProperties/uniqueItems/contains` bị "strip và bỏ qua" chứ không báo lỗi. Trang không nói gì về `$ref`/`$defs`/`const` và không nói quy tắc này có áp cho `parametersJsonSchema` không. Vì vậy `clean` chỉ bỏ những gì đã chọn ở trên, phần còn lại để đường thử lại bằng `parameters` lo.
