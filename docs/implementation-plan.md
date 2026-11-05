# Kế hoạch triển khai theo module — G1

> Ngày bắt đầu: 2026-11-05. Kiến trúc: `analyst.md` (v0.4). Yêu cầu: `requirements.md`.
> G1 gồm 6 module, làm **lần lượt**; xong và test đạt module trước mới sang module sau.

| Module | Nội dung | Trạng thái |
|---|---|---|
| Module | Nội dung | Trạng thái |
|---|---|---|
| **M0** | Khung dự án: Cargo workspace (core / api / worker), Docker Compose dev, cấu hình, lỗi thống nhất, middleware nền, health check, migrations; khung Vue 3 + H-UI | ✅ Xong (2026-11-05) |
| **M1** | Đăng ký, xác minh email bằng OTP, đăng nhập, phiên (access token + refresh token xoay vòng), đăng xuất, quên / đổi mật khẩu, giới hạn tần suất | ✅ Xong |
| **M2** | Phân quyền: vai trò → quyền, middleware kiểm tra quyền, `/api/auth/me` trả quyền, menu và route theo quyền ở frontend | ✅ Xong |
| **M3** | Admin quản lý user: danh sách (tìm, lọc, phân trang), tạo, sửa, đổi vai trò, khóa / mở khóa, gửi đặt lại mật khẩu, nhật ký thao tác | ✅ Xong |
| **M4** | Master data: môn học, lớp (Lớp 1–12 + lớp riêng), danh mục (nhãn trong môn), mức độ câu hỏi; giáo viên tự thêm, admin gộp / đưa lên danh sách chung | ✅ Xong |
| **M5** | Đăng ký làm giáo viên (chọn môn, lớp) và admin duyệt | ✅ Xong |

| **M6.1** | SSO phần 1: bảng liên kết, chính sách đăng nhập / tạo / liên kết tài khoản (chính sách đã chốt 2026-11-05), xác nhận bằng mật khẩu hoặc OTP, tự gỡ / admin gỡ liên kết | ✅ Xong (2026-11-05) |
| **M6.2** | SSO phần 2: luồng OIDC thật với Google / Microsoft, nút đăng nhập + trang liên kết ở giao diện, giới hạn số thiết bị | Chờ (làm tiếp) |

Sau M6: G2 (thư viện bài, lớp học + lộ trình) theo `analyst.md` mục 7.

### M6 — SSO

**M6.1 (đã xong)** — module `core/src/modules/sso`, migration `20261105000006_sso.sql` (`user_identities`, `sso_link_requests`).

| API | Mô tả |
|---|---|
| `POST /api/auth/sso/link/send-code` | Gửi mã OTP về email của tài khoản cần liên kết (cách nhau ≥ 60 giây) |
| `POST /api/auth/sso/link` | Xác nhận bằng `password` hoặc `code` → liên kết + đăng nhập (đặt cookie refresh) |
| `GET /api/auth/identities` · `DELETE /api/auth/identities/{provider}` | Xem / tự gỡ liên kết (thiết bị khác bị đăng xuất) |
| `GET /api/admin/users/{id}/identities` · `DELETE …/{provider}` | Admin xem / gỡ liên kết (mọi phiên của user bị thu hồi) |

Hàm `sso::service::complete_login(ctx, ProviderIdentity, client)` nhận danh tính **đã được nhà cung cấp xác thực** và trả về `LoggedIn` hoặc `LinkRequired` — phần 2 gọi hàm này sau khi đổi code lấy id_token.

| # | Ca test (`api/tests/sso.rs`, 10 test đạt) | Kết quả |
|---|---|---|
| 1 | Email chưa có tài khoản | Tạo tài khoản học sinh, không mật khẩu, username gợi ý từ email (trùng thì thêm số), đăng nhập |
| 2 | Nhà cung cấp chưa xác minh email / không có email | 400 `SSO_EMAIL_UNVERIFIED` |
| 3 | Trùng email tài khoản đã xác minh | `LinkRequired` (email che bớt); sai mật khẩu 401; đúng → liên kết + đăng nhập; token dùng lại → `SSO_LINK_EXPIRED` |
| 4 | Xác nhận bằng OTP | Gửi mã, chặn gửi lại < 60 giây, mã sai `INVALID_CODE`, mã đúng → đăng nhập |
| 5 | Sai 5 lần / hết hạn / không gửi gì | `TOO_MANY_ATTEMPTS` rồi `SSO_LINK_EXPIRED`; 422 khi thiếu cả mật khẩu và mã |
| 6 | Trùng email tài khoản **chưa xác minh** | Người đăng nhập SSO giành lại: mật khẩu và OTP cũ mất hiệu lực |
| 7 | Tài khoản bị khóa | `ACCOUNT_LOCKED` (cả khi đã liên kết) |
| 8 | Đã liên kết Google, thêm một Google khác cùng email | `SSO_PROVIDER_ALREADY_LINKED`; Microsoft vẫn liên kết được |
| 9 | Tự gỡ liên kết | Tài khoản không mật khẩu → `LAST_LOGIN_METHOD`; có mật khẩu → gỡ được, thiết bị khác bị đăng xuất |
| 10 | Admin xem / gỡ liên kết | Giáo viên 403; admin gỡ → mọi phiên bị thu hồi; nhật ký `identity.unlink` / `identity.link` |

**M6.2 (làm tiếp):**
1. Cấu hình `SSO_GOOGLE_CLIENT_ID/SECRET`, `SSO_MICROSOFT_CLIENT_ID/SECRET/TENANT`, `SSO_REDIRECT_BASE`. **Bạn cần tạo ứng dụng OAuth** ở Google Cloud Console và Microsoft Entra (Azure) để có client id / secret; dev dùng redirect `http://127.0.0.1:5173/api/auth/sso/{provider}/callback`.
2. `GET /api/auth/sso/{provider}/start`: tạo `state` + `nonce` + PKCE, lưu ở Valkey 10 phút, chuyển hướng tới nhà cung cấp.
3. `GET /api/auth/sso/{provider}/callback`: kiểm tra `state`, đổi code lấy token, kiểm tra id_token (chữ ký qua JWKS, `iss`, `aud`, `exp`, `nonce`) → `complete_login` → đặt cookie refresh rồi chuyển về giao diện (`/sso/xong`), hoặc về `/sso/lien-ket?token=…` khi cần xác nhận.
4. Giao diện: nút "Đăng nhập với Google / Microsoft", trang xác nhận liên kết (mật khẩu hoặc mã OTP), mục "Tài khoản liên kết" trong Hồ sơ, xem / gỡ liên kết trong trang admin.
5. Giới hạn thiết bị: tối đa N chuỗi phiên đang mở (mặc định 2); đăng nhập thiết bị mới thì phiên cũ nhất bị thu hồi.
6. Tài khoản tạo từ SSO có thể đặt mật khẩu lần đầu (hiện dùng được "Quên mật khẩu").

### Kết quả kiểm tra G1 (2026-11-05)

| Hạng mục | Kết quả |
|---|---|
| Backend unit test (`sls-core`) | 25 / 25 đạt |
| Backend integration test (`sls-api/tests`, PostgreSQL thật, mỗi test một database) | 40 / 40 đạt — health 3, auth 18, rbac 3, admin_users 7, catalog 6, teachers 3 |
| `cargo fmt --check`, `cargo clippy -D warnings` | Đạt |
| Frontend unit test (Vitest) | 21 / 21 đạt — API client 9, menu + guard 7, danh sách phân trang 3, Enter gửi form 2 |
| `npm run build` | Đạt |
| Chạy thật với Docker (api + worker + Mailpit) | Đăng ký → email OTP qua RabbitMQ → Mailpit → xác minh → đăng nhập; admin tạo user, duyệt hồ sơ giáo viên, nhật ký ghi đúng |
| Kiểm tra trên trình duyệt | Đăng nhập admin, menu theo quyền, danh sách / thêm user qua modal, mọi trang admin tải dữ liệu; học sinh: đăng ký → OTP → bị chặn khỏi trang admin (403) → gửi hồ sơ giáo viên |

### Còn lại / lưu ý

- **SMTP production** cần TLS: worker đang gửi SMTP không mã hóa (đủ cho Mailpit khi dev). Bật feature TLS của `lettre` khi có nhà cung cấp email thật.
- Khóa tài khoản: refresh token bị thu hồi ngay, access token đang cầm còn dùng được tối đa 15 phút (đánh đổi đã chọn ở mục 3).
- Gộp lớp / danh mục: hiện chỉ ẩn mục bị gộp. Khi G2 có bảng tham chiếu (lớp học, câu hỏi…), phần chuyển tham chiếu đặt trong `catalog::service::merge_*`.
- Lúc mở trang khi chưa đăng nhập, trình duyệt ghi một lỗi 401 của `/api/auth/refresh` trong console — đúng thiết kế (cookie HttpOnly nên JS không biết trước có phiên hay không).

### Ghi chú H-UI phát hiện khi tích hợp Vue [UPDATED 2026-11-05 — đã sửa trong H-UI, app đã gỡ cách xử lý tạm]

| Vấn đề | Trạng thái |
|---|---|
| `hui-modal` có cả property `open` và method `open()` → Vue `:open` không mở được modal | Đã sửa trong H-UI (commit cục bộ, chưa push); đang đổi tiếp sang quy ước `open` (boolean) + `show()` / `hide()` / `toggle()` cho mọi component. App dùng lại `:open="x"`, ~~`:open.attr="x \|\| null"`~~ |
| `hui-input` nhấn Enter không gửi `<form>` | Đã sửa trong H-UI. App đã bỏ ~~`src/utils/enterSubmit.js`~~ |
| Bind động viết kebab-case (`:confirm-loading="false"`) thành attribute chuỗi `"false"` → bị hiểu là `true` | Giữ nguyên trong H-UI (đúng chuẩn HTML), chỉ ghi vào README. App tuân theo quy ước đặt tên ở dưới |

### Quy ước đặt tên (frontend + H-UI)

| Loại | Kiểu viết | Ví dụ | Ghi chú |
|---|---|---|---|
| Attribute HTML (viết tĩnh trong thẻ) | kebab-case | `ok-text="Lưu"`, `show-total` | Attribute HTML không phân biệt hoa thường nên luôn viết thường, nối bằng `-` (như `aria-label`, `data-id`) |
| Property JS / bind động `:` trong Vue | camelCase | `:confirmLoading="x"`, `:pageSize="n"` | Vue chỉ nhận ra property khi tên khớp đúng property → luôn gán đúng kiểu (boolean, số, mảng) |
| Property chỉ trạng thái | danh từ / tính từ | `open`, `loading`, `disabled`, `value` | |
| Method | camelCase, **động từ** | `show()`, `hide()`, `toggle()`, `reset()` | Method là động từ, property là trạng thái → không bao giờ trùng tên |
| Sự kiện giống trình duyệt | Giữ tên gốc | `input` (đang gõ), `change` (đã chốt) | Nhờ vậy `v-model` và `<form>` hoạt động |
| Sự kiện riêng của H-UI | `hui-` + kebab-case, **việc đã xảy ra**, không có tiền tố `on` | `@hui-open`, `@hui-close`, `@hui-select` | Sự kiện không nằm trên phần tử như property → dùng chung chữ với property (`open` / `hui-open`) không xung đột, giống `<dialog>` có `close()` và sự kiện `close` |
| Trước / sau hiệu ứng | thêm `after-` khi xong | `hui-close` → `hui-after-close` | |
| Yêu cầu có thể chặn | sự kiện cancelable, chặn bằng `preventDefault()` | `hui-cancel`, `hui-ok` | |
| `detail` của sự kiện | luôn là object | `{ value }`, `{ open, source }` | Thêm trường sau này không phá code cũ |
| Hàm xử lý trong app | `on` + tên sự kiện | `onClose`, `onSaved`, `onAction` | `on` chỉ dành cho hàm xử lý |
| Biến CSS, CSS part | kebab-case | `--hui-primary` | |
| JSON giữa backend và frontend | camelCase | `fullName`, `createdAt` | `serde(rename_all = "camelCase")` ở DTO |
| Rust (biến, field), cột / bảng DB | snake_case | `full_name`, `password_hash` | Chỉ dùng phía backend |

Tóm lại trong template Vue: **giá trị cố định → kebab-case; có dấu `:` → camelCase.** Không cần `.prop` / `.attr`.

**Overlay của H-UI (modal, drawer, popover, dropdown, tooltip…) — chốt 2026-11-05:**

| Loại | Tên |
|---|---|
| Property / attribute | `open` (boolean) |
| Method | `show()`, `hide()`, `toggle()` — bỏ method `open()` |
| Sự kiện | `hui-open`, `hui-after-open`, `hui-close`, `hui-after-close` (không dùng `hui-show` / `hui-hide`) |
| Có thể chặn (khi cần) | `hui-cancel`, `hui-ok` |

Dùng trong Vue: `<hui-modal :open="x" @hui-close="x = false">`, hoặc bọc trong component nhỏ có `v-model:open`.

---

## 1. Quy trình cho mỗi module

1. Migration SQL (`backend/migrations/`).
2. Entity (`model.rs`) → DTO (`dto.rs`) → Repository (chỉ SQL) → Service (nghiệp vụ, transaction, kiểm tra quyền sở hữu) → Controller (`api/src/controllers/`).
3. Test backend: unit test cho hàm thuần; integration test gọi thẳng router với database riêng cho từng test.
4. Trang frontend dùng component H-UI có sẵn.
5. Chạy checklist thủ công trên trình duyệt.

**Xong một module khi:** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` đều đạt; `npm run build` và `npm test` ở frontend đạt; checklist thủ công đạt; tài liệu được cập nhật.

## 2. Môi trường dev

| Cần có | Ghi chú |
|---|---|
| Rust stable (rustup) + Visual Studio Build Tools (C++) | Build backend |
| Docker Desktop | Chạy PostgreSQL, Valkey, RabbitMQ, Mailpit |
| Node.js LTS | Build frontend |

```bash
docker compose -f deploy/docker-compose.dev.yml up -d   # hạ tầng
cd backend && cargo run -p api                          # API ở http://127.0.0.1:8080
cd backend && cargo run -p worker                       # gửi email (xem ở http://127.0.0.1:8025)
cd frontend && npm run dev                              # giao diện ở http://127.0.0.1:5173
cd backend && cargo test                                # test backend (cần Postgres đang chạy)
cd frontend && npm test                                 # test frontend
```

Cấu hình đọc từ biến môi trường; mẫu ở `backend/.env.example` (giá trị chỉ dùng cho dev).

## 3. Quyết định kỹ thuật của G1

| Chủ đề | Quyết định | Lý do |
|---|---|---|
| Truy vấn SQL | `sqlx::query_as::<_, Entity>()` (kiểm tra lúc chạy), integration test phủ mọi câu SQL | Không bắt buộc database phải chạy mỗi lần `cargo check`. Có thể chuyển sang macro `query_as!` (kiểm tra lúc compile) sau, bằng `cargo sqlx prepare` |
| Migration | Nhúng vào binary (`sqlx::migrate!`), tự chạy khi `api` khởi động | Không cần cài thêm sqlx-cli |
| ID | `uuid` v7 (PostgreSQL 18 có sẵn hàm `uuidv7()`) | Có thứ tự theo thời gian, index tốt hơn UUID ngẫu nhiên |
| Mật khẩu | Argon2id, chạy trong `spawn_blocking` | Không chặn luồng async |
| Access token | JWT HS256, 15 phút, gửi qua header `Authorization: Bearer`; frontend chỉ giữ trong bộ nhớ | Không lưu ở `localStorage` để tránh bị đánh cắp qua XSS |
| Refresh token | Chuỗi ngẫu nhiên 32 byte, DB chỉ lưu hash SHA-256; 30 ngày; cookie `HttpOnly`, `SameSite=Strict`, `Path=/api/auth`; **xoay vòng** mỗi lần dùng; dùng lại token cũ → thu hồi cả chuỗi phiên | Phát hiện token bị đánh cắp |
| Khóa tài khoản | Thu hồi mọi refresh token ngay; access token còn hạn tối đa 15 phút | Không phải tra DB ở mỗi request |
| Phân quyền | Danh sách quyền và bảng vai trò → quyền **viết trong code** (`core::auth::permissions`); quyền theo từng lớp học kiểm tra trong service | 3 vai trò cố định; khi nào cần admin tự cấu hình vai trò thì chuyển bảng này vào DB |
| Email | `api` đẩy việc vào RabbitMQ → `worker` gửi qua SMTP (Mailpit khi dev). Trong test, hàng đợi là bộ nhớ để đọc lại nội dung email | Đúng kiến trúc; test không cần RabbitMQ |
| Giới hạn tần suất | Bộ đếm trong Valkey; trong test dùng bộ đếm trong bộ nhớ | Test độc lập với nhau |
| Lỗi API | `{ "error": { "code": "VALIDATION_FAILED", "message": "…", "fields": { "email": ["…"] } } }` + header `x-request-id` | Frontend xử lý thống nhất |
| Admin đầu tiên | Lúc khởi động, nếu chưa có admin và có `BOOTSTRAP_ADMIN_*` thì tạo | Không phải sửa DB bằng tay |
| H-UI | Dùng bản `@itmrhoang/h-ui` 0.5.1 từ npm. Khi cần component mới (từ G2) thì chuyển sang link tới repo H-UI ở máy | G1 chỉ dùng component có sẵn |

---

## 4. Chi tiết từng module

### M0 — Khung dự án

**Backend:** workspace `core` / `api` / `worker`; `Config` từ biến môi trường; `AppError` → JSON lỗi; middleware theo thứ tự request_id → trace → CORS → timeout → giới hạn body; `GET /api/health` (kiểm tra Postgres + Valkey); migration đầu tiên bật extension (`pg_trgm`, `unaccent`).
**Hạ tầng:** `deploy/docker-compose.dev.yml` — PostgreSQL 18, Valkey 9.1, RabbitMQ 4.x, Mailpit; chỉ mở cổng trên `127.0.0.1`; ghim tag cụ thể.
**Frontend:** Vite + Vue 3 + Vue Router + Pinia + H-UI; layout đăng nhập và layout ứng dụng; API client; trang trạng thái hệ thống.

| Test | Loại |
|---|---|
| `Config` thiếu biến bắt buộc → báo lỗi rõ ràng | Unit |
| `AppError` → đúng mã HTTP và JSON | Unit |
| `GET /api/health` → 200, có `x-request-id` | Integration |
| `docker compose up` → 4 service healthy; `npm run build` đạt | Thủ công |

### M1 — Đăng ký & đăng nhập

**Bảng:** `users`, `email_verifications`, `refresh_tokens`, `password_resets`.

| API | Mô tả |
|---|---|
| `POST /api/auth/register` | username, email, họ tên, mật khẩu → tài khoản `pending_verification`, gửi OTP 6 số qua email |
| `POST /api/auth/verify-email` | email + OTP → kích hoạt và đăng nhập luôn (trả token) |
| `POST /api/auth/resend-verification` | Gửi lại OTP (cách nhau ≥ 60 giây); luôn trả 202 để không lộ email nào đã đăng ký |
| `POST /api/auth/login` | username **hoặc** email + mật khẩu → access token + cookie refresh |
| `POST /api/auth/refresh` | Đổi refresh token lấy access token mới, xoay vòng refresh token |
| `POST /api/auth/logout` · `logout-all` | Thu hồi phiên hiện tại / mọi phiên |
| `POST /api/auth/forgot-password` · `reset-password` | Gửi link đặt lại (hạn 30 phút, dùng một lần); đặt lại xong thu hồi mọi phiên |
| `POST /api/auth/change-password` | Đổi mật khẩu, thu hồi các phiên khác |
| `GET /api/auth/me` | Thông tin người dùng hiện tại |

**Frontend:** trang Đăng nhập, Đăng ký, Nhập OTP (`hui-otp-input`), Quên mật khẩu, Đặt lại mật khẩu, Hồ sơ (đổi mật khẩu); tự gọi `refresh` khi access token hết hạn.

| # | Ca test | Kết quả mong đợi |
|---|---|---|
| 1 | Đăng ký hợp lệ | 201; user `pending_verification`; có email chứa OTP |
| 2 | Trùng username / email (khác hoa thường) | 409 `USERNAME_TAKEN` / `EMAIL_TAKEN` |
| 3 | Dữ liệu sai (email sai dạng, mật khẩu < 8 ký tự) | 422 kèm lỗi từng trường |
| 4 | OTP sai; sai 5 lần; hết hạn | 400 `INVALID_CODE`; mã bị hủy; 400 `CODE_EXPIRED` |
| 5 | OTP đúng | 200, có token; user `active` |
| 6 | Đăng nhập khi chưa xác minh | 403 `EMAIL_NOT_VERIFIED` |
| 7 | Sai mật khẩu / user không tồn tại | Cùng 401 `INVALID_CREDENTIALS` (không lộ user nào tồn tại) |
| 8 | Đăng nhập bằng username và bằng email | 200 |
| 9 | Tài khoản bị khóa | 403 `ACCOUNT_LOCKED` |
| 10 | Đăng nhập sai quá 10 lần / 15 phút | 429 `RATE_LIMITED` |
| 11 | Refresh hợp lệ | Token mới, cookie mới; token cũ hết dùng được |
| 12 | Dùng lại refresh token cũ | 401; cả chuỗi phiên bị thu hồi |
| 13 | Đăng xuất rồi refresh | 401 |
| 14 | Quên mật khẩu với email không tồn tại | Vẫn 202 |
| 15 | Đặt lại mật khẩu; dùng lại link | 204, phiên cũ bị thu hồi; lần 2 → 400 |
| 16 | `/me` không có token / token hết hạn / token sai chữ ký | 401 |
| U1 | Băm và kiểm tra mật khẩu; JWT tạo / đọc / hết hạn; OTP đủ 6 số | Unit |

### M2 — Phân quyền

| Vai trò | Quyền (ban đầu) |
|---|---|
| `admin` | Tất cả |
| `teacher` | `master_data.propose` (tự thêm lớp riêng / danh mục); các quyền dạy học thêm ở G2 |
| `student` | Các quyền học tập thêm ở G2 |

Quyền quản trị: `admin.access`, `users.read`, `users.manage`, `audit.read`, `master_data.manage`, `teacher_applications.review`.
**Backend:** extractor `CurrentUser` (đọc JWT); `require_permission(...)` gắn vào từng nhóm route; `/api/auth/me` trả thêm danh sách quyền.
**Frontend:** `meta.permission` trên route + guard; menu ẩn mục không có quyền; trang 403.

| # | Ca test | Kết quả |
|---|---|---|
| 1 | Bảng vai trò → quyền: admin có đủ; teacher / student không có `admin.access` | Unit |
| 2 | Gọi `/api/admin/*` không có token | 401 |
| 3 | Student / teacher gọi `/api/admin/*` | 403 `FORBIDDEN` |
| 4 | Admin gọi `/api/admin/*` | 200 |
| 5 | `/me` trả đúng quyền theo vai trò | Integration |
| 6 | Vào thẳng URL trang admin khi là student | Chuyển sang trang 403 (thủ công) |

### M3 — Admin quản lý user

**Bảng:** `audit_logs`.

| API | Mô tả |
|---|---|
| `GET /api/admin/users` | Tìm theo username / email / họ tên; lọc vai trò, trạng thái; phân trang |
| `GET /api/admin/users/{id}` | Chi tiết |
| `POST /api/admin/users` | Admin tạo tài khoản (đã kích hoạt) |
| `PATCH /api/admin/users/{id}` | Sửa họ tên, email, vai trò |
| `POST /api/admin/users/{id}/lock` · `unlock` | Khóa (thu hồi mọi phiên) / mở khóa |
| `POST /api/admin/users/{id}/send-password-reset` | Gửi email đặt lại mật khẩu |
| `GET /api/admin/audit-logs` | Nhật ký thao tác, phân trang |

**Frontend:** trang Người dùng (`hui-table` phân trang phía server, ô tìm kiếm, bộ lọc), form tạo / sửa trong `hui-modal`, nút khóa / mở khóa có xác nhận; trang Nhật ký.

| # | Ca test | Kết quả |
|---|---|---|
| 1 | Danh sách: phân trang, lọc vai trò, tìm theo tên | Đúng tổng số và trang |
| 2 | Tạo user trùng username / email | 409 |
| 3 | Đổi vai trò user khác | 200, có dòng nhật ký |
| 4 | Admin tự hạ vai trò / tự khóa mình | 409 `CANNOT_MODIFY_SELF` |
| 5 | Khóa user → user đó refresh thất bại, đăng nhập 403 | Integration |
| 6 | Mở khóa → đăng nhập lại được | Integration |
| 7 | Gửi đặt lại mật khẩu → có email | Integration |
| 8 | Mọi thao tác ghi nhật ký (người làm, hành động, đối tượng) | Integration |

### M4 — Master data

**Bảng:** `subjects`, `grades` (seed Lớp 1–12, `is_system`), `tags` (thuộc `subject_id`). Mức độ câu hỏi là enum trong code: nhận biết, thông hiểu, vận dụng, vận dụng cao.

| API | Mô tả |
|---|---|
| `GET /api/master-data` | Môn học, lớp, mức độ (lớp chung + lớp riêng của chính mình) |
| `GET /api/subjects/{id}/tags` | Danh mục của một môn (chung + của mình) |
| `POST /api/grades` · `POST /api/subjects/{id}/tags` | Giáo viên tự thêm (chỉ mình thấy); admin thêm thì vào danh sách chung |
| `POST/PATCH /api/admin/subjects…` | Admin thêm / sửa / ẩn môn học, sắp thứ tự |
| `PATCH /api/admin/grades/{id}` · `PATCH /api/admin/tags/{id}` | Sửa tên, thứ tự, ẩn |
| `POST /api/admin/{grades\|tags}/{id}/promote` | Đưa mục của giáo viên lên danh sách chung |
| `POST /api/admin/{grades\|tags}/{id}/merge` | Gộp vào mục khác (`intoId`) |
| `GET /api/admin/master-data/pending` | Các mục giáo viên mới thêm, chờ admin xử lý |

**Frontend:** trang admin Môn học, Lớp, Danh mục (chọn môn → danh sách nhãn), Mục chờ xử lý (gộp / đưa lên chung).

| # | Ca test | Kết quả |
|---|---|---|
| 1 | Seed có Lớp 1 → Lớp 12 đúng thứ tự | Integration |
| 2 | Sửa / ẩn lớp hệ thống | 409 `SYSTEM_ITEM` |
| 3 | Admin tạo môn trùng tên | 409 |
| 4 | Giáo viên tạo môn | 403 |
| 5 | Giáo viên A thêm lớp riêng → A thấy, giáo viên B không thấy, admin thấy trong danh sách chờ | Integration |
| 6 | Admin đưa lên chung → mọi người thấy | Integration |
| 7 | Admin gộp A vào B → A không còn hiện | Integration |
| 8 | Danh mục chỉ thuộc đúng một môn; trùng tên trong cùng môn → 409, khác môn → được | Integration |

### M5 — Đăng ký làm giáo viên & duyệt

**Bảng:** `teacher_applications`, `teacher_subjects`.

| API | Mô tả |
|---|---|
| `POST /api/teacher-applications` | User đã xác minh gửi hồ sơ: môn, lớp, giới thiệu, kinh nghiệm |
| `GET /api/teacher-applications/me` | Xem trạng thái hồ sơ của mình |
| `GET /api/admin/teacher-applications` | Danh sách hồ sơ theo trạng thái |
| `POST /api/admin/teacher-applications/{id}/approve` · `reject` | Duyệt → vai trò `teacher` + môn được dạy; từ chối kèm lý do; email báo kết quả |

| # | Ca test | Kết quả |
|---|---|---|
| 1 | Gửi hồ sơ hợp lệ | 201 `pending` |
| 2 | Gửi khi đã có hồ sơ đang chờ / đã là giáo viên | 409 |
| 3 | Admin duyệt | Vai trò `teacher`, có `teacher_subjects`, có email; lần refresh token sau mang vai trò mới |
| 4 | Admin từ chối | `rejected` + lý do, có email; được gửi lại hồ sơ mới |
| 5 | Student gọi API duyệt | 403 |

---

## 5. Test frontend

| Loại | Công cụ | Phạm vi trong G1 |
|---|---|---|
| Unit | Vitest | API client (tự refresh khi 401, chỉ refresh một lần khi nhiều request cùng lỗi), store đăng nhập, hàm kiểm tra quyền |
| Thủ công | Trình duyệt | Checklist cuối mỗi module (ở trên) |
| E2E | Playwright | Thêm từ G2, khi luồng học tập đã có |
