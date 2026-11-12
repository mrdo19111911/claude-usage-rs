# So sánh: thu tiền khi bán SCAS (tra 2026-11-12)

Mục tiêu: bán license theo tháng/năm, người bán ở Việt Nam, khách ở nhiều nước, không phải tự lo VAT.

| Cổng | Vai trò | Phí bán | Chi tiền về VN | License key có sẵn | Ghi chú |
|---|---|---|---|---|---|
| **Polar** | Merchant of record (đứng ra bán, lo thuế) | khoảng 5% + 50¢ theo các review 2026 (kiểm lại trên polar.sh) | **Có**: trang supported-countries ghi "🇻🇳 Vietnam". Chi qua Stripe Connect Express; phí chi $2 mỗi tháng có rút tiền, 0,25% + $0,25 mỗi lần, đổi tiền tới 1% | Có (gói quyền lợi license key: kiểm tra, kích hoạt theo máy) | Hợp người bán lẻ, có API |
| **Lemon Squeezy** | Merchant of record (thuộc Stripe từ 7/2024) | 5% + 50¢, cộng 1,5% thẻ quốc tế, 0,5% thuê bao | Chưa xác nhận VN; rút về ngân hàng ngoài Mỹ mất 1%, qua PayPal 3% (tối đa $30); tối thiểu $50 | Có | Stripe đang chuyển sang sản phẩm mới, tương lai chưa rõ |
| **Paddle** | Merchant of record | 5% + 50¢ | Có theo WannaGuide; trả hằng tháng qua chuyển khoản hoặc Payoneer, có thể mất $15 phí SWIFT, tối thiểu $100 | **Không** (phải tự làm hoặc dùng thêm Keygen) | Có báo cáo tài khoản bị khoá và giữ tiền đột ngột |
| **Keygen** | Chỉ quản lý license, không thu tiền | Gói miễn phí cho dev; gói trả phí khoảng $19–79/tháng (chưa xác nhận) | không áp dụng | Mạnh nhất: file license có chữ ký, dùng offline, khoá theo máy | Ghép với Paddle hoặc Stripe khi cần kiểm soát chặt |

## Chọn

**Polar**: có chi tiền về Việt Nam (đã xác nhận trên trang chính thức), lo VAT, có sẵn license key và kích hoạt theo máy, phí chi tiền thấp. Cần chặt hơn thì sau này thêm Keygen.

Trước khi mở bán: mở tài khoản Polar, tạo một sản phẩm thử, rút thử một khoản nhỏ về tài khoản ngân hàng VN để đo phí thật.

## Nguồn
- https://polar.sh/docs/merchant-of-record/supported-countries
- https://polar.sh/docs/merchant-of-record/fees
- https://polar.sh/docs/features/finance/payouts
- https://agentaos.ai/answers/lemon-squeezy-fees
- https://www.lemonsqueezy.com/blog/payout-fees-sliced
- https://www.paddle.com/help/manage/get-paid/when-and-how-do-i-get-paid
- https://wanna.guide/payments/paddle
- https://keygen.sh/docs/api/cryptography
