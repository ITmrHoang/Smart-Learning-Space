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
| **chrono**, **time** | Ngày giờ (hạn token, hạn OTP), thời hạn cookie; lịch lặp hằng tuần tính theo giờ Việt Nam (UTC+7) |
| **redis** (client) | Kết nối Valkey để đếm số lần thử (giới hạn tần suất) |
| **lapin** | Kết nối RabbitMQ: API đẩy việc vào hàng đợi, worker nhận việc ra làm |
| **lettre** (rustls) | Worker gửi email qua SMTP có mã hóa TLS — STARTTLS hoặc TLS, đặt bằng `SMTP_TLS` (OTP, link đặt lại mật khẩu, kết quả duyệt giáo viên, lời mời / nhắc lịch buổi học). Lời mời kèm file lịch `.ics` (chuẩn RFC 5545, tự sinh) để học sinh thêm vào Google Calendar / Outlook / điện thoại; dời / hủy buổi thì lịch trong máy tự cập nhật |
| **deunicode** | Bỏ dấu tiếng Việt khi tạo slug / gợi ý username (`Trần Thị` → `tran.thi`) |
| **calamine** / **rust_xlsxwriter** | Đọc file Excel câu hỏi giáo viên tải lên / tạo file Excel mẫu để giáo viên điền, xuất bảng điểm lớp học |
| **zip** + **quick-xml** | Đọc file Word (.docx) đề câu hỏi giáo viên tải lên: tách "Câu 1." / phương án A–D / đáp án gạch chân, tô đỏ / lời giải, đổi công thức Equation của Word (OMML) sang LaTeX, lấy ảnh trong đề; tạo file Word mẫu |
| **hayro** 0.9 | Dự phòng nhập đề PDF lên bảng: khi trình duyệt của giáo viên không tự đọc được PDF, máy chủ vẽ từng trang thành ảnh PNG (Rust thuần, chỉ dùng CPU, không cần cài thêm chương trình); mỗi lần một trang, cùng lúc một file |
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
| `hui-markdown` + `hui-math` (trong H-UI) | Hiển thị nội dung bài học / câu hỏi: Markdown + công thức LaTeX vẽ bằng MathML gốc của trình duyệt (không cần KaTeX / MathJax) |
| `hui-timetable` (trong H-UI) | Lịch tuần / ngày có khung giờ cho Lịch dạy / Lịch học: buổi chồng giờ chia cột, vạch giờ hiện tại, kéo chọn khung giờ trống để tạo buổi học |
| `hui-flashcard` (trong H-UI) | Thẻ ghi nhớ lật được (Markdown + công thức), 4 nút tự chấm Quên / Khó / Được / Dễ cho ôn tập lặp lại ngắt quãng |
| `hui-whiteboard` (trong H-UI) | Bảng vẽ cho dạy trực tuyến (bút, hình, chữ, tẩy, nhiều trang, hoàn tác), phát thao tác để đồng bộ giữa các máy trong phòng; dạy Toán: đồ thị hàm số (ƒ), công thức LaTeX (∑), hình học phẳng (điểm tự đặt tên, đoạn / đường tròn / đa giác bắt dính điểm), nền ô vuông / ô ly / dòng kẻ / chấm / hệ trục Oxy, chèn / dán ảnh (phòng học nhập được cả đề PDF: mỗi trang PDF thành một trang bảng có ảnh nền khóa), công cụ chọn để dời – đổi cỡ – nhân bản – xóa. Phòng học tự lưu nháp bảng nên giáo viên tải lại trang không mất bảng |
| `hui-math-input` (trong H-UI) | Ô gõ công thức có bàn phím ký hiệu Toán (phân số, căn, mũ, chỉ số, hệ phương trình, π ≤ ≥ ∈ ℝ…) và xem trước — dùng cho công thức trên bảng và ô trả lời ngắn kiểu biểu thức của học sinh |
| `hui-plot` (trong H-UI) | Đồ thị hàm số từ công thức gõ tự nhiên (`y = 2x + 1`, miền nghiệm, điểm, tham số có thanh trượt), kéo / phóng to; trong nội dung Markdown dùng khối ` ```plot ` |
| `hui-slides` (trong H-UI) | Trình chiếu bài giảng: mỗi `<section>` là một slide 16:9 tự co theo khung, chuyển bằng phím ← → / Space / vuốt, toàn màn hình, xem lưới. Bài học tách slide ở dòng `---`; phòng live dùng để giáo viên chiếu, học sinh xem theo |
| Xuất Word (`src/utils/docx/`, tự viết, không thư viện) | Giáo án, các mã đề (kiểu đề THPT, mỗi mã một trang) và bảng đáp án → file `.docx` ngay trên trình duyệt: Markdown của H-UI → WordprocessingML, công thức MathML → OMML (sửa được trong Word), đồ thị → ảnh PNG, ảnh trong bài nhúng vào file; khổ A4, lề 3 / 2 / 2 / 2 cm, Times New Roman 13. Trang in riêng để "In → Lưu PDF" |
| **livekit-client** 2.23 | Kết nối trình duyệt với phòng LiveKit: video, âm thanh, chia sẻ màn hình, kênh dữ liệu (giơ tay, chat, bình chọn, bảng) |
| **pdfjs-dist** 6.5 | Nhập đề PDF lên bảng: trình duyệt của giáo viên tự đọc PDF, vẽ từng trang thành ảnh làm nền trang bảng — máy chủ không phải làm gì. Chỉ tải khi giáo viên chọn PDF (file riêng); font chuẩn / bảng mã / bộ giải mã ảnh scan nằm ở `/pdfjs/*` (Vite chép từ thư viện vào `dist/`). Không đọc được thì nhờ máy chủ (`hayro`) |
| **PWA** (manifest + service worker tự viết, không thư viện) | Cài ứng dụng lên màn hình chính điện thoại / máy tính (nút "Cài ứng dụng"), mở cửa sổ riêng; mất mạng thì hiện trang "Bạn đang ngoại tuyến". Service worker không lưu dữ liệu / API (chỉ trang ngoại tuyến + icon) nên không bao giờ giữ bản cũ của ứng dụng. Cần HTTPS khi chạy thật |
| **Vitest** + **jsdom** | Unit test cho API client, kiểm tra quyền, tiện ích SSO… |

### Hạ tầng (Docker, file `deploy/docker-compose.dev.yml`)

| Dịch vụ | Phiên bản | Tác dụng |
|---|---|---|
| **PostgreSQL** | 18.6 | Cơ sở dữ liệu chính: người dùng, phiên đăng nhập, môn học, lớp… Có sẵn `uuidv7()`; extension `pg_trgm` + `unaccent` để tìm kiếm tiếng Việt không dấu |
| **Valkey** | 9.1.3 | Bộ nhớ đệm key-value (bản mã nguồn mở của Redis): đếm số lần đăng nhập sai, gửi OTP… để giới hạn tần suất |
| **RabbitMQ** | 4.4.0 | Hàng đợi việc: API trả lời ngay, việc chậm (gửi email, sau này chuyển mã video, đọc file đề) để worker làm sau. Trang quản trị: http://127.0.0.1:15672 |
| **Mailpit** | 1.33.0 | Hộp thư giả khi dev: bắt mọi email worker gửi để xem OTP / link tại http://127.0.0.1:8025, không gửi ra ngoài |
| **Garage** | 2.5.0 | Lưu file tương thích S3, tự host: ảnh trong bài học / câu hỏi / hỏi đáp, ảnh / PDF bài làm của học sinh, ảnh đại diện giáo viên, tài liệu đính kèm bài học / giáo án (PDF, Word, Excel, PowerPoint). Trình duyệt upload / tải thẳng bằng presigned URL (API chỉ ký link và kiểm tra file), đi qua đường dẫn `/files/*` cùng domain |
| **LiveKit** | 1.13.11 | Phòng học trực tuyến tự host (WebRTC SFU): video / âm thanh / màn hình đi thẳng giữa trình duyệt và LiveKit, api chỉ cấp token theo vai trò và đổi quyền phát. Dev: `ws://127.0.0.1:7880` |

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
| **Caddy** 2.11.4 | Reverse proxy production: tự cấp HTTPS, phục vụ file tĩnh của frontend, chuyển `/api` cho backend — đã cấu hình ở `deploy/Caddyfile` |

## Chạy môi trường dev

Cần cài: Rust stable, Visual Studio Build Tools (C++), Docker Desktop, Node.js LTS.

```bash
docker compose -f deploy/docker-compose.dev.yml up -d
```

```bash
cp backend/.env.example backend/.env
```

```bash
sh deploy/garage/init.sh   # một lần: tạo bucket + khóa cho Garage (Windows: chạy bằng Git Bash)
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
| http://127.0.0.1:5173/p/lop-hoc/{id} · `/p/giao-vien/{username}` · `/sitemap.xml` | Trang công khai cho máy tìm kiếm (HTML do API dựng sẵn) |

Lần chạy đầu, API tạo tài khoản admin từ `BOOTSTRAP_ADMIN_*` trong `backend/.env` (giá trị mẫu chỉ dùng cho dev).

Mở giao diện bằng **http://127.0.0.1:5173** (không dùng `localhost`): cookie của trình duyệt tách riêng theo tên máy, nên đăng nhập Google / Microsoft chỉ hoạt động khi địa chỉ khớp `APP_BASE_URL`.

### Đăng nhập Google / Microsoft

Nút đăng nhập chỉ hiện khi có `SSO_GOOGLE_*` / `SSO_MICROSOFT_*` trong `backend/.env`.

| Nhà cung cấp | Tạo ứng dụng OAuth ở | Redirect URI (dev) |
|---|---|---|
| Google | Google Cloud Console → APIs & Services → Credentials → OAuth client ID (Web application) | `http://127.0.0.1:5173/api/auth/sso/google/callback` |
| Microsoft | Microsoft Entra admin center → App registrations → New registration (Web) | `http://127.0.0.1:5173/api/auth/sso/microsoft/callback` |

Microsoft: nên bật optional claim `xms_edov` cho id token (Token configuration). Với tài khoản trường / công ty, hệ thống chỉ tin email khi có claim này (tài khoản Microsoft cá nhân luôn được tin).

## Triển khai (production, một hoặc nhiều máy)

Mọi máy dùng **chung project và chung file** [`deploy/docker-compose.yml`](deploy/docker-compose.yml). Mỗi máy chỉ khác nhau file `deploy/.env`: máy này chạy service nào (`COMPOSE_PROFILES`) và service ở máy khác nằm ở IP / cổng nào (`*_HOST`, `*_PORT`). Mẫu có giải thích từng biến: [`deploy/.env.example`](deploy/.env.example).

```bash
cp deploy/.env.example deploy/.env      # sửa mật khẩu, IP, danh sách service của máy này
```

```bash
cd frontend && npm ci && npm run build  # chỉ cần trên máy chạy caddy
```

```bash
docker compose -f deploy/docker-compose.yml up -d --build
```

Ví dụ hai máy cùng mạng LAN / VPN, máy A có IP `10.0.0.1`:

| | Máy A (chính) | Máy B (xử lý nặng) |
|---|---|---|
| `COMPOSE_PROFILES` | `postgres,valkey,rabbitmq,api,worker,caddy` | `worker` (sau này thêm `import-worker`, `media-worker`, `ai-service`) |
| `PUBLISH_IP` | `10.0.0.1` (mở cổng cho máy B) | `127.0.0.1` |
| `RABBITMQ_HOST` | `rabbitmq` (cùng máy) | `10.0.0.1` |

- Service lưu dữ liệu (Postgres, Valkey, RabbitMQ) chỉ chạy **trên một máy**; worker chạy bao nhiêu máy cũng được, cùng lấy việc từ một hàng đợi.
- Qua Internet: nối các máy bằng WireGuard / Tailscale rồi dùng IP VPN như IP LAN. **Không** mở Postgres / Valkey / RabbitMQ ra IP public.
- Lưu file: thêm `garage` vào `COMPOSE_PROFILES`, điền `GARAGE_RPC_SECRET`, `GARAGE_ADMIN_TOKEN`, `S3_ACCESS_KEY_ID`, `S3_SECRET_ACCESS_KEY` trong `deploy/.env`, sau lần `up` đầu tiên chạy `sh deploy/garage/init.sh prod`. Caddy chuyển `/files/*` vào Garage. **Bắt buộc sao lưu** volume `garagedata` (một node chỉ có một bản dữ liệu).
- Phòng học trực tuyến: thêm `livekit` vào `COMPOSE_PROFILES`, điền `LIVEKIT_API_KEY`, `LIVEKIT_API_SECRET`, `LIVEKIT_KEYS` trong `deploy/.env`; máy chạy LiveKit cần IP public, mở **7881/tcp** và **7882/udp**. Trình duyệt vào qua `wss://<tên miền>/rtc` (Caddy chuyển vào LiveKit). Chưa cấu hình thì buổi học chỉ dùng link họp ngoài.
- Cài ứng dụng (PWA): trình duyệt chỉ cho cài khi trang chạy qua **HTTPS** (Caddy tự lấy chứng chỉ khi `SITE_ADDRESS` là tên miền). Khi dev, service worker chỉ bật ở bản build: `npm --prefix frontend run build` rồi `npm --prefix frontend run preview`.
- Image backend build từ [`backend/Dockerfile`](backend/Dockerfile) (một lần build ra `sls-api` và `sls-worker`, image distroless chạy user không phải root). Migration tự chạy khi api khởi động.

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
backend/Dockerfile            # image sls-api + sls-worker
deploy/docker-compose.dev.yml # hạ tầng khi dev
deploy/docker-compose.yml     # production, dùng chung cho mọi máy
deploy/.env.example           # cấu hình từng máy: chạy service nào, service khác ở IP nào
deploy/Caddyfile
docs/implementation-plan.md
```
