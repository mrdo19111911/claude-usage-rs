# License (spec, chưa code)

Hỏi 2026-11-12: "Tôi muốn thu tiền khi ai đó dùng app này". Cổng bán: Polar (docs/SO_SANH_BAN_HANG.md).
Chủ app chọn: dùng thử 30 ngày, tính theo email tài khoản Claude; hết thử thì trả phí (0,5 $, chưa
chốt chu kỳ) hoặc xem quảng cáo.

## Người dùng thấy gì
- Dùng thử 30 ngày, đủ mọi tính năng, tính từ lần đầu **một trong các email Claude** của họ đăng nhập vào SCAS.
- Một email đã từng dùng thử (trên bất kỳ máy nào) thì không được thử lại. Cài lại hay xoá dữ liệu cũng không reset được.
- Dashboard ghi "Trial: 23 days left".
- Hết dùng thử: hoặc trả phí qua Polar (nhập key ở Settings, nút "Activate"), hoặc chọn "xem quảng cáo" để dùng tiếp.
  Chưa làm cả hai thì vẫn xem được usage, vẫn chuyển tay được, chỉ **tắt tự chuyển** (có ghi lý do và link mua).
  Không bao giờ xoá dữ liệu.
- Mất mạng: trạng thái lần kiểm tra thành công gần nhất được giữ 7 ngày.

## Miễn phí (hỏi 2026-11-12)
- 1–2 tài khoản Claude khác nhau trong SCAS: miễn phí hoàn toàn, không cần dùng thử. Email trùng (khác hoa thường) chỉ tính một.
- Từ 3 tài khoản: áp dụng dùng thử 30 ngày, rồi key hoặc quảng cáo.
- Bất kỳ tài khoản nào có chữ "gosmartlog" trong email: miễn phí vĩnh viễn (`status_for`).

## Lưu email
- Không gửi email thật. Gửi `SHA-256(lowercase(email) + muối cố định của SCAS)`.
- Máy chủ trial nhỏ (ví dụ Cloudflare Worker + KV) lưu `hash -> ngày bắt đầu thử`. Polar không có sẵn chỗ lưu này.
- Hỏi: gửi các hash của những email đang đăng nhập; nhận lại ngày bắt đầu sớm nhất. Email mới thì ghi ngày hôm nay.
- Lần chạy đầu phải báo rõ cho người dùng, và có chính sách riêng tư.

## Module
- `src/license.rs` (thuần, có test): `fn status(s: &Saved, now: i64) -> Status`, với
  `Status = Trial{days_left} | Licensed | Ads | Expired{why}`. Thêm `fn allows_auto(&Status) -> bool`.
- `src/license_hash.rs` (thuần): băm email.
- `src/license_net.rs`: gọi máy chủ trial và API license key của Polar; lưu `license.json` trong thư mục dữ liệu.

## Test (viết trước)
- băm: không phân biệt hoa thường, ra cùng một hash; hai email khác nhau ra hai hash khác nhau;
- dùng thử: ngày đầu sớm nhất trong các email được dùng; ngày 0 còn 30, ngày 30 thành Expired;
- thêm một email mới vào máy không reset ngày dùng thử;
- đã kích hoạt key, lần kiểm tra cuối cách đây 3 ngày thì Licensed; cách đây 8 ngày thì Expired;
- Polar trả lời "revoked" thì Expired, why = "revoked";
- `allows_auto`: true với Trial, Licensed và Ads; false với Expired.

## Giá (chốt 2026-11-12)
- 1 $ / 1 tháng, 1,5 $ / 3 tháng, **5 $ / năm (gói nổi bật)**. Ba sản phẩm thuê bao trên Polar, cùng một quyền lợi license key.
- Phí Polar khoảng 5% + 0,5 $ mỗi giao dịch, nên bạn nhận khoảng 0,45 $, 0,925 $ và 4,25 $.
- Phí rút tiền tính theo lần rút, không theo từng đơn: 2 $ cho tháng có rút, cộng 0,25% + 0,25 $ mỗi lần, cộng đổi tiền tới 1%.

## Chưa quyết
- **Quảng cáo:** các mạng quảng cáo lớn hầu như không làm cho app desktop. Thực tế là bán chỗ banner cho nhà tài trợ, hoặc hiện quảng cáo của chính mình.
- Mỗi key dùng mấy máy; có dừng công khai nhánh `main` không.
