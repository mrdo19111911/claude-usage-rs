# So sánh: đồng bộ thẻ session giữa N thư mục bằng trạng thái lần sync trước

Vấn đề: hiện thẻ có `max(lastActivityAt, lastFocusedAt)` lớn nhất thì thắng, bằng nhau thì để yên.
Lưu trữ (`isArchived`) không đổi hai mốc đó nên không lan sang account khác (đo được: một account
116 session, các account khác 26). Cần biết bản nào "đã đổi từ lần sync trước", không chỉ bản nào "mới hơn".

Mã đã đọc (clone 2026-11-12): Unison `8772d26`, rclone `f42d829`, Syncthing `ed2e0fd`, claude-session-sync `962fd40`.

| Dự án | Phương pháp | Mức phù hợp | Chi phí |
|---|---|---|---|
| [Unison](https://github.com/bcpierce00/unison) | Archive mỗi bên lưu fingerprint + props lúc sync xong. `update.ml` `checkContentsChange`: khác archive thì là `Updates`, không thì `NoUpdates`. `recon.ml` `reconcile`: một bên `NoUpdates` thì bên kia thắng; hai bên đổi mà cùng digest thì coi như bằng; khác thì `Conflict "contents changed on both sides"` (chỉ tự chọn khi bật `-prefer`/`-force newer`). Lần đầu (`NoArchive`): mọi file là mới, nội dung khác thì xung đột. Archive chỉ cập nhật cho path đã chép xong (`files.ml` gọi `updateArchive`/`replaceArchiveLocal` sau khi chép; comment trong `checkContentsChange` nói rõ: chép hỏng thì không được ghi archive, không thì lần sau sót thay đổi). | Cao: đúng mô hình "so với bản lần trước", nhưng chỉ cho 2 bên | Archive nhị phân mỗi bên, có cả UI để giải xung đột |
| [rclone bisync](https://github.com/rclone/rclone/tree/master/cmd/bisync) | Listing lần trước `.path1.lst`/`.path2.lst` (size, modtime, hash). `deltas.go` `findDeltas`: so listing cũ với listing hiện tại *của cùng bên* để ra `deltaNew/Newer/Older/Hash/Deleted`. `applyDeltas`: đổi ở cả hai bên mà giống nhau thì để yên (`sameCompared`), khác thì `resolve.go` `resolve` + `conflictWinner` (`--conflict-resolve newer`: modtime lớn hơn; thiếu modtime thì không có người thắng), bên thua đổi tên `.conflictN` chứ không mất. Lần đầu bắt buộc `--resync` với `--resync-mode` (path1/newer/...). Lỗi giữa chừng: `listing.go` `modifyListing` chỉ ghi vào listing các file chép xong, `prepareRollback`/`revertToOldListings` giữ listing cũ; lỗi nặng thì đổi thành `.lst-err` và đòi `--resync`. | Cao: gần nhất với kế hoạch (hash theo bên, so với listing cũ) | Hai file listing, chế độ resync riêng |
| [Syncthing](https://github.com/syncthing/syncthing) | Version vector mỗi file (`protocol/vector.go` `Update`, `Compare`, `Concurrent`). `bep_fileinfo.go` `InConflictWith`: vector mới không `GreaterEqual` vector cũ thì là xung đột. `WinsConflict`: modtime mới hơn thắng, bằng thì so ID thiết bị trong vector. Bên thua đổi thành `name.sync-conflict-<ngày>-<giờ>-<thiết bị>` (`model/folder_sendrecv.go` `moveForConflict`). Cùng block (`BlocksEqual`) thì chỉ cập nhật metadata, không xung đột. | Trung bình: giải được N bên đổi đồng thời, nhưng ta chỉ có 1 bên đổi giữa hai lần sync | Phải lưu vector/thiết bị cho mỗi thẻ; quá nặng cho 6 thư mục chạy tuần tự |
| [bdarbaz/claude-session-sync](https://github.com/bdarbaz/claude-session-sync) | `claude_session_sync.py` `Syncer.run`: `rank = (freshness(rec), f.account == active, f.key)`; cập nhật khi `freshness(best) >= freshness(cur)`. Không lưu trạng thái nội dung (`seen` chỉ để phát hiện xoá). | Thấp về phát hiện thay đổi: **cùng lỗi**. Test `test_updates_flow_into_signed_out_account` tăng `lastActivityAt` 100→300 cùng lúc với `isArchived`, nên không bắt được. Nó đỡ được nhờ tình cờ: hoà thì thư mục của account *đang đăng nhập* thắng, chạy 5 giây một lần. Ta chép quy tắc nhưng bỏ ưu tiên đó ("bằng nhau thì để yên") và chỉ chạy lúc chuyển. | Không thêm gì |

## Chọn:

- **rclone bisync**: hash theo từng bên so với chính nó ở lần sync trước (`findDeltas`), không so chéo giữa các bên.
  Ghi trạng thái theo từng thẻ, chỉ cho những gì thực sự đã nằm trên đĩa (`modifyListing`).
- **Unison**: chỉ một bên đổi thì bên đó thắng, không cần nhìn thời gian (`reconcile` gặp `NoUpdates`);
  nhiều bên đổi mà nội dung bằng nhau thì không có gì để làm.
- **Syncthing**: khi phải chọn giữa các bản đã đổi thì dùng thời gian, hoà thì dùng khoá cố định (`WinsConflict`).
  Ta dùng (freshness, mtime, khoá thư mục).
- **claude-session-sync**: giữ nguyên các quy tắc chép (khoá account, template, trash). Ưu tiên "account đang
  đăng nhập" của nó tương ứng với "account vừa đóng" của ta. Hash tổng quát hơn: vẫn đúng khi người dùng tự
  đăng nhập Desktop ngoài app, hoặc một lần sync bị lỡ.

Rủi ro trong kế hoạch mà các dự án trên xử lý tốt hơn:

1. **Ghi hỏng.** Nếu ghi bản thắng sang B hỏng mà vẫn lưu hash mới của A, lần sau cả A và B đều "không đổi".
   Khi đó quay về so freshness và thay đổi `isArchived` lại mất. Unison và rclone chỉ đẩy trạng thái lên cho
   phần đã chép xong. Làm theo: thẻ nào còn bản ghi hỏng thì *không* cập nhật hash của bên thắng
   (giữ hash cũ để lần sau nó vẫn là "đã đổi"). B giữ hash cũ của B.
2. **Lần đầu / thiếu hash.** Kế hoạch tự chọn theo (freshness, mtime). Unison báo xung đột, rclone bắt chọn
   `--resync-mode`, Syncthing giữ bản thua thành file conflict. mtime có thể là thời điểm *app ta* ghi, không
   phải lúc người dùng sửa. Làm theo: lần chuyển sang trạng thái có hash, chép bản thua vào trash (như
   `.sync-conflict`), ghi số thẻ được chọn theo cách này vào log. Phải định nghĩa rõ: thẻ mới, chưa có hash
   trong thư mục của nó, thì tính là "đã đổi".
3. **Hai bên cùng đổi.** Làm ở mức thẻ thì A lưu trữ (freshness giữ nguyên) và B có tin nhắn mới (freshness
   tăng) cho ra B thắng, việc lưu trữ ở A mất mà không báo. Hiếm, vì giữa hai lần sync chỉ một account chạy
   Desktop. rclone và Syncthing giữ bản thua; gộp từng trường (như git merge-base) thì phải lưu cả nội dung
   gốc, không chỉ hash. Làm theo: bản thua trong nhóm "đã đổi" vào trash, không mất hẳn.
4. **Hash ổn định.** Phải sắp khoá *đệ quy* (cả object lồng nhau), không chỉ khoá cấp đầu. Desktop đổi
   phiên bản mà thêm khoá mới vào mọi thẻ thì cả thư mục thành "đã đổi"; khi đó vẫn đúng, chỉ là thắng theo freshness.
