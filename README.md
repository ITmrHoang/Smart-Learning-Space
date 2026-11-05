# Smart-Learning-Space
A smart technology-integrated learning environment connecting teachers and students. Một môi trường/không gian học tập tích hợp công nghệ thông minh, giúp kết nối giáo viên và học sinh

Nền tảng học và dạy trực tuyến cho học sinh lớp 1–12 và giáo viên.

| Tài liệu | Nội dung |
|---|---|
| [requirements.md](requirements.md) | Yêu cầu nghiệp vụ (đánh dấu Chốt / Đề xuất / Sau) |
| [analyst.md](analyst.md) | Phân tích kiến trúc, tech stack, module, lộ trình triển khai |
| [docs/implementation-plan.md](docs/implementation-plan.md) | Kế hoạch từng module, ca test, kết quả kiểm tra |

## Công nghệ

- **Backend:** Rust (Axum 0.8, SQLx 0.9, Tokio) — modular monolith: `crates/core` (nghiệp vụ), `crates/api` (HTTP), `crates/worker` (việc nền).
- **Frontend:** Vue 3 + Vite + Vue Router + Pinia (JavaScript), giao diện bằng [H-UI](../HUI).
- **Hạ tầng dev (Docker):** PostgreSQL 18, Valkey 9, RabbitMQ 4, Mailpit.

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
