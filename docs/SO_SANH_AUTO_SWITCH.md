# So sánh cách tự đổi account theo quota

Vấn đề: `pick_auto` trong `src/switch.rs` chỉ đổi khi account đang dùng chạm 5h ≥ 95% hoặc
tuần ≥ 99%, rồi chọn account có `min(100−5h, 100−tuần)` lớn nhất. Nó bỏ qua `resets_at`,
nên quota tuần của account sắp reset bị bỏ phí (phần chưa dùng mất lúc reset). Hướng định
làm: Earliest-Deadline-First (EDF): trong các account còn chỗ, dùng account có cửa sổ tuần
reset sớm nhất; quy 5h sang đơn vị tuần bằng k (1% tuần ≈ 6,7 × 1% 5h); đổi chủ động, có
chống đổi qua lại. Đã đọc code của 4 dự án (2026-11-06, link ghim theo commit).

| Dự án | Cách chọn account | Dùng giờ reset? | Đổi trước/chống đổi qua lại | Mức phù hợp | Chi phí |
|---|---|---|---|---|---|
| [realiti4/claude-swap](https://github.com/realiti4/claude-swap/blob/5bded0f30d82fe1eb53dc159250ded366fb7d226/src/claude_swap/autoswitch.py#L2101) (Python, đổi login Claude Code như app này) | 4 chiến lược: `best` (mặc định, headroom lớn nhất so với ngưỡng riêng), `weekly-first`, `consume-first`, `priority-consume-first`. Ứng viên phải dưới mọi ngưỡng ("landing must be healthy") | Có: `consume-first` xếp theo `seven_day.resets_at` sớm nhất, hòa thì theo headroom; reset không biết hoặc đã qua thì xếp cuối. Dưới ngưỡng vẫn đổi, nhưng chỉ sang account reset **sớm hơn hẳn** account đang dùng | Có cả hai: `threshold=90`, `hysteresis_pct=10`, `cooldown_seconds=300` (bỏ qua khi đã chạm giới hạn), "no-return bar" không quay về account vừa rời, `RECOVERY_HYSTERESIS_S=300`. Khi mọi account đều quá ngưỡng: bỏ xếp theo reset, chọn theo chỗ còn thô `100−max%`, biên `min(2×active, active+3)` điểm ([L115–L178](https://github.com/realiti4/claude-swap/blob/5bded0f30d82fe1eb53dc159250ded366fb7d226/src/claude_swap/autoswitch.py#L115-L178), [settings](https://github.com/realiti4/claude-swap/blob/5bded0f30d82fe1eb53dc159250ded366fb7d226/src/claude_swap/settings.py#L57-L66)) | Rất cao: cùng bài toán, cùng dữ liệu (5h, 7d, `resets_at`). Không có hệ số quy đổi giữa 2 cửa sổ ("no exchange rate") | 2746 dòng; nhiều trường hợp biên, ta chỉ lấy quy tắc |
| [tombii/better-ccflare](https://github.com/tombii/better-ccflare/blob/629c53198cbb2bf94cfbacd2a97ae17a603a6043/packages/load-balancer/src/strategies/session-drain-soonest.ts#L106-L160) (TypeScript, proxy nhiều account) | `session-drain-soonest`: so sánh theo reset tuần sớm nhất → `priority` → utilization; chế độ `strict` thêm "đang có session" làm tie-break, cuối cùng theo id cho ổn định | Có: reset tuần **làm tròn tới phút** (API báo lệch ~1,5 s, lệch đó từng đè cả `priority`); reset đã qua = không biết, xếp cuối | Chế độ `sticky`: session 5h đang chạy không bao giờ bị cướp vì xếp hạng, chỉ xếp lại khi session hết hoặc account hết chỗ. Ngưỡng tạm dừng theo từng account cho 5h và tuần; chỉ mở lại khi **mọi** cửa sổ đã dưới ngưỡng ([usage-threshold.ts](https://github.com/tombii/better-ccflare/blob/629c53198cbb2bf94cfbacd2a97ae17a603a6043/packages/core/src/usage-threshold.ts#L116-L148)). 429: chờ 30 s, gấp đôi mỗi lần, tối đa 5 phút; nếu 429 có giờ reset thì nghỉ tới giờ đó (tối đa 8 ngày) ([constants.ts](https://github.com/tombii/better-ccflare/blob/629c53198cbb2bf94cfbacd2a97ae17a603a6043/packages/core/src/constants.ts#L66-L80)) | Cao: đúng EDF; nhưng chỉ coi account là "còn/hết" (≥100% mới loại), không cân lượng còn lại | Không lấy code (proxy), chỉ lấy quy tắc |
| [router-for-me/CLIProxyAPI](https://github.com/router-for-me/CLIProxyAPI/blob/c2f402b45643f0740d70b9010fd7d8d0aab7ef65/sdk/cliproxy/auth/selector.go#L821) (Go, proxy nhiều OAuth) | `round-robin` (mặc định), `weighted-round-robin`, `fill-first` (luôn lấy account đầu tiên còn dùng được, dùng cạn rồi mới sang cái sau), `session-affinity` | Không khi chọn; chỉ dùng `NextRecoverAt` để loại account đang nghỉ và báo lỗi khi mọi account cùng nghỉ | Chỉ đổi sau 429: theo `Retry-After` (tối thiểu 10 s), không có thì 1 s gấp đôi tới 30 phút ([nextQuotaCooldown](https://github.com/router-for-me/CLIProxyAPI/blob/c2f402b45643f0740d70b9010fd7d8d0aab7ef65/sdk/cliproxy/auth/conductor_cooldown.go#L3208)). Không có hysteresis | Thấp: phản ứng sau lỗi; `fill-first` thì giống cách cũ của app | — |
| [BerriAI/litellm](https://github.com/BerriAI/litellm/blob/b02bb99b088601407232ae617d832f59a41558b5/litellm/router_strategy/lowest_tpm_rpm_v2.py#L348) Router | `usage-based-routing-v2`: TPM thấp nhất trong phút hiện tại; bỏ deployment nếu `tpm+input > giới hạn` hoặc `rpm+1 ≥ giới hạn`. `least-busy`: ít request đang chạy nhất | Không: cửa sổ cố định 1 phút (khóa `%H-%M`, TTL 60 s) | Cooldown sau lỗi ([cooldown_handlers.py](https://github.com/BerriAI/litellm/blob/b02bb99b088601407232ae617d832f59a41558b5/litellm/router_utils/cooldown_handlers.py#L479)): 429 thì nghỉ ngay (nếu nhóm có >1 deployment), tỷ lệ lỗi >50% với ≥5 request/phút, mặc định nghỉ 5 s, `allowed_fails=3` | Thấp: quota theo phút, không có cửa sổ tuần | — |

Ngoài bảng: [claude-code-router](https://github.com/musistudio/claude-code-router) chọn provider/model theo
điều kiện trên header/body và danh sách fallback theo thứ tự, không xét quota hay reset.

## Chọn

Giữ EDF và lấy quy tắc của `consume-first` (claude-swap) cùng `session-drain-soonest` (better-ccflare).
Hai dự án này đã chạy thật với đúng loại dữ liệu này:

- **Khóa xếp hạng**: `resets_at` tuần làm tròn tới phút → chỗ còn (quy 5h sang tuần bằng k) → thứ tự
  trong danh sách. Reset không biết hoặc đã qua thì xếp cuối, không coi là "sớm nhất".
- **Nơi đến phải khỏe**: ứng viên phải dưới mọi ngưỡng; giữ `has_room`.
- **Đổi chủ động dưới ngưỡng** chỉ sang account reset *sớm hơn hẳn* account đang dùng (so sau khi làm tròn).
- **Chống đổi qua lại**: cooldown 300 s giữa hai lần đổi chủ động (bỏ qua khi active đã chạm 5h/tuần);
  không quay về account vừa rời cho tới khi nó hồi (`RECOVERY_HYSTERESIS_S=300`, hoặc chỗ còn +3 điểm).
- **Khi mọi account đều quá ngưỡng**: bỏ EDF, chọn chỗ còn thô lớn nhất; chỉ đổi khi ứng viên hơn
  `min(2×chỗ còn active, chỗ còn active + 3)`. claude-swap đo được: xếp theo reset ở trạng thái này đưa
  cả 18 lần đổi vào account đã ≥95%.
- **Lỗi/429**: account báo hết quota thì loại tới `resets_at`; không có giờ reset thì nghỉ 30 s gấp đôi tới
  5 phút (better-ccflare). Mở lại chỉ khi mọi cửa sổ đã dưới ngưỡng.
- Hệ số k là phần riêng của ta: cả hai dự án đều không quy đổi 5h ↔ tuần. Phải kiểm bằng test có số đo.
