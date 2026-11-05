# Smart-Learning-Space
A smart technology-integrated learning environment connecting teachers and students. Một môi trường/không gian học tập tích hợp công nghệ thông minh, giúp kết nối giáo viên và học sinh

Nền tảng học và dạy trực tuyến cho học sinh lớp 1–12 và giáo viên.

| Tài liệu | Nội dung |
|---|---|
| [requirements.md](requirements.md) | Yêu cầu nghiệp vụ (đánh dấu Chốt / Đề xuất / Sau) |
| [analyst.md](analyst.md) | Phân tích kiến trúc, tech stack, module, lộ trình triển khai |
| [docs/implementation-plan.md](docs/implementation-plan.md) | Kế hoạch từng module, ca test, kết quả kiểm tra |

## Công nghệ

Tổng quan: trình duyệt chạy **Vue 3** (giao diện **H-UI**) → gọi API của backend **Rust** → dữ liệu ở **PostgreSQL**. Việc chậm (gửi email…) được đẩy qua **RabbitMQ** cho `worker` làm nền. **Valkey** đếm số lần thao tác để chặn spam.

Backend là một *modular monolith* gồm 3 crate:

| Crate | Tác dụng |
|---|---|
| `crates/core` | Phần dùng chung: cấu hình, lỗi, kết nối DB, các module nghiệp vụ (`model` / `dto` / `repository` / `service`) |
| `crates/api` | Chương trình API: route HTTP (`controllers/`), middleware, đọc request |
| `crates/worker` | Chương trình chạy nền: nhận việc từ RabbitMQ và gửi email qua SMTP |

### Backend (Rust)

| Công nghệ | Tác dụng trong dự án |
|---|---|
| **Rust** (stable, edition 2024) | Ngôn ngữ backend: chạy nhanh, tốn ít RAM, trình biên dịch bắt lỗi bộ nhớ và lỗi kiểu dữ liệu từ sớm |
| **Tokio** | Runtime bất đồng bộ: một tiến trình phục vụ được nhiều request cùng lúc mà không cần mỗi request một luồng |
| **Axum** 0.8 | Web framework: khai báo route, đọc JSON / query / cookie, trả response |
| **tower** / **tower-http** | Middleware dùng chung: gắn `x-request-id`, ghi log request, CORS, giới hạn thời gian xử lý (30 giây) và kích thước body (1 MB) |
| **axum-extra** (cookie) | Đọc / ghi cookie `HttpOnly` (refresh token, `state` của SSO) |
| **SQLx** 0.9 | Truy vấn PostgreSQL bất đồng bộ, map kết quả vào struct; chạy migration (`backend/migrations/`) tự động khi API khởi động |
| **serde** / **serde_json** | Chuyển struct Rust ↔ JSON (đặt tên camelCase cho JSON) |
| **validator** | Kiểm tra dữ liệu gửi lên (độ dài, email, mật khẩu đủ mạnh…) và trả lỗi theo từng trường |
| **argon2** | Băm mật khẩu bằng Argon2id — thuật toán được khuyến nghị hiện nay, chống dò mật khẩu bằng GPU |
| **jsonwebtoken** | Tạo / kiểm tra access token (JWT HS256, 15 phút); kiểm tra chữ ký `id_token` của Google / Microsoft (RS256) |
| **reqwest** (rustls) | HTTP client gọi Google / Microsoft khi đăng nhập SSO: đọc cấu hình OIDC, lấy khóa công khai (JWKS), đổi `code` lấy token |
| **sha2**, **getrandom**, **base64**, **hex** | Sinh token / mã OTP ngẫu nhiên an toàn, lưu token dạng hash SHA-256 (DB bị lộ cũng không dùng được token), PKCE cho SSO |
| **uuid** (v7) | Khóa chính dạng UUID v7: có thứ tự theo thời gian nên index DB hiệu quả hơn UUID ngẫu nhiên |
| **chrono**, **time** | Ngày giờ (hạn token, hạn OTP), thời hạn cookie |
| **redis** (client) | Kết nối Valkey để đếm số lần thử (giới hạn tần suất) |
| **lapin** | Kết nối RabbitMQ: API đẩy việc vào hàng đợi, worker nhận việc ra làm |
| **lettre** | Worker gửi email qua SMTP (OTP, link đặt lại mật khẩu, kết quả duyệt giáo viên) |
| **deunicode** | Bỏ dấu tiếng Việt khi tạo slug / gợi ý username (`Trần Thị` → `tran.thi`) |
| **thiserror** / **anyhow** | Khai báo lỗi có kiểu trong thư viện / gom lỗi gọn khi khởi động chương trình |
| **tracing** / **tracing-subscriber** | Ghi log có cấu trúc (kèm `request_id`), đổi sang JSON khi chạy production (`LOG_FORMAT=json`) |
| **dotenvy** | Nạp cấu hình từ file `backend/.env` khi dev |

### Frontend

| Công nghệ | Tác dụng trong dự án |
|---|---|
| **Vue 3** (JavaScript) | Framework giao diện: component, dữ liệu phản ứng (đổi dữ liệu → giao diện tự cập nhật) |
| **Vite** | Chạy dev server có tải lại nóng; build ra file tĩnh để Caddy phục vụ (production không cần Node). Khi dev, Vite chuyển tiếp `/api` sang backend nên frontend và API dùng chung một địa chỉ |
| **Vue Router** | Điều hướng trang (`/dang-nhap`, `/ho-so`, `/admin/...`) và chặn trang theo quyền |
| **Pinia** | Lưu trạng thái dùng chung, ví dụ người dùng đang đăng nhập và danh sách quyền |
| **[H-UI](../HUI)** (`@itmrhoang/h-ui`) | Thư viện web component của dự án (nút, ô nhập, bảng, modal…); link bản build ở máy (`file:../../HUI`) |
| **Vitest** + **jsdom** | Unit test cho API client, kiểm tra quyền, tiện ích SSO… |

### Hạ tầng (Docker, file `deploy/docker-compose.dev.yml`)

| Dịch vụ | Phiên bản | Tác dụng |
|---|---|---|
| **PostgreSQL** | 18.6 | Cơ sở dữ liệu chính: người dùng, phiên đăng nhập, môn học, lớp… Có sẵn `uuidv7()`; extension `pg_trgm` + `unaccent` để tìm kiếm tiếng Việt không dấu |
| **Valkey** | 9.1.3 | Bộ nhớ đệm key-value (bản mã nguồn mở của Redis): đếm số lần đăng nhập sai, gửi OTP… để giới hạn tần suất |
| **RabbitMQ** | 4.4.0 | Hàng đợi việc: API trả lời ngay, việc chậm (gửi email, sau này chuyển mã video, đọc file đề) để worker làm sau. Trang quản trị: http://127.0.0.1:15672 |
| **Mailpit** | 1.33.0 | Hộp thư giả khi dev: bắt mọi email worker gửi để xem OTP / link tại http://127.0.0.1:8025, không gửi ra ngoài |

### Bảo mật đăng nhập

| Cơ chế | Tác dụng |
|---|---|
| Access token (JWT, 15 phút) chỉ giữ trong bộ nhớ trình duyệt | Không lưu `localStorage` → script lạ (XSS) khó lấy cắp |
| Refresh token trong cookie `HttpOnly`, xoay vòng mỗi lần dùng | JS không đọc được; dùng lại token cũ → cả chuỗi phiên bị thu hồi (phát hiện bị đánh cắp) |
| OTP 6 số qua email | Xác minh email khi đăng ký, xác nhận liên kết SSO |
| OIDC (Google / Microsoft) + PKCE + `state` + `nonce` | Đăng nhập bằng tài khoản ngoài an toàn: chống giả mạo callback, chống dùng lại mã |
| Giới hạn 2 thiết bị | Chống chia sẻ tài khoản |

### Sẽ dùng ở các giai đoạn sau (xem [analyst.md](analyst.md))

| Công nghệ | Tác dụng |
|---|---|
| **Caddy** | Reverse proxy production: tự cấp HTTPS, phục vụ file tĩnh của frontend, chuyển `/api` cho backend |
| **Garage** | Lưu file (video, ảnh, tài liệu) tương thích S3, tự host; trình duyệt upload thẳng bằng presigned URL |
| **LiveKit** | Dạy trực tiếp: video, âm thanh, chia sẻ màn hình (WebRTC) |

## Chạy môi trường dev

Cần cài: Rust stable, Visual Studio Build Tools (C++), Docker Desktop, Node.js LTS.

```bash
docker compose -f deploy/docker-compose.dev.yml up -d
```

```bash
cp backend/.env.example backend/.env
```

```bash
cd backend && cargo run -p sls-api
```

```bash
cd backend && cargo run -p sls-worker
```

```bash
cd frontend && npm install && npm run dev
```

| Địa chỉ | Dùng để |
|---|---|
| http://127.0.0.1:5173 | Giao diện |
| http://127.0.0.1:8080/api/health | Kiểm tra API |
| http://127.0.0.1:8025 | Mailpit — xem email (OTP, đặt lại mật khẩu) |
| http://127.0.0.1:15672 | Trang quản trị RabbitMQ |

Lần chạy đầu, API tạo tài khoản admin từ `BOOTSTRAP_ADMIN_*` trong `backend/.env` (giá trị mẫu chỉ dùng cho dev).

Mở giao diện bằng **http://127.0.0.1:5173** (không dùng `localhost`): cookie của trình duyệt tách riêng theo tên máy, nên đăng nhập Google / Microsoft chỉ hoạt động khi địa chỉ khớp `APP_BASE_URL`.

### Đăng nhập Google / Microsoft

Nút đăng nhập chỉ hiện khi có `SSO_GOOGLE_*` / `SSO_MICROSOFT_*` trong `backend/.env`.

| Nhà cung cấp | Tạo ứng dụng OAuth ở | Redirect URI (dev) |
|---|---|---|
| Google | Google Cloud Console → APIs & Services → Credentials → OAuth client ID (Web application) | `http://127.0.0.1:5173/api/auth/sso/google/callback` |
| Microsoft | Microsoft Entra admin center → App registrations → New registration (Web) | `http://127.0.0.1:5173/api/auth/sso/microsoft/callback` |

Microsoft: nên bật optional claim `xms_edov` cho id token (Token configuration). Với tài khoản trường / công ty, hệ thống chỉ tin email khi có claim này (tài khoản Microsoft cá nhân luôn được tin).

## Test

```bash
cd backend && cargo test --workspace
```

```bash
cd frontend && npm test
```

Test backend cần PostgreSQL đang chạy (Docker ở trên); mỗi test tự tạo một database riêng. `DATABASE_URL` cho dev đặt sẵn trong `backend/.cargo/config.toml`.

## Cấu trúc

```
backend/
  migrations/                 # SQL migration (tự chạy khi api khởi động)
  crates/core/src/
    config.rs, error.rs, db.rs, jobs.rs, rate_limit.rs, …
    security/                 # mật khẩu, JWT, token, phân quyền, Actor (người dùng hiện tại)
    modules/<module>/         # model.rs (entity) · dto.rs · repository.rs (SQL) · service.rs (nghiệp vụ)
  crates/api/src/
    app.rs                    # ghép router + middleware
    controllers/              # mỗi file một nhóm route
    extract.rs, middleware.rs
  crates/api/tests/           # integration test theo module
  crates/worker/src/main.rs   # gửi email
frontend/src/
  api/                        # API client (tự refresh token)
  stores/, router/, layouts/, pages/, components/, utils/
deploy/docker-compose.dev.yml
docs/implementation-plan.md
```
