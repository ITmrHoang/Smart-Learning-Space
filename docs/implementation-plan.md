# Kế hoạch triển khai theo module — G1

> Ngày bắt đầu: 2026-11-05. Kiến trúc: `analyst.md` (v0.4). Yêu cầu: `requirements.md`.
> G1 gồm 6 module, làm **lần lượt**; xong và test đạt module trước mới sang module sau.

| Module | Nội dung | Trạng thái |
|---|---|---|
| **M0** | Khung dự án: Cargo workspace (core / api / worker), Docker Compose dev, cấu hình, lỗi thống nhất, middleware nền, health check, migrations; khung Vue 3 + H-UI | ✅ Xong (2026-11-05) |
| **M1** | Đăng ký, xác minh email bằng OTP, đăng nhập, phiên (access token + refresh token xoay vòng), đăng xuất, quên / đổi mật khẩu, giới hạn tần suất | ✅ Xong |
| **M2** | Phân quyền: vai trò → quyền, middleware kiểm tra quyền, `/api/auth/me` trả quyền, menu và route theo quyền ở frontend | ✅ Xong |
| **M3** | Admin quản lý user: danh sách (tìm, lọc, phân trang), tạo, sửa, đổi vai trò, khóa / mở khóa, gửi đặt lại mật khẩu, nhật ký thao tác | ✅ Xong |
| **M4** | Master data: môn học, lớp (Lớp 1–12 + lớp riêng), danh mục (nhãn trong môn), mức độ câu hỏi; giáo viên tự thêm, admin gộp / đưa lên danh sách chung | ✅ Xong |
| **M5** | Đăng ký làm giáo viên (chọn môn, lớp) và admin duyệt | ✅ Xong |
| **M6.1** | SSO phần 1: bảng liên kết, chính sách đăng nhập / tạo / liên kết tài khoản (chính sách đã chốt 2026-11-05), xác nhận bằng mật khẩu hoặc OTP, tự gỡ / admin gỡ liên kết | ✅ Xong (2026-11-05) |
| **M6.2** | SSO phần 2: luồng OIDC thật với Google / Microsoft, nút đăng nhập + trang liên kết ở giao diện, giới hạn số thiết bị, đặt mật khẩu lần đầu | ✅ Xong (2026-11-05) — chờ bạn tạo ứng dụng OAuth thật để thử với Google / Microsoft |

## Đợt 2 — Bài học, lớp học, câu hỏi, ôn tập / kiểm tra (G2 + G3, bắt đầu 2026-11-05)

> Yêu cầu: "làm tiếp phần tạo bài học và nội dung ôn tập, kiểm tra, các khóa cho học sinh và giáo viên; chưa cần live stream và bảng thông minh". "Khóa học" trên giao diện = **lớp học** (analyst.md v0.4: không còn tầng khóa học riêng; lớp học kiểu tự học chính là khóa tự học).

| Module | Nội dung | Trạng thái |
|---|---|---|
| **M7** | Thư viện bài: bài học (Markdown + công thức + video YouTube / link), gắn môn / lớp / danh mục, bản nháp → xuất bản, lịch sử phiên bản | ✅ Xong (2026-11-06) |
| **M8** | Lớp học + ghi danh: tạo / sửa / sao chép lớp học, hiển thị public / unlisted / private, ghi danh tự do / chờ duyệt / mã mời, giáo viên thêm / duyệt / mời ra học sinh | ✅ Xong (2026-11-06) |
| **M9** | Lộ trình + tiến độ: các bước (mốc, bài, đề) có thứ tự, kéo thả, điều kiện mở khóa, bản nháp → xuất bản; học sinh học theo lộ trình, đánh dấu hoàn thành; giáo viên xem tiến độ | ✅ Xong (2026-11-06) |
| **M10** | Ngân hàng câu hỏi: thư mục, 4 dạng câu (một đáp án, nhiều đáp án, đúng/sai 4 ý, trả lời ngắn), mức độ, danh mục, phiên bản; import Excel (xem trước → lưu) | ✅ Xong (2026-11-06) |
| **M11** | Đề ôn tập / kiểm tra: phần I/II/III, câu cố định hoặc **ma trận** (danh mục × mức độ), trộn câu trong phần + trộn đáp án theo từng lượt, đồng hồ ở server, lưu đáp án liên tục, chấm theo cấu trúc THPT, ôn tập kiểm tra từng câu, xem lại + lời giải, chấm lại khi sửa đáp án, kết quả cho giáo viên | ✅ Xong (2026-11-06) |
| **M12** | Giao diện giáo viên + học sinh; H-UI thêm `hui-math` (LaTeX → MathML) và `hui-markdown` | ✅ Xong (2026-11-06) |

**Để sau (ghi rõ để không quên):** upload video lên Garage (chưa dựng Garage — hiện dùng link YouTube / mp4), import Word / PDF, ôn tập lặp lại ngắt quãng (flashcard), trang công khai có SEO, trợ giảng, nhóm học sinh + lộ trình riêng, theo dõi % xem video, ghi nhận rời tab khi làm bài, xuất bảng điểm Excel. H-UI `hui-question`, `hui-quiz`, `hui-learning-path`, `hui-class-card` tạm làm trong app (Vue), tách sang H-UI sau.

### Thiết kế chung

- Quyền mới `teaching.manage` (giáo viên + admin). Giáo viên chỉ tạo bài / lớp học / câu hỏi / đề trong **môn đã được duyệt** (`teacher_subjects`); admin không giới hạn. Mọi nội dung thuộc về người tạo (`owner_id`), chỉ người đó sửa (admin cũng không sửa nội dung của giáo viên).
- API giáo viên dưới `/api/teaching/*`; API học sinh dưới `/api/classes/*`, `/api/my/*`, `/api/attempts/*`. Học sinh = bất kỳ tài khoản đã đăng nhập nào ghi danh vào lớp học.
- **Bài học**: bảng `lessons` là bản nháp; "Xuất bản" chép sang `lesson_revisions` (số phiên bản tăng dần). Học sinh luôn đọc bản xuất bản mới nhất → sửa bài một lần, mọi lớp học dùng bài đó đều thấy bản mới.
- **Lộ trình**: mỗi lớp học có một lộ trình chính. `path_steps` là bản nháp (giáo viên kéo thả, lưu cả danh sách); "Xuất bản" lưu ảnh chụp JSON vào `path_versions`; học sinh đọc phiên bản mới nhất. Tiến độ gắn với **ID bước** nên đổi thứ tự / sửa không mất tiến độ. Bước: `milestone` (mốc), `lesson`, `exam`. Mở khóa: `free` hoặc `previous` (phải xong bước trước, bỏ qua mốc). Bước đề có thể đặt `passScore` — chỉ tính là xong khi điểm cao nhất ≥ mức này.
- **Câu hỏi**: nội dung Markdown + LaTeX `$…$`. Mỗi lần sửa tạo `question_versions` mới (JSON: nội dung, đáp án có **ID cố định**, đáp án đúng theo ID, lời giải, có trộn đáp án hay không). Lượt làm bài trỏ tới đúng phiên bản đã làm. Sửa đáp án đúng → API báo số bài đã nộp bị ảnh hưởng → giáo viên bấm "Chấm lại".
- **Đề**: `kind = practice` (ôn tập: không giới hạn lần, kiểm tra từng câu ngay, xem lời giải) hoặc `test` (kiểm tra: thời gian làm, khung giờ mở, số lần làm tối đa, chế độ hiện đáp án). Mỗi phần có cách chấm: `per_question` (đúng hết mới được điểm) hoặc `thpt_true_false` (đúng 1/2/3/4 ý = 0,1 / 0,25 / 0,5 / 1 × điểm câu). Lượt làm lưu `attempt_items` (câu, phiên bản, thứ tự đáp án đã trộn). **Đáp án đúng không bao giờ gửi xuống trước khi nộp** (trừ câu ôn tập vừa bấm "Kiểm tra"). Hết giờ: lần gọi API kế tiếp tự nộp bài.

### Kết quả đợt 2 (2026-11-06)

Migration `20261106000002_lessons_classes_paths.sql` (`lessons`, `lesson_tags`, `lesson_revisions`, `classes`, `class_grades`, `class_tags`, `class_members`, `path_steps`, `path_versions`, `step_progress`) và `20261106000003_questions_exams.sql` (`question_folders`, `questions`, `question_tags`, `question_versions`, `exams`, `exam_sections`, `exam_section_questions`, `exam_section_rules`, `attempts`, `attempt_items`). Module backend: `core/src/modules/{lessons, classes, questions, exams}`; controller `api/src/controllers/{teaching, learning}.rs`.

| API giáo viên (`/api/teaching`) | Mô tả |
|---|---|
| `lessons` (GET, POST) · `lessons/{id}` (GET, PUT, DELETE = ẩn) · `lessons/{id}/publish` · `lessons/{id}/revisions` | Thư viện bài |
| `classes` (GET, POST) · `classes/{id}` (GET, PUT) · `…/archive` · `…/restore` · `…/copy` · `…/invite-code` | Lớp học |
| `classes/{id}/members` (GET `?status=`, POST `{login}`) · `…/members/{userId}/approve\|reject\|remove` | Học sinh của lớp |
| `classes/{id}/path` (GET, PUT `{steps}`) · `…/path/publish` · `classes/{id}/progress` | Lộ trình + tiến độ cả lớp |
| `questions` (GET lọc `q, subjectId, folderId (id\|none), tagId, level, kind`; POST) · `questions/{id}` (GET, PUT → `{question, answerChanged, affectedAttempts}`, DELETE) · `questions/{id}/regrade` | Ngân hàng câu hỏi |
| `questions/import/template` (tải .xlsx) · `questions/import/preview?subjectId=` (body = file .xlsx) · `questions/bulk` | Import Excel |
| `question-folders` (GET, POST) · `question-folders/{id}` (PUT, DELETE) | Thư mục |
| `exams` (GET, POST) · `exams/{id}` (GET, PUT, DELETE) · `exams/{id}/preview` (làm thử) · `exams/{id}/attempts?classId=` | Đề + kết quả |

| API học sinh | Mô tả |
|---|---|
| `GET /api/classes` (lớp công khai, lọc `q, subjectId, gradeId`) · `GET /api/classes/{id}` (giới thiệu + mục lục) | Tìm lớp học |
| `POST /api/classes/{id}/enroll` · `POST /api/classes/join {code}` · `POST /api/classes/{id}/leave` · `GET /api/my/classes` | Ghi danh, lớp của tôi (kèm % tiến độ) |
| `GET /api/classes/{id}/learn` · `GET …/steps/{stepId}/lesson` · `POST …/steps/{stepId}/complete` | Học theo lộ trình |
| `POST …/steps/{stepId}/attempts` (bắt đầu / làm tiếp) · `GET …/steps/{stepId}/attempts` | Làm đề trong lớp |
| `GET /api/attempts/{id}` · `PUT …/items/{position}` (lưu câu trả lời) · `POST …/items/{position}/check` (ôn tập) · `POST …/submit` | Làm bài, xem lại |

**Quyết định khi làm:**

| Chủ đề | Quyết định |
|---|---|
| Điểm | Lưu `double precision`, làm tròn 0,01 trong code (không thêm crate số thập phân) |
| Trộn | Trộn câu **trong từng phần** (giữ thứ tự phần), trộn phương án câu một / nhiều đáp án; câu đúng/sai giữ thứ tự ý a–d; câu bỏ chọn "cho phép trộn" thì không trộn |
| Ma trận | Ô ma trận = danh mục × mức độ × dạng câu × thư mục (đều tùy chọn) × số câu; lưu đề báo ô nào không đủ câu; mỗi lượt rút ngẫu nhiên, không trùng câu cố định / câu đã rút |
| Lượt làm dở | Bấm "Làm bài" lần nữa thì làm tiếp lượt cũ; hết giờ (cho trễ 15 giây) thì lần gọi kế tiếp tự nộp |
| Hiện đáp án | Ôn tập: luôn hiện sau khi nộp + từng câu khi bấm "Kiểm tra". Kiểm tra: ngay sau khi nộp / sau giờ đóng đề / không hiện (vẫn có điểm) |
| Bước đề "xong" | Đã nộp ≥ 1 lần và điểm cao nhất ≥ `passScore` (nếu có) |
| Import | Gửi nguyên file .xlsx làm body (không multipart), tối đa 1 MB / 500 câu; danh mục ghi theo tên, tên không có thì cảnh báo và bỏ qua |
| Giao diện câu hỏi / làm bài / lộ trình | Làm bằng component Vue trong app (`QuestionView`, `AttemptPage`, `PathEditor`, `ClassCard`); kéo thả lộ trình dùng `hui-sortable` có sẵn. Tách thành `hui-question` / `hui-quiz` / `hui-learning-path` / `hui-class-card` sau |
| H-UI mới | `hui-math` (LaTeX → MathML, zero-dependency) và `hui-markdown` (Markdown + công thức, dựng DOM an toàn) — commit `2abf0d2` trong repo H-UI, changeset minor |

| Kiểm tra | Kết quả |
|---|---|
| Backend unit test (`sls-core`) | 40 / 40 đạt (thêm: chấm điểm THPT / trả lời ngắn, đọc Excel + file mẫu đọc lại được, mở khóa lộ trình, `next` an toàn…) |
| Backend integration test | 79 / 79 đạt — thêm `lessons` 2, `questions` 4, `exams` 6, `classes` 4 (ca chính: xuất bản / phiên bản bài; quyền môn dạy; câu hỏi 4 dạng + phiên bản giữ ID; thư mục; import Excel xem trước → lưu, lỗi một câu không lưu câu nào; đề không lộ đáp án trước khi nộp; chấm THPT 1,25 / 2; trộn khác nhau mỗi lượt; ma trận rút đúng ô; ôn tập kiểm tra từng câu + khóa câu; sửa đáp án → chấm lại; ghi danh 3 chế độ + mã mời + mời ra; lộ trình xuất bản / khóa bước / điểm đạt / đổi thứ tự không mất tiến độ; đề đóng, hết giờ tự nộp, hết lượt; sao chép / lưu trữ lớp) |
| `cargo fmt --check`, `cargo clippy -D warnings` | Đạt |
| Frontend (Vitest) | 29 / 29 đạt (thêm video YouTube → nhúng, đồng hồ theo giờ server, câu đã làm, menu Học tập / Dạy học); `npm run build` đạt |
| H-UI | 148 unit test + 7 story test đạt; `npm run build`, `build:cdn`, `check:types` đạt |
| Trình duyệt | Ngân hàng câu hỏi hiển thị công thức (cả hệ phương trình), xem đáp án; soạn bài có xem trước Markdown + bảng + công thức, nhúng YouTube, xuất bản v1; tạo lớp học, thêm mốc / bài / 2 đề, **kéo thả** đổi thứ tự, xuất bản; học sinh tìm lớp → vào học → học bài → "Đã học xong" → làm "Kiểm tra 15 phút" (đồng hồ, lưu tự động, 5/5 câu) → nộp → 1,75 / 2,25 đúng kỳ vọng, xem lại có tô đúng / sai / một phần + lời giải; bước ôn tập mở khóa sau khi xong đề, "Kiểm tra đáp án" từng câu; giáo viên xem tab Tiến độ |

**Dữ liệu thử còn trong DB dev:** lớp học "Toán 9 — ôn thi vào 10 (tự học)", bài "Bài 1. Căn bậc hai", 5 câu hỏi, 2 đề (của admin) và học sinh thử `hs.thu` (mật khẩu `MatKhau123`, chỉ dùng cho dev).

## Đợt 3 — Nhóm học sinh, lịch & buổi học, thông báo (G4, bắt đầu 2026-11-08)

> Yêu cầu: "tiếp tục". Theo `analyst.md` mục 7, sau G2 + G3 là G4. Vẫn **chưa** làm live stream (LiveKit) và bảng thông minh: buổi học tạm dùng **link họp ngoài** (Google Meet / Zoom…) do giáo viên dán vào; khi có phòng live (G5) thì nút "Vào học" chuyển sang `/live/{mã buổi}`.

| Module | Nội dung | Trạng thái |
|---|---|---|
| **M13** | Nhóm học sinh: giáo viên tạo nhóm từ học sinh các lớp học của mình (một lớp hoặc nhiều lớp); dùng để mời buổi học | ✅ Xong (2026-11-08) |
| **M14** | Lịch & buổi học kiểu Google Meet: buổi lẻ hoặc lặp hằng tuần (chọn thứ, ngày kết thúc); mời lớp học / nhóm / từng học sinh; ai được vào (người được mời · thành viên lớp · mọi tài khoản); link họp; bài sẽ dạy; chặn trùng lịch giáo viên, cảnh báo học sinh trùng lịch; sửa / dời / hủy "chỉ buổi này" hoặc "buổi này và các buổi sau"; email lời mời kèm `.ics` (cập nhật / hủy dùng cùng `UID`, tăng `SEQUENCE`); học sinh trả lời Tham gia / Không; "Vào học" mở trước 10 phút, ghi điểm danh; giáo viên điểm danh tay | ✅ Xong (2026-11-08) |
| **M15** | Thông báo trong app (chuông, số chưa đọc) + nhắc lịch qua email và trong app trước giờ học (15 phút / 1 giờ / 1 ngày); thông báo khi được mời / dời / hủy buổi, được duyệt vào lớp, lộ trình có bản mới | ✅ Xong (2026-11-08) |
| **M16** | Buổi học là một bước của lộ trình (xong khi có điểm danh); trang lớp học có "Buổi học sắp tới"; trang chủ có lịch 7 ngày tới + học tiếp; H-UI thêm `hui-timetable` (lịch tuần / ngày có khung giờ, kéo chọn khung giờ để tạo buổi) | ✅ Xong (2026-11-08) |

**Để sau:** bài tập về nhà nộp ảnh / PDF (cần dựng Garage để lưu file — làm cùng upload video), lộ trình riêng cho nhóm / học sinh, báo cáo email cho phụ huynh, đặt lịch học kèm 1-1 (D15), đồng bộ hai chiều Google Calendar, thông báo realtime qua WebSocket (hiện hỏi lại mỗi 60 giây).

**Thiết kế:**
- Giờ lưu UTC; tính lịch lặp và hiển thị theo giờ Việt Nam (UTC+7, không đổi giờ mùa hè). Lịch lặp sinh sẵn từng buổi (`sessions`, tối đa 200 buổi / lịch) và nhớ luật ở `schedules` để sửa "buổi này và các buổi sau".
- Người được mời = học sinh đang học của lớp được mời ∪ thành viên nhóm được mời ∪ học sinh được mời riêng — tính lúc xem, nên học sinh vào lớp sau vẫn thấy các buổi sắp tới. Email chỉ gửi cho người được mời tại thời điểm tạo / sửa.
- Nhắc lịch: tiến trình `api` chạy vòng 60 giây, "nhận" buổi cần nhắc bằng `UPDATE … WHERE reminded_at IS NULL RETURNING` nên chạy nhiều instance cũng không gửi trùng.
- File `.ics` tự sinh theo RFC 5545 (không thêm crate): `METHOD:REQUEST` / `METHOD:CANCEL`, `UID` = `{id buổi}@smart-learning-space`, `SEQUENCE` tăng mỗi lần dời / hủy.

### Kết quả đợt 3 (2026-11-08)

Migration `20261108000001_groups_sessions_notifications.sql` (`student_groups`, `student_group_members`, `schedules`, `sessions`, `session_invitees`, `session_lessons`, `session_attendees`, `notifications`; `path_steps` thêm loại `session` + cột `session_id`). Module backend: `core/src/modules/{groups, sessions, notifications}` (`sessions/time.rs` giờ VN + lịch lặp, `sessions/ics.rs` sinh `.ics`); email có file đính kèm (`jobs::Attachment`, worker gửi email nhiều phần).

| API giáo viên (`/api/teaching`) | Mô tả |
|---|---|
| `students` (GET `?classId=&q=`) | Học sinh đang học trong các lớp học của mình (để chọn vào nhóm / mời) |
| `groups` (GET, POST) · `groups/{id}` (GET, PUT, DELETE) | Nhóm học sinh |
| `sessions` (GET `?from=&to=&classId=`, tối đa 93 ngày; POST, có `repeat: { weekdays, until }`) | Lịch dạy, tạo buổi lẻ / lặp → `{ sessions, studentConflicts }` |
| `sessions/{id}` (PUT `{…, scope: this\|following}`) · `sessions/{id}/cancel` (`{scope}`) · `sessions/{id}/attendance` (PUT `{userId, attended}`) | Sửa / dời / hủy, điểm danh tay |

| API mọi tài khoản | Mô tả |
|---|---|
| `GET /api/my/sessions?from=&to=&classId=` | Lịch học (buổi được mời, kèm trả lời của mình) |
| `GET /api/sessions/{id}` | Chi tiết buổi; giáo viên thêm danh sách mời + học sinh / trả lời / điểm danh; có `canJoin`, `joinOpensAt`, `invited`, `attended` |
| `POST /api/sessions/{id}/rsvp {response: yes\|no}` · `POST /api/sessions/{id}/join` | Trả lời tham gia; "Vào học" (trả link họp, ghi điểm danh) |
| `GET /api/notifications?unread=&page=` · `GET …/unread-count` · `POST …/read-all` · `POST …/{id}/read` | Chuông thông báo |

**Quyết định khi làm:**

| Chủ đề | Quyết định |
|---|---|
| Trùng lịch | Giáo viên trùng giờ với buổi khác của mình → chặn (`409 SESSION_CONFLICT`); học sinh trùng → vẫn tạo, trả `studentConflicts` để giao diện cảnh báo |
| Sửa "buổi này và các buổi sau" | Dời mọi buổi sau cùng một khoảng chênh giờ, áp nội dung mới; buổi đã hủy không đổi; mỗi lần dời tăng `SEQUENCE` và gửi lại `.ics` |
| Nhắc lịch | Trước giờ học `remindMinutes` (0 = không nhắc); đổi giờ thì nhắc lại; email nhắc không kèm `.ics` (đã có từ lời mời) |
| Bước "buổi học" | Chỉ chọn buổi của chính lớp học đó; xong khi học sinh được điểm danh (tự động khi bấm "Vào học", hoặc giáo viên đánh dấu) |
| Giờ trên giao diện | Hiển thị theo giờ của máy (người dùng ở Việt Nam = UTC+7, trùng giờ backend dùng cho lịch lặp và email) |
| "Vào học" | Là link `<a target="_blank">` tới phòng họp ngoài (không bị chặn pop-up), đồng thời gọi `join` để điểm danh; trang tự mở nút khi tới giờ |
| Chuông | Hỏi số chưa đọc mỗi 60 giây khi tab đang mở + khi quay lại tab; danh sách trong `hui-drawer`, bấm thông báo → đánh dấu đã đọc + mở link |
| H-UI mới | `hui-timetable` (lịch tuần / ngày có khung giờ: sự kiện chồng giờ chia cột, vạch giờ hiện tại, kéo chọn khung giờ → `hui-slot-select`, đổi tuần → `hui-range-change`); icon `bell` đăng ký trong app bằng `registerIcon` |

| Kiểm tra | Kết quả |
|---|---|
| Backend unit test (`sls-core`) | 46 / 46 đạt (thêm: lịch lặp theo giờ VN, ngày đầu không thuộc thứ đã chọn, quá 200 buổi, định dạng giờ VN, `.ics` escape + gập dòng 75 byte, hủy dùng `METHOD:CANCEL`) |
| Backend integration test | 84 / 84 đạt — thêm `schedule` 5: nhóm học sinh (chỉ học sinh của mình); tạo buổi mời lớp học → email + `.ics`, học sinh thấy trên lịch, người ngoài 404, học sinh vào lớp sau vẫn thấy, quyền "mọi tài khoản"; lịch lặp + trùng lịch + sửa / hủy theo phạm vi (`SEQUENCE` tăng); trả lời, "Vào học" chưa mở / đã mở, điểm danh, nhắc lịch không gửi trùng; bước buổi học trong lộ trình + thông báo duyệt / thêm vào lớp / lộ trình mới |
| `cargo fmt --check`, `cargo clippy -D warnings` | Đạt |
| Frontend (Vitest) | 37 / 37 đạt (thêm giờ máy ↔ ISO, thứ trong tuần, định dạng giờ buổi, trạng thái buổi theo thời điểm, màu sự kiện trên lịch, gộp danh sách, menu Lịch học / Lịch dạy / Nhóm học sinh); `npm run build` đạt |
| H-UI | 160 unit test (thêm 12 cho `hui-timetable`) + 5 story test `hui-timetable` đạt; `npm run build`, `build:cdn`, `check-cdn`, `check:types` đạt. Sửa luôn trang kiểm tra CDN chưa nạp nhóm `learning` (lỗi từ đợt 2) |
| Trình duyệt | Giáo viên: Lịch dạy (tuần, vạch giờ hiện tại, sang tuần sau tự cuộn tới buổi đầu tiên), **kéo chọn** 14:00 thứ Tư → form điền sẵn giờ, chọn lớp học tự mời cả lớp, lặp thứ Tư + thứ Sáu 8 tuần → "Đã tạo 16 buổi học"; sửa thời lượng "buổi này và các buổi sau" → cập nhật 16 buổi, email gửi lại `.ics` 16 sự kiện `SEQUENCE:1`; trang buổi học có danh sách học sinh / trả lời / điểm danh; nhóm học sinh sửa + lưu; lộ trình có bước "Buổi học". Học sinh: chuông 6 thông báo (mời, đổi giờ, nhắc lịch, lộ trình mới), bấm thông báo nhắc lịch → buổi sắp bắt đầu → "Vào học" → "Đã điểm danh" → bước buổi học trong lộ trình thành "đã xong" (tiến độ 3/4); trả lời "Tham gia"; trang chủ "Lịch 7 ngày tới" + "Học tiếp"; Lịch học trên điện thoại (375px) tự xem theo ngày, không tràn ngang |

**Dữ liệu thử thêm trong DB dev:** nhóm "Nhóm ôn thi chuyên" (có `hs.thu`), buổi "Chữa đề kiểm tra chương 2" (8/11, là bước 4 của lộ trình, `hs.thu` đã điểm danh), lịch lặp "Ôn thi vào 10 — Hệ phương trình" (thứ Ba / thứ Năm 19:30, 8 buổi) và "Học kèm: phương trình bậc hai" (thứ Tư / thứ Sáu 14:00–16:00, 16 buổi).

## Đợt 4 — Báo cáo, hỏi đáp, thẻ ghi nhớ, phụ huynh (bắt đầu 2026-11-13)

> Yêu cầu: "làm tiếp các tính năng". Live (LiveKit) + bảng thông minh (G5) vẫn để sau: phiên khác đang làm phần tách service / hàng đợi / triển khai nhiều máy — nền cho live và AI. Đợt này chỉ làm phần không cần hạ tầng mới (không Garage, không sửa worker, không thêm crate).

| Module | Nội dung | Trạng thái |
|---|---|---|
| **M17** | Báo cáo lớp học cho giáo viên (D11): bảng điểm (điểm cao nhất từng đề), tỷ lệ điểm theo danh mục và mức độ, câu sai nhiều nhất, học sinh cần chú ý (tiến độ thấp / điểm thấp / lâu không học), xuất bảng điểm Excel | ✅ Xong (2026-11-13) |
| **M18** | Hỏi đáp theo bài (D3): học sinh hỏi trong lớp học (gắn với bước bài / đề), giáo viên và bạn cùng lớp trả lời bằng Markdown + công thức; giáo viên ghim, chọn câu trả lời đúng, ẩn nội dung; báo vi phạm và giáo viên xử lý (D7, phần hỏi đáp); thông báo khi có câu hỏi / trả lời | ✅ Xong (2026-11-13) |
| **M19** | Thẻ ghi nhớ ôn tập lặp lại ngắt quãng: giáo viên soạn bộ thẻ (Markdown + công thức) hoặc tạo từ câu hỏi; bộ thẻ là một bước của lộ trình; học sinh ôn, tự chấm Quên / Khó / Được / Dễ, hệ thống hẹn ngày ôn lại; trang "Ôn tập hôm nay"; H-UI thêm `hui-flashcard` | ✅ Xong (2026-11-13) |
| **M20** | Phụ huynh (analyst.md "Thông tin phụ huynh"): học sinh khai email phụ huynh → phụ huynh bấm link xác nhận đồng ý; báo cáo tuần tự gửi tối Chủ nhật (tiến độ, điểm, buổi học có mặt / vắng, thẻ đã ôn); giáo viên gửi ngay kèm nhận xét; link hủy nhận; lịch sử gửi | ✅ Xong (2026-11-13) |

**Để sau:** live + bảng thông minh (G5); bài tập nộp ảnh / PDF và ảnh trong hỏi đáp (cần Garage); email báo cáo dạng HTML (worker đã có `mailer.rs` từ phiên triển khai — thêm phần HTML sau); lộ trình riêng cho nhóm / học sinh; đặt lịch học kèm 1-1 (D15); trang công khai có SEO (D14).

### Kết quả đợt 4 (2026-11-13)

Migration `20261113000001_reports_qa_flashcards_guardians.sql` (`class_threads`, `thread_posts`, `content_reports`, `decks`, `cards`, `card_reviews`, `card_review_log`, `student_guardians`, `report_deliveries`; `path_steps` thêm loại `deck` + cột `deck_id`). Module backend: `classes/report.rs`, `discussions`, `flashcards` (`srs.rs` lịch ôn), `guardians` (`report.rs` nội dung báo cáo); controller `api/src/controllers/community.rs`. Báo cáo tuần chạy chung vòng 60 giây với nhắc lịch trong `api`.

| API giáo viên (`/api/teaching`) | Mô tả |
|---|---|
| `classes/{id}/report` · `classes/{id}/report.xlsx` | Báo cáo lớp học (JSON) + bảng điểm Excel 3 sheet (Bảng điểm · Theo danh mục · Câu sai nhiều) |
| `classes/{id}/content-reports` (GET `?status=open\|all`) · `content-reports/{id}/resolve` (`{action: hide\|dismiss}`) | Báo vi phạm trong hỏi đáp |
| `classes/{id}/students/{userId}/guardian-report` (POST `{note}`) | Gửi ngay báo cáo 7 ngày của lớp học cho phụ huynh đã xác nhận |
| `decks` (GET `?q=&subjectId=`, POST) · `decks/{id}` (GET, PUT, DELETE = ẩn) · `decks/{id}/from-questions` (`{questionIds}`) | Bộ thẻ ghi nhớ |

| API học sinh / mọi tài khoản | Mô tả |
|---|---|
| `classes/{id}/threads` (GET `?stepId=&status=all\|open\|answered\|mine&q=`, POST) · `classes/{id}/qa-counts` | Hỏi đáp của lớp học |
| `threads/{id}` (GET, PUT, DELETE) · `threads/{id}/posts` · `threads/{id}/moderate` (`pin\|unpin\|hide\|unhide`) · `posts/{id}` (PUT, DELETE) · `posts/{id}/moderate` (`accept\|unaccept\|hide\|unhide`) · `content-reports` (POST) | Câu hỏi, trả lời, quản lý, báo vi phạm |
| `classes/{id}/steps/{stepId}/deck` · `classes/{id}/steps/{stepId}/reviews` (`{cardId, rating: again\|hard\|good\|easy}`) · `my/reviews` | Ôn thẻ trong lộ trình, "Ôn tập hôm nay" |
| `my/guardians` (GET, POST) · `my/guardians/{id}` (PUT, DELETE) · `my/guardians/{id}/resend` | Học sinh khai phụ huynh |
| `guardians/consent` (GET `?token=`, POST `{token, accept}`) · `guardians/subscription` (GET `?token=`) · `guardians/unsubscribe` · `guardians/subscribe` (`{token}`) | Phụ huynh bấm link trong email — **không cần đăng nhập** |

**Quyết định khi làm:**

| Chủ đề | Quyết định |
|---|---|
| Báo cáo | Chỉ tính bài đã nộp trong lớp học (bỏ làm thử). Tỷ lệ điểm danh mục / mức độ = tổng điểm đạt / tổng điểm tối đa (câu đúng sai THPT tính theo phần điểm). "Cần chú ý": lâu không học (≥ 7 ngày không có hoạt động: học bài, làm đề, vào buổi học, ôn thẻ), tiến độ < 30 % sau 7 ngày vào lớp, điểm trung bình các đề < 50 % |
| Hỏi đáp | Giáo viên chủ lớp + học sinh đang học; câu hỏi có thể gắn một bước của lộ trình. Ẩn câu hỏi → học sinh không thấy; ẩn trả lời → hiện "đã bị ẩn". Người hỏi hoặc giáo viên chọn câu trả lời đúng. Người hỏi chỉ xóa được câu hỏi chưa có trả lời. Báo vi phạm: không báo nội dung của mình, mỗi người báo một lần; giáo viên ẩn hoặc bỏ qua (đóng mọi báo cáo của nội dung đó). Chưa có ảnh (cần Garage) |
| Thẻ ghi nhớ | Lịch ôn kiểu SM-2 rút gọn: Quên → học lại sau 10 phút; Khó ×1,2; Được 1 → 3 ngày → × độ dễ; Dễ 4 ngày → × độ dễ × 1,3; độ dễ 1,3–3,0; tối đa 365 ngày. Lịch ôn gắn với (học sinh, thẻ) nên một bộ thẻ dùng ở nhiều lớp học vẫn chung lịch. Sửa thẻ giữ ID → giữ lịch ôn. Bước "thẻ ghi nhớ" xong khi đã ôn mọi thẻ ít nhất một lần; bộ thẻ trống / đã ẩn không xuất bản được. Thẻ từ câu hỏi: mặt trước = đề + phương án, mặt sau = đáp án đúng + lời giải |
| Phụ huynh | Tối đa 3 người; email không trùng, không phải email của chính học sinh. Link xác nhận hạn 14 ngày (lưu hash), gửi lại tối đa 1 lần / 10 phút. Link hủy nhận dùng token riêng của từng phụ huynh (chỉ hủy / nhận lại được). Báo cáo tuần gửi từ 19h tối Chủ nhật (giờ VN), kỳ từ 0h thứ Hai; mỗi phụ huynh một lần / tuần nhờ chỉ mục duy nhất trên `report_deliveries` (chạy nhiều instance không gửi trùng). Email dạng chữ |
| Giao diện | Trang quản lý lớp học thêm tab **Báo cáo** và **Hỏi đáp** (`?tab=bao-cao` / `?tab=hoi-dap`, số câu chưa giải đáp / báo vi phạm trên tab); trang lớp học của học sinh có khu Hỏi đáp (`#hoi-dap`), bài học có nút "Hỏi về bài này"; trang ôn thẻ dùng `hui-flashcard` (phím Space lật, 1–4 chấm; "Quên" gặp lại cuối lượt); Hồ sơ của học sinh có mục Phụ huynh; trang chủ hiện số thẻ cần ôn |
| H-UI mới | `hui-flashcard` (thẻ lật 3D, Markdown + công thức, 4 nút chấm kèm gợi ý, `reset()` về mặt trước tức thì để không lộ đáp án thẻ kế) |

| Kiểm tra | Kết quả |
|---|---|
| Backend unit test (`sls-core`) | 56 / 56 đạt (thêm: lịch ôn SM-2 — thẻ mới, nhớ liên tiếp, quên, giới hạn; mặt sau thẻ từ câu hỏi; cờ cần chú ý; tỷ lệ / cắt chuỗi; kỳ báo cáo tuần; nội dung email báo cáo; số kiểu Việt Nam) |
| Backend integration test | 88 / 88 đạt — thêm `community` 4: báo cáo lớp học (điểm, danh mục, mức độ, câu sai, cờ điểm thấp, quyền, file Excel); hỏi đáp (gắn bước, thông báo câu hỏi / trả lời / báo vi phạm, chọn câu trả lời đúng, lọc, quyền, ẩn qua xử lý báo cáo, ẩn trả lời, xóa); thẻ ghi nhớ (soạn, thêm từ câu hỏi, giữ ID khi sửa, bộ trống không xuất bản được, lịch ôn, bước xong, "Ôn tập hôm nay", giáo viên xem thử không lưu, tiến độ cả lớp); phụ huynh (khai, trùng / email của mình / quá sớm / quá 3, xác nhận, gửi ngay kèm nhận xét, báo cáo tuần đúng một lần, hủy nhận / nhận lại, từ chối) |
| `cargo fmt --check`, `cargo clippy -D warnings` | Đạt |
| Frontend (Vitest) | 40 / 40 đạt (thêm định dạng khoảng ôn, hàng đợi ôn, "Quên" học lại cuối lượt, menu Ôn tập hôm nay / Thẻ ghi nhớ); `npm run build` đạt |
| H-UI | 163 unit test + 2 story test `hui-flashcard` đạt; `npm run build`, `build:cdn`, `check-cdn`, `check:types` đạt |
| Trình duyệt | Giáo viên: tab Báo cáo (4 chỉ số, bảng điểm, mức độ, câu sai nhiều), gửi báo cáo phụ huynh kèm nhận xét → email trong Mailpit đúng nội dung + link hủy nhận; tab Hỏi đáp (đếm câu chưa giải đáp), trang câu hỏi có công thức ở tiêu đề / nội dung, trả lời được; soạn bộ thẻ, xem thử thẻ lật. Học sinh: trang chủ "4 thẻ ghi nhớ cần ôn", Ôn tập hôm nay → ôn thẻ (lật bằng click, chấm bằng phím 3, thẻ kế hiện ngay mặt trước), trang lớp học có bước thẻ "3 thẻ cần ôn" + Hỏi đáp (cuộn tới `#hoi-dap`); Hồ sơ → thêm phụ huynh → mở link xác nhận (trang công khai) → đồng ý; trang hủy nhận hiển thị đúng |

**Dữ liệu thử thêm trong DB dev:** bộ thẻ "Căn bậc hai — ghi nhớ nhanh" (6 thẻ, là bước cuối của lộ trình "Toán 9"), 2 câu hỏi trong hỏi đáp (1 đã giải đáp), phụ huynh `bo.thu@example.com` và `me.thu@example.com` của `hs.thu` (đã xác nhận).

## Đợt 5 — Lưu file, bài tập về nhà, học kèm 1-1, lộ trình riêng, trang công khai (bắt đầu 2026-11-13)

> Yêu cầu: "còn tính năng nào làm nốt cho tôi". Làm nốt các phần của G2–G4 và đề xuất đã chốt còn treo vì chưa có chỗ lưu file. Live (LiveKit) + bảng thông minh (G5 / G6), thanh toán (G7), AI (G8) để các đợt riêng.

| Module | Nội dung | Trạng thái |
|---|---|---|
| **M21** | Lưu file trên **Garage 2.5** (tương thích S3, tự host): trình duyệt upload / tải thẳng bằng presigned URL qua `/files/*` cùng domain; API ký link (AWS SigV4 viết tay, không thêm crate), kiểm tra file sau khi upload (có thật, kích thước, chữ ký định dạng). Chèn ảnh vào mọi ô soạn Markdown (bấm nút, dán, kéo thả) | ✅ Xong (2026-11-13) |
| **M22** | Bài tập về nhà (D2 + D9): thư viện bài tập của giáo viên; bước "Bài tập" trong lộ trình có hạn nộp; học sinh nộp ảnh / PDF (chụp bằng điện thoại, ảnh lớn tự thu nhỏ); giáo viên chấm điểm, nhận xét, **vẽ nhận xét lên ảnh**, trả lại để làm lại; nhắc hạn nộp trước 24 giờ (trong app + email) | ✅ Xong (2026-11-13) |
| **M23** | Đặt lịch học kèm 1-1 (D15): giáo viên mở khung giờ rảnh hằng tuần; học sinh chọn giờ còn trống, gửi yêu cầu; giáo viên nhận → tự tạo buổi học + lời mời `.ics`; từ chối / hủy có lý do | ✅ Xong (2026-11-13) |
| **M24** | Lộ trình riêng cho nhóm / học sinh: mỗi bước chọn "Dành cho" (cả lớp, nhóm, từng học sinh); học sinh chỉ thấy và chỉ tính tiến độ các bước được giao | ✅ Xong (2026-11-13) |
| **M25** | Hồ sơ giáo viên công khai (D1) + trang công khai cho máy tìm kiếm (D14): trang HTML dựng sẵn `/p/giao-vien/{username}`, `/p/lop-hoc/{id}` (tiêu đề, mô tả, Open Graph, schema.org), `sitemap.xml`, `robots.txt`; trang giáo viên trong ứng dụng có lớp học công khai + đặt lịch học kèm | ✅ Xong (2026-11-13) |

**Để sau:** live + bảng thông minh (G5 / G6), ghi hình buổi live (D4), upload video bài giảng + chuyển HLS (cần `media-worker` + FFmpeg; Garage đã sẵn), import Word / PDF (cần `import-worker`), câu hỏi chèn giữa video (D5), watermark video (D6), trợ giảng (D12), PWA (D10), email báo cáo dạng HTML, thông báo realtime qua WebSocket, thanh toán (G7), AI (G8). Dọn file `pending` không ai báo xong (sau 1 ngày) chưa làm.

### Kết quả đợt 5 (2026-11-13)

Migration `20261113000002_files_homework_tutoring_profiles.sql` (`files`, `assignments`, `homework_submissions`, `submission_files`, `homework_due_reminders`, `tutoring_offers`, `tutoring_bookings`, `teacher_profiles`; `path_steps` thêm loại `homework` + cột `assignment_id`, `due_at`, `audience_group_ids`, `audience_user_ids`). Mã: `core/src/storage.rs` (Garage / S3, kho bộ nhớ cho test), module `files`, `assignments`, `classes/homework.rs`, `tutoring`, `profiles`; `sessions::create_booked` / `cancel_by_id` / `busy` cho học kèm; controller `api/src/controllers/{files, coursework}.rs`. Hạ tầng: Garage trong `deploy/docker-compose.dev.yml` + `deploy/docker-compose.yml` (profile `garage`), `deploy/garage/{garage.dev.toml, garage.toml, init.sh}`, Caddy chuyển `/files/*` vào Garage và `/p/*`, `/sitemap.xml`, `/robots.txt` vào api.

| API | Mô tả |
|---|---|
| `POST /api/files` (`{purpose: content\|homework, name, contentType, size}` → link upload) · `POST /api/files/{id}/complete` · `GET /api/files/{id}` (chuyển hướng sang presigned URL; file bài làm cần `?exp=&sig=`) | Lưu file |
| `teaching/assignments` (GET, POST) · `teaching/assignments/{id}` (GET, PUT, DELETE = ẩn) | Thư viện bài tập |
| `classes/{id}/steps/{stepId}/homework` (GET, PUT `{note, fileIds}`) | Học sinh xem đề, nộp / nộp lại |
| `teaching/classes/{id}/homework` · `teaching/classes/{id}/steps/{stepId}/submissions` · `teaching/submissions/{id}` · `teaching/submissions/{id}/grade` (`{action: grade\|return, score, feedback, annotations}`) | Giáo viên xem bài nộp, chấm, trả lại |
| `teaching/tutoring/offers` (GET, POST) · `teaching/tutoring/offers/{id}` (PUT) · `teaching/tutoring/bookings?status=` · `…/bookings/{id}/approve\|decline\|cancel` | Giáo viên: khung giờ, yêu cầu đặt lịch |
| `tutoring/offers` · `tutoring/offers/{id}` · `tutoring/offers/{id}/slots` · `tutoring/offers/{id}/bookings` (POST) · `my/bookings` · `my/bookings/{id}/cancel` | Học sinh đặt lịch học kèm |
| `teaching/profile` (GET, PUT) · `public/teachers/{username\|id}` (không cần đăng nhập) | Hồ sơ giáo viên |
| `/p/giao-vien/{username}` · `/p/lop-hoc/{id}` · `/sitemap.xml` · `/robots.txt` (ngoài `/api`, trả HTML / XML / text) | Trang cho máy tìm kiếm |

**Quyết định khi làm:**

| Chủ đề | Quyết định |
|---|---|
| Upload | Đúng như `analyst.md`: trình duyệt PUT thẳng lên Garage bằng presigned URL (15 phút), `Content-Type` nằm trong chữ ký (gửi sai kiểu → Garage từ chối). Bucket tên `files` nên URL path-style `/files/{key}` dùng chung domain. Sau khi upload, API HEAD + đọc 16 byte đầu: sai kích thước / sai định dạng → xóa file. Ảnh nội dung ≤ 5 MB (PNG, JPG, GIF, WebP); bài làm ≤ 15 MB (thêm PDF), ảnh chụp lớn hơn 2.200 px được thu nhỏ trên trình duyệt |
| Xem file | `GET /api/files/{id}` chuyển hướng sang presigned GET 1 giờ, ép `Content-Type` + `Content-Disposition` (Garage không bao giờ trả được trang HTML). Ảnh nội dung: ai có link cũng xem (ID ngẫu nhiên 122 bit) để `<img>` trong Markdown hiện được. Bài làm: link có chữ ký HMAC hết hạn ~6 giờ, chỉ cấp cho học sinh đó và giáo viên chủ lớp |
| Dev | Vite chuyển `/files` vào Garage với `changeOrigin: false` (dạng chuỗi của Vite tự đổi `Host` → chữ ký sai, đã gặp khi thử). Khóa dev cố định trong `backend/.env.example`; `sh deploy/garage/init.sh` tạo layout + bucket + khóa (chạy lại không sao) |
| Bài tập | Mỗi học sinh một bài nộp / bước; nộp lại thay toàn bộ file khi chưa chấm hoặc bị trả lại; quá hạn vẫn nộp (đánh dấu muộn). Bước xong khi đã nộp (bị trả lại = chưa xong). Thang điểm tùy chọn (không có = chỉ nhận xét). Vẽ nhận xét: canvas trên trình duyệt ghép ảnh gốc + nét vẽ thành ảnh JPEG mới, lưu cạnh ảnh gốc (học sinh xem được cả hai) |
| Nhắc hạn nộp | Vòng 60 giây của `api`: bước bài tập còn ≤ 24 giờ tới hạn → "nhận" (bước, hạn nộp) bằng `INSERT … ON CONFLICT DO NOTHING RETURNING`, nhắc học sinh được giao mà chưa nộp; đổi hạn nộp thì nhắc lại |
| Học kèm | Khung giờ theo giờ VN, chia thành các khoảng bằng thời lượng một buổi; bỏ khoảng trùng buổi học khác của giáo viên và yêu cầu đang chờ / đã nhận; đặt trước tối thiểu / tối đa theo cài đặt. Chỉ mục duy nhất `(teacher_id, starts_at)` cho yêu cầu đang chờ / đã nhận → hai người bấm cùng lúc chỉ một người được. Mỗi học sinh tối đa 3 yêu cầu chờ với một giáo viên. Nhận → buổi học mời riêng học sinh (học sinh không cần thuộc lớp học của giáo viên khi khung mở cho mọi tài khoản); link họp chỉ lộ sau khi được nhận |
| Lộ trình riêng | Bước có `audienceGroupIds` / `audienceUserIds` (rỗng = cả lớp); nhóm phải của giáo viên, học sinh phải đang học lớp. Mốc tự ẩn nếu mọi bước dưới nó không giao cho học sinh đó. Tiến độ, "Ôn tập hôm nay", báo cáo phụ huynh chỉ tính bước được giao; bảng tiến độ của giáo viên hiện "—" ở ô không giao; trang công khai không hiện bước riêng |
| Trang công khai | SPA không có nội dung khi chưa chạy JavaScript nên api dựng sẵn HTML nhẹ (không cần SSR cho cả ứng dụng); nút "Vào học" / "Xem trong ứng dụng" dẫn vào SPA. Giáo viên tắt công khai → 404 với người khác, bỏ khỏi sitemap. Hồ sơ chưa sửa lần nào dùng phần giới thiệu trong hồ sơ đăng ký giáo viên đã duyệt |

| Kiểm tra | Kết quả |
|---|---|
| Backend unit test (`sls-core`) | 69 / 69 đạt + 1 test chạy với Garage thật (`cargo test -p sls-core --lib storage -- --ignored`: upload, gửi sai `Content-Type` bị 403, HEAD, đọc vài byte, tải với `Content-Type` ép, xóa — đạt). Thêm: HMAC theo RFC 4231, chữ ký khớp ví dụ presigned của AWS, mã hóa URI, chữ ký định dạng file, tên lưu trên kho, chia khung giờ học kèm theo giờ VN, bỏ giờ bận, lộ trình riêng (mốc tự ẩn), mô tả ngắn / thoát HTML trang công khai, cấu hình `S3_*` |
| Backend integration test | 93 / 93 đạt — thêm `coursework` 5: upload + link xem (sai loại, quá lớn, chưa upload, nội dung giả ảnh bị xóa, link bài làm phải có chữ ký); bài tập (thư viện, nộp, file của người khác bị chặn, chấm + ảnh vẽ, trả lại, nộp lại, tiến độ, nhắc hạn nộp đúng người một lần, nộp muộn, giáo viên xem thử không nộp được, bài tập bị ẩn không xuất bản được); lộ trình riêng (nhóm / học sinh, mốc ẩn, bước không giao → 404, tiến độ); học kèm (chỉ học sinh của giáo viên, trùng giờ, nhận → buổi học + `.ics`, từ chối, học sinh hủy → hủy buổi, tối đa 3 yêu cầu chờ, mở cho mọi tài khoản); hồ sơ + trang công khai (JSON, HTML có Open Graph / schema.org, thoát HTML, không lộ bước riêng, sitemap, tắt công khai, lớp riêng tư 404) |
| `cargo fmt --check`, `cargo clippy -D warnings` | Đạt |
| Frontend (Vitest) | 43 / 43 đạt (thêm kiểm tra file trước khi upload, chữ thay thế ảnh, nhóm khoảng trống theo ngày, mô tả khung giờ, menu Học kèm 1-1 / Bài tập về nhà); `npm run build` đạt |
| Trình duyệt | Giáo viên: soạn bài tập, chèn ảnh (upload thật qua Garage, xem trước hiện ảnh), thêm bước bài tập có hạn nộp vào lộ trình + chọn "Dành cho", xuất bản; tab Bài tập → danh sách bài nộp → chấm: vẽ dấu tích + gạch chân lên ảnh, điểm 9,5, nhận xét có công thức; mở lịch học kèm (2 khung giờ) → nhận yêu cầu → email lời mời kèm `.ics` trong Mailpit. Học sinh: bước bài tập trong lớp học, đề bài có ảnh, nộp 2 trang ảnh + lời nhắn, email nhắc hạn nộp; xem bài đã chấm (điểm, nhận xét, ảnh có nét vẽ); học kèm: giờ thứ Ba bị ẩn vì trùng buổi học sẵn có, đặt 19:00 thứ Hai. Giáo viên mới: hồ sơ công khai (ảnh đại diện, giới thiệu) → trang trong ứng dụng + trang `/p/giao-vien/…`; trang `/p/lop-hoc/…` có lộ trình. Sửa 2 lỗi phát hiện khi thử: proxy `/files` của Vite đổi `Host` (Garage báo 403), ô chọn màu của hộp vẽ bị kéo dãn |

**Dữ liệu thử thêm trong DB dev:** bài tập "Bài tập về nhà — Hệ phương trình" (bước cuối của lộ trình "Toán 9", `hs.thu` đã nộp, đã chấm 9,5), lịch học kèm "Học kèm Toán 9 — ôn thi vào 10" của admin (T2 / T3 19:00–21:00, `hs.thu` đã được nhận buổi 19:00 thứ Hai 16/11), giáo viên `gv.lan` (cùng mật khẩu dev với `hs.thu`, có hồ sơ công khai).

## Đợt 6 — Dạy trực tiếp trong ứng dụng (G5: LiveKit, tương tác, bảng thông minh WB1, khách) (bắt đầu 2026-11-13)

> Yêu cầu: "làm tiếp các tính năng còn lại". Theo `analyst.md` mục 7, sau G2–G4 là G5: phòng live tự host (LiveKit), giơ tay → mời lên bảng, chat, bình chọn / quiz nhanh, điểm danh, bảng thông minh WB1, buổi mở cho khách có phòng chờ.

| Module | Nội dung | Trạng thái |
|---|---|---|
| **M26** | Phòng học của hệ thống (LiveKit 1.13 tự host): buổi học chọn "phòng của hệ thống" hoặc link họp ngoài; api cấp token theo vai trò (giáo viên phát mọi nguồn + quản trị phòng; học sinh chỉ xem / nghe / gửi dữ liệu; khách chỉ xem / nghe); vào phòng = điểm danh; giáo viên cho / thu quyền bật mic, camera, chia sẻ màn hình ngay lập tức, mời ra, kết thúc buổi | ✅ Xong (2026-11-13) |
| **M27** | Tương tác: giơ tay ("Em giải" / "Em hỏi") → hàng đợi theo thứ tự cho giáo viên → **mời lên bảng** (+ bật mic); chat (giáo viên tắt được); **bình chọn / quiz nhanh** (kết quả hiện ngay, quiz công bố đáp án khi đóng); khung chính đổi giữa bảng / màn hình / camera; lưu kết quả bình chọn, lượt lên bảng | ✅ Xong (2026-11-13) |
| **M28** | Bảng thông minh WB1: H-UI thêm `hui-whiteboard` (bút, bút dạ quang, tẩy, đoạn thẳng, mũi tên, hình chữ nhật, elip, chữ, màu, độ dày, hoàn tác / làm lại, nhiều trang, xuất PNG); đồng bộ trong phòng; giáo viên cấp quyền vẽ cho học sinh được mời; lưu ảnh từng trang sau buổi để học sinh xem lại | ✅ Xong (2026-11-13) |
| **M29** | Buổi mở cho khách (`public_link`, `analyst.md` 4.3): link có khóa ngẫu nhiên (đổi được), khách nhập tên → **phòng chờ** → giáo viên duyệt (từng người / tất cả) → vào phòng chỉ xem và nghe, gắn nhãn "Khách"; giới hạn số khách, giới hạn số lần xin vào theo IP; mời ra thì không vào lại được | ✅ Xong (2026-11-13) |

**Để sau:** ghi hình buổi live (LiveKit Egress → Garage → HLS, D4), WB2 (hệ trục + đồ thị từ công thức, hình học, hình khối — `hui-plot`, `hui-solid`), bảng riêng từng học sinh, PDF làm nền bảng, công thức trên bảng, đồng bộ bảng bằng CRDT (Yjs / `yrs`) thay cho kênh dữ liệu của LiveKit, captcha (ALTCHA) cho khách, trợ giảng (D12), thông báo realtime qua WebSocket, tách `hui-video-grid` / `hui-live-controls` / `hui-poll` sang H-UI (đang là component Vue trong app).

### Kết quả đợt 6 (2026-11-13)

Migration `20261113000003_live.sql` (`sessions` thêm `room` external | live, mức truy cập `public_link`, `guest_key_hash`, `guest_limit`, `live_started_at`, `live_ended_at`; bảng `live_guests`, `live_polls`, `live_board_pages`, `live_board_turns`). Mã: module `live` (`livekit.rs`: token JWT HS256 + gọi RoomService qua Twirp, không thêm crate), `sessions::enter_live` / `live_window` / `own_live`; controller `api/src/controllers/live.rs`. Frontend: `composables/useLiveRoom.js`, `components/live/{LiveRoom, VideoTile, LivePoll, LiveRecords}.vue`, `pages/live/{LiveRoomPage, GuestJoinPage}.vue`, `utils/live.js` (giao thức tin nhắn); thư viện `livekit-client` 2.23.0. Hạ tầng: LiveKit v1.13.11 trong `deploy/docker-compose.dev.yml` + `deploy/docker-compose.yml` (profile `livekit`), `deploy/livekit/{livekit.dev.yaml, livekit.yaml}`, Caddy chuyển `/rtc*` vào LiveKit (cùng domain).

| API | Mô tả |
|---|---|
| `GET /api/live/status` | Đã cấu hình LiveKit chưa |
| `POST /api/sessions/{id}/live/join` | Vào phòng → `{ url, token, room, identity, name, role, session }` (điểm danh học sinh) |
| `POST …/live/media` (`{identity, microphone, camera, screen}`) · `…/live/remove` · `…/live/end` | Giáo viên đổi quyền phát, mời ra, kết thúc (đóng phòng) |
| `POST …/live/board-turns/start\|end` · `POST …/live/polls` · `PUT\|GET …/live/board` · `GET …/live/records` | Lượt lên bảng, kết quả bình chọn, ảnh + dữ liệu bảng, xem lại sau buổi |
| `POST …/live/guest-link` · `GET …/live/guests` · `POST …/live/guests/{id}/decide` · `POST …/live/guests/decide-all` | Link khách, phòng chờ |
| `POST /api/live/guest/{sessionId}/join` (`{key, name}`) · `GET /api/live/guest/state/{guestId}?token=` | Khách (không cần đăng nhập) xin vào, hỏi trạng thái → token khi được duyệt |

**Quyết định khi làm:**

| Chủ đề | Quyết định |
|---|---|
| Token | api ký token LiveKit (HS256, sống 6 giờ) bằng `jsonwebtoken` có sẵn; vai trò nằm trong `metadata` do server ký, người tham gia không sửa được (`canUpdateOwnMetadata = false`). Danh tính `u:{id}` / `g:{id}` |
| Quyền phát | Học sinh vào phòng không phát được gì; giáo viên bấm "Cho bật mic" → api gọi `UpdateParticipant` của LiveKit (đổi quyền ngay, không cần vào lại), học sinh tự bật mic. Khách: chỉ nhận (`canPublishData = false` → không chat / giơ tay / bình chọn) |
| Đồng bộ trong phòng | Giơ tay, chat, bình chọn, thao tác bảng đi qua **kênh dữ liệu của LiveKit** (reliable) — không phải dựng WebSocket riêng. Máy nhận chỉ chấp nhận tin "của giáo viên" khi người gửi có vai trò giáo viên (LiveKit xác thực danh tính người gửi), thao tác bảng chỉ từ giáo viên + học sinh đang được mời lên bảng. Người vào sau nhận ảnh chụp bảng / bình chọn từ giáo viên qua text stream. Khác `analyst.md` (Yjs + `yrs` + WebSocket): đủ cho WB1 và quy mô 30–60 người; chuyển sang CRDT khi làm WB2 / bảng riêng từng học sinh |
| Giờ vào phòng | Học sinh / khách: từ 10 phút trước giờ học tới 30 phút sau giờ kết thúc; giáo viên mở phòng sớm 60 phút |
| Kết thúc | Giáo viên bấm "Kết thúc buổi": lưu ảnh từng trang bảng (Garage) + dữ liệu nét vẽ, đóng phòng (`DeleteRoom`) → học sinh / khách thấy "Buổi học đã kết thúc"; trang buổi học hiện "Lưu lại từ buổi học" (ảnh bảng, bình chọn / quiz, lượt lên bảng cho giáo viên) |
| Học kèm 1-1 | Lịch học kèm không có link họp → buổi học kèm tự dùng phòng của hệ thống (khi đã cấu hình LiveKit) |
| Dev trên Windows | LiveKit chạy trong Docker với `--node-ip 127.0.0.1`, mở 7880 (WebSocket), 7881/tcp, 7882/udp trên 127.0.0.1; trình duyệt kết nối `ws://127.0.0.1:7880` |

| Kiểm tra | Kết quả |
|---|---|
| Backend unit test (`sls-core`) | 71 / 71 đạt + 1 test Garage thật (thêm: token LiveKit đúng định dạng, cấu hình `LIVEKIT_*`) |
| Backend integration test | 96 / 96 đạt — thêm `live` 3 với **LiveKit giả** (server HTTP kiểm tra token quản trị + ghi lại các lần gọi RoomService): token giáo viên / học sinh đúng quyền, chưa tới giờ, người ngoài 404, điểm danh khi vào phòng, đổi quyền mic (`UpdateParticipant` đúng nguồn), lượt lên bảng, bình chọn (quiz phải có đáp án), lưu bảng, xem lại sau buổi, mời ra (`RemoveParticipant`), kết thúc (`DeleteRoom`); khách: sai khóa, phòng chờ, duyệt / từ chối / duyệt tất cả, token khách chỉ nhận, mời ra, đổi link; chưa cấu hình LiveKit |
| `cargo fmt --check`, `cargo clippy -D warnings` | Đạt |
| Frontend (Vitest) | 48 / 48 đạt (thêm giao thức tin nhắn, vai trò từ metadata, hàng đợi giơ tay, đếm phiếu, thời gian, tên ngắn); `npm run build` đạt |
| H-UI | 168 unit test (thêm 5 cho `hui-whiteboard`) + 3 story test đạt; `npm run build`, `check-cdn`, `check:types` đạt. `build:cdn` báo lỗi "module `hui-calendar/date-utils.js` bị gộp vào 2 nhóm (form, data)" — **có sẵn từ trước** (chạy trên bản H-UI chưa sửa cũng lỗi y hệt), không liên quan `hui-whiteboard` |
| Trình duyệt (LiveKit thật) | Giáo viên mở phòng; học sinh (tab khác) vào → 2 người; giáo viên vẽ bút / hình chữ nhật / chữ → học sinh thấy ngay; học sinh giơ tay → giáo viên thấy hàng đợi → "Mời lên bảng" → học sinh vẽ (màu xanh) → giáo viên thấy; "Cho bật mic" → LiveKit đổi quyền, học sinh có nút "Bật mic" (pane của ứng dụng chặn thiết bị nên không thu âm thật); quiz 3 phương án có công thức → học sinh chọn → "Chính xác!"; link khách → khách nhập tên → phòng chờ → giáo viên "Cho vào" → khách thấy bảng (chỉ xem); "Kết thúc buổi" → trang buổi học hiện ảnh bảng + kết quả quiz. Sửa khi thử: chữ trên bảng bị ghi 2 lần (H-UI, thêm test), nút mic / camera của giáo viên không hiện (computed của Vue), thanh điều khiển bị kéo cao, học sinh / khách chỉ thấy "Mất kết nối" khi buổi kết thúc (giờ hiện đúng lý do) |

**Chưa thử được trong pane của ứng dụng:** camera / mic / chia sẻ màn hình thật (pane chặn thiết bị) — phần phát / nhận video dùng thẳng API của `livekit-client`, cần thử lại trên trình duyệt thường với 2 máy.

**Dữ liệu thử thêm trong DB dev:** buổi "Chữa bài hệ phương trình (trực tuyến)" (phòng của hệ thống, mở cho khách) của lớp "Toán 9", có ảnh bảng + 1 quiz đã lưu.

## Đợt 7 — Đồ thị hàm số, nhập đề Word, trợ giảng, PWA (bắt đầu 2026-11-18)

> Yêu cầu: "làm tiếp tính năng" (sau khi dọn branch / worktree). Lấy từ danh sách "Để sau" của đợt 5–6: WB2 phần đồ thị (`hui-plot`), import Word, trợ giảng (D12), PWA (D10).

| Module | Nội dung | Trạng thái |
|---|---|---|
| **M30** | H-UI thêm `hui-plot`: đồ thị hàm số từ công thức gõ tự nhiên (nhân ngầm `2x`, dấu phẩy thập phân, `^ ² √ \|x\|`, sin / cos / tan / cot, ln, log…), nhiều đồ thị một hệ trục, miền nghiệm `y > …`, đường `x = c`, điểm `A(1; 2)`, tham số tự sinh thanh trượt, khung trục `x: -2 .. 5`; hai trục cùng đơn vị (tự tính trục y theo khung); kéo / phóng to. Tự phân tích biểu thức, **không dùng eval**. `hui-markdown`: khối ` ```plot ` → `<hui-plot>`. `hui-whiteboard`: công cụ **ƒ** (kéo khung → gõ công thức → Ctrl + Enter; chạm lại để sửa), thao tác mới `update`. App: nút "Chèn đồ thị" ở ô soạn Markdown | ✅ Xong (2026-11-18) |
| **M31** | Nhập câu hỏi từ **Word (.docx)** soạn theo cách giáo viên vẫn gõ: "Câu 1 (NB)." (cả số câu đánh tự động của Word), phương án A. B. C. D. (chung một dòng cũng được), đúng sai a) b) c) d), đáp án = gạch chân / tô đỏ / tô nền / `*B.` / dòng "Đáp án: …" / "Chọn B" trong lời giải, "Lời giải:", tiêu đề "PHẦN I / II / III" để nhận loại câu; công thức Equation (OMML) → LaTeX; ảnh trong đề → ảnh nội dung trên Garage (trùng nội dung thì dùng lại); MathType / WMF chưa đọc được → cảnh báo rõ. Dùng chung bảng xem trước + lưu hàng loạt với Excel; file Word mẫu tải về được | ✅ Xong (2026-11-18) |
| **M32** | **Trợ giảng** (D12): chủ lớp mời tài khoản giáo viên (username / email, tối đa 5) làm trợ giảng. Trợ giảng: xem lớp, học sinh, duyệt / từ chối ghi danh, tiến độ, báo cáo, chấm bài tập, trả lời / ghim / ẩn hỏi đáp (hiện nhãn giáo viên, nhận thông báo câu hỏi mới + bài nộp), vào phòng học trực tuyến của lớp với quyền điều khiển như giáo viên. Không được: sửa lớp / lộ trình, đổi mã mời, thêm / mời ra học sinh, lưu trữ / sao chép lớp, kết thúc buổi live, tạo link khách | ✅ Xong (2026-11-18) |
| **M33** | **PWA** (D10): `manifest.json` + icon (thường, maskable, Apple), service worker tự viết: mất mạng → trang "Bạn đang ngoại tuyến" (tự mở lại khi có mạng); nút "Cài ứng dụng" ở thanh trên khi trình duyệt cho cài | ✅ Xong (2026-11-18) |

### Kết quả đợt 7 (2026-11-18)

Migration `20261118000001_word_import.sql` (`files.purpose` thêm `import`, cột `sha256` + index để dùng lại ảnh trùng nội dung). Bảng `class_members` đã có sẵn vai `assistant` từ M8 — không cần đổi schema cho trợ giảng.

| Phần | Mã |
|---|---|
| H-UI | `src/components/hui-plot/{expr.js, plot-core.js, hui-plot.js, hui-plot.css, hui-plot.stories.js}`; `hui-markdown/markdown.js` (khối `plot`); `hui-whiteboard` (công cụ `plot`, `addPlot()`, thao tác `update`); nhóm CDN `learning`; changeset `plot.md` |
| Backend | `questions/word.rs` (đọc .docx: cây XML bằng `quick-xml`, đánh số tự động, OMML → LaTeX, nhận dạng câu / phương án / đáp án; ghi file .docx mẫu + dữ liệu test), `questions/service.rs` (`import_word_preview`, lưu ảnh); `files` (`Purpose::Import`, `save_bytes` chống trùng SHA-256, `take_import` đọc rồi xóa); `storage.rs` (`put` ký SHA-256 nội dung, `get` có giới hạn dung lượng); `classes/assistants.rs`, `classes::service::{load_staff, StaffRole, is_assistant}`; `sessions::{LiveRole, live_staff}`; `live` cho trợ giảng điều khiển phòng |
| Frontend | `ImportQuestionsModal.vue` (Excel / Word), `MarkdownEditor.vue` (Chèn đồ thị), `ClassesPage.vue` (tab Trợ giảng), `ClassManagePage.vue` (khu Trợ giảng, ẩn việc của chủ lớp), `LiveRoom.vue` (vai `assistant`), `utils/pwa.js`, `public/{manifest.json, sw.js, offline.html, icons/}` |

| API | Mô tả |
|---|---|
| `GET /api/teaching/questions/import/word-template` | File Word mẫu (4 câu đủ 4 loại, công thức Word, gạch chân / tô đỏ đáp án) |
| `POST /api/files` (`purpose: "import"`) → `PUT` lên Garage → `POST /api/teaching/questions/import/word/preview` (`{subjectId, fileId}`) | File .docx tới 20 MB đi thẳng lên Garage (không qua giới hạn 1 MB của API); api đọc một lần rồi xóa, trả bảng xem trước như Excel (thêm `label` "II · Câu 3") |
| `GET\|POST /api/teaching/classes/{id}/assistants` · `DELETE …/assistants/{userId}` | Danh sách / mời / bỏ trợ giảng (trợ giảng tự rời lớp được) |
| `GET /api/teaching/classes?assisting=true` | Lớp mình làm trợ giảng (`myRole`, `ownerName`, không có mã mời) |

**Quyết định khi làm:**

| Chủ đề | Quyết định |
|---|---|
| Đồ thị không dùng thư viện | Tự viết bộ phân tích biểu thức (Pratt parser, ~250 dòng) thay vì `eval` / math.js: an toàn với nội dung học sinh nhập, hiểu cách viết của Việt Nam (dấu phẩy thập phân, `tg`, `cotg`, nhân ngầm). Lõi vẽ `drawPlot` dùng chung cho `hui-plot` và bảng vẽ. Phương trình ẩn (`x² + y² = 4`) chưa hỗ trợ — báo lỗi rõ ở dòng đó |
| Đồ thị trên bảng | Lưu dạng phần tử `{ type: 'plot', khung, value }` (công thức, không phải ảnh) → đồng bộ nhẹ, sửa được, vẽ lại sắc nét ở mọi cỡ màn hình; đi qua kênh dữ liệu cũ của phòng live, không đổi giao thức |
| Word: nhận dạng thay vì bắt theo mẫu cứng | Giáo viên đã có sẵn ngân hàng đề Word; quy tắc nhận dạng (số câu, nhãn phương án liên tiếp từ đầu dòng, đáp án gạch chân / tô đỏ, nhãn "Đáp án" / "Lời giải" / "PHẦN") khớp cách soạn phổ biến. Nhãn phương án phải liên tiếp A → B → C và bắt đầu đầu dòng nên "Cho điểm A. …" trong đề không bị cắt nhầm |
| Word: file lớn | .docx thường có ảnh nên tới hàng chục MB → upload thẳng Garage bằng presigned URL (mục đích `import`, tối đa 20 MB, kiểm tra chữ ký zip), không nới giới hạn body 1 MB của API. Caddy `/files` nâng 20MB → 25MB (20 MiB của api lớn hơn 20 MB của Caddy) |
| Word: MathType | Công thức MathType là đối tượng OLE + ảnh WMF — không chuyển được sang LaTeX trên server mà không có MathType. Để chỗ trống `[công thức MathType]` + cảnh báo hướng dẫn "MathType → Convert Equations" sang Equation của Word |
| Trợ giảng | Dùng vai `assistant` sẵn có trong `class_members` (một người một vai trong lớp). Mọi chỗ "chủ lớp" chia làm hai: `load_own` (sửa) và `load_staff` (xem / chấm / hỏi đáp). Lớp đã lưu trữ: chỉ chủ lớp vào được. Phòng live: trợ giảng nhận vai `assistant` trong metadata do server ký, máy học sinh tin lệnh của trợ giảng như giáo viên; trợ giảng không đổi quyền / mời ra giáo viên chủ buổi được |
| PWA | Không dùng Workbox / `vite-plugin-pwa`: service worker chỉ lo trang ngoại tuyến + icon, **không** lưu mã ứng dụng hay dữ liệu API (dữ liệu riêng tư, link file có chữ ký) → không có lỗi "kẹt bản cũ", cập nhật ứng dụng như web thường (Caddy đã cache lâu `/assets/*` có hash). Chỉ đăng ký ở bản build |

| Kiểm tra | Kết quả |
|---|---|
| Backend unit test (`sls-core`) | 76 / 76 đạt (thêm `word`: file mẫu đọc lại đủ 4 câu, OMML → LaTeX (mũ, căn bậc n, hệ phương trình, ngoặc, hàm, giới hạn, tổng, vector, góc, chữ tiếng Việt), nhiều kiểu đánh dấu đáp án, phương án chung dòng tiêu đề, "Chọn B" trong lời giải, chỉ số trên gõ bằng định dạng, đánh số tự động, ảnh PNG / WMF, MathType, file hỏng; Excel bỏ nhãn "a)" của ý đúng sai; loại file `import`) + test Garage thật có thêm `put` / `get` |
| Backend integration test | 99 / 99 đạt — thêm 3: nhập Word (file mẫu qua kho, file tạm bị xóa, ảnh lưu thành ảnh nội dung + dùng lại khi xem trước lần hai, câu lỗi không lưu ảnh, file người khác 404, `.doc` cũ bị từ chối); trợ giảng (chỉ giáo viên, chỉ chủ lớp mời, danh sách lớp trợ giảng, được / không được làm gì, thông báo, hỏi đáp nhãn giáo viên, tự rời lớp, lớp lưu trữ); phòng live (token vai `assistant`, buổi hiện trong lịch, cho học sinh nói, mời lên bảng, không đụng giáo viên chủ buổi, không kết thúc buổi / tạo link khách) |
| `cargo fmt --check`, `cargo clippy -D warnings` | Đạt |
| Frontend (Vitest) | 50 / 50 đạt (thêm PWA: sự kiện cài đặt, manifest + icon có thật; loại file Word; vai trợ giảng trong phòng live); `npm run build` đạt |
| H-UI | 176 unit test (thêm 6 `hui-plot`, 1 công cụ đồ thị của bảng, 1 khối `plot` trong Markdown) đạt; `npm run build`, `check:types` đạt. **Sửa lỗi CI đỏ từ đợt 3** (H-UI `2a6a99b`): `build:cdn` báo `hui-calendar/date-utils.js` nằm ở 2 nhóm (form, data) — module chỉ có hàm thuần được "đẩy vào core" bằng `import 'x'` nên bị tree-shake mất; giờ core lộ luôn module đó ở `HUI.__m`. Chạy đủ các bước CI ở máy: build, types, CDN, trang mẫu, 220 file / 1602 test (gồm story), kích thước — đạt |
| Trình duyệt | **Nhập Word:** tab "Word (đề soạn sẵn)" → file mẫu upload thật lên Garage → xem trước 4 câu hợp lệ, công thức hiện đúng. **Đồ thị:** "Chèn đồ thị" → xem trước có `hui-plot` (hai đồ thị + điểm A); bảng của phòng live (LiveKit thật): công cụ ƒ → kéo khung → gõ 3 dòng → Ctrl + Enter → parabol, đường thẳng, điểm I(1; −2), hai trục cùng đơn vị. **Trợ giảng:** admin mời `gv.lan` qua giao diện → `gv.lan` thấy lớp ở tab "Trợ giảng", trang lớp không có tab Lộ trình / mời học sinh / "Mời ra", các tab Tiến độ, Bài tập, Báo cáo, Hỏi đáp tải đúng. **PWA** (bản build, `vite preview`): service worker kích hoạt, lưu sẵn trang ngoại tuyến; tắt server rồi mở `/lich-hoc` → trang "Bạn đang ngoại tuyến". Sửa khi thử: tọa độ điểm hiện "1,00; 3,00" → "1; 3", trục x / y không cùng đơn vị (y = x không nghiêng 45°) → tự tính trục y theo khung, ô công thức trên bảng chưa nhận phím khi pane ẩn (focus ngay, không chỉ chờ `requestAnimationFrame`) |

**Chưa thử được:** hộp thoại "Cài ứng dụng" thật (pane của ứng dụng không phát `beforeinstallprompt`; đã test bằng sự kiện giả) — thử trên Chrome / Edge qua HTTPS. File Word thật của giáo viên (nhiều kiểu soạn khác nhau) — nên thử với vài đề thật, gặp kiểu chưa nhận được thì bổ sung quy tắc trong `word.rs`.

**Dữ liệu thử thêm trong DB dev:** `gv.lan` là trợ giảng của lớp "Toán 9 — ôn thi vào 10 (tự học)" (của admin).

**Để sau:** phương trình ẩn / đường tròn trên `hui-plot`, hình học (`hui-geometry`) và hình khối (`hui-solid`) của WB2; nhập đề từ PDF; xem lại bài học ngoại tuyến (lưu nội dung bài vào bộ nhớ máy); thông báo đẩy (Web Push) cho PWA; nhắc lịch buổi học cho trợ giảng qua email.

## Đợt 8 — Giáo án & bài giảng (bắt đầu 2026-11-19)

> Yêu cầu: phân tích giáo viên dạy thêm (ưu tiên Toán) còn thiếu gì → phân tích + lộ trình đợt 8–12 ở `analyst.md` mục 9.
> Đã chốt: làm **Giáo án & bài giảng** trước; chưa làm học phí; chưa dùng AI API (trắc nghiệm chấm bằng thuật toán,
> tự luận có lời giải mẫu, giáo viên tự chấm); liên lạc: Web Push trước, Zalo sau.

| Module | Nội dung | Trạng thái |
|---|---|---|
| **M34** | **Giáo án theo mẫu**: CV 5512 (mục tiêu → thiết bị → 4 hoạt động Mở đầu / Hình thành kiến thức / Luyện tập / Vận dụng, mỗi hoạt động có a) Mục tiêu b) Nội dung c) Sản phẩm d) Tổ chức thực hiện 4 bước), CV 2345 (yêu cầu cần đạt, đồ dùng, bảng hai cột GV / HS), dạy thêm (kiến thức cần nhớ, các dạng bài: phương pháp — ví dụ mẫu — bài tập, bài về nhà), trống. Khối: mục, hoạt động, hai cột, dạng bài, câu hỏi từ ngân hàng (có / không kèm đáp án). Thông tin chung (trường, tổ, giáo viên, số tiết, ngày soạn / dạy, lớp) tự điền từ giáo án gần nhất; liên kết bài học; sao chép ("dùng lại giáo án năm trước"); lưu trữ; tự lưu sau 2 giây | ✅ Xong (2026-11-19) |
| **M35** | **Xuất Word + trang in**: file `.docx` tạo ngay trên trình duyệt (không thư viện): bảng thông tin đầu trang, tên bài, các khối, câu hỏi; đáp án + lời giải ở phụ lục cuối; công thức → Equation của Word (OMML); đồ thị → ảnh PNG; ảnh trong bài nhúng vào file. A4, lề trái 3 cm / còn lại 2 cm, Times New Roman 13, số trang ở chân trang. Trang in A4 riêng (`/day-hoc/giao-an/:id/in`) để "In → Lưu PDF" | ✅ Xong (2026-11-19) |
| **M36** | **Trình chiếu**: H-UI thêm `hui-slides` (khung 1280 × 720 tự co, ← → / Space / PageUp / PageDown / Home / End, vuốt, F toàn màn hình, G xem lưới, thanh tiến độ + số trang). Bài học: dòng `---` đứng riêng là chỗ sang slide mới (không có thì mỗi tiêu đề `#` / `##` là một slide). Trang trình chiếu bản nháp cho giáo viên; học sinh mở bài đã gắn vào buổi học (đọc / trình chiếu). Phòng live: chế độ **📑 Slide** — giáo viên chọn bài của buổi, học sinh / khách xem theo trang của giáo viên, người vào sau nhận qua snapshot | ✅ Xong (2026-11-19) |
| **M37** | **Tài liệu đính kèm**: PDF / Word / Excel / PowerPoint tới 20 MB, tối đa 10 file cho bài học và giáo án; tài liệu đi theo lần xuất bản (học sinh thấy tài liệu của bản đã xuất bản); link có chữ ký; PDF mở trong tab mới, file khác tải về | ✅ Xong (2026-11-19) |

### Kết quả đợt 8 (2026-11-19)

Migration `20261119000001_lesson_plans_attachments.sql`: bảng `lesson_plans` (`template` cv5512 / cv2345 / day_them /
blank, `info` + `blocks` + `attachments` jsonb, `lesson_id` → SET NULL khi xóa bài, `copied_from`, `archived_at`);
`files.purpose` thêm `attachment`; cột `attachments` jsonb (`[{fileId, name}]`) cho `lessons` và `lesson_revisions`.

| Phần | Mã |
|---|---|
| H-UI | `src/components/hui-slides/{hui-slides.js, hui-slides.css, hui-slides.stories.js, index.js}`, `test/slides.test.js`, nhóm CDN `learning`, export ở `src/index.js` + `package.json`, changeset `slides.md` |
| Backend | Module mới `lesson_plans` (khối serde `tag = "kind"`, giới hạn 80 khối / 20.000 ký tự mỗi trường / 60 dòng bảng / 100 câu hỏi, câu hỏi và bài liên kết phải của chính giáo viên); `files` (`Purpose::Attachment`, kiểm tra `%PDF` / `PK`, `check_attachments`, `attachment_views`); `lessons` (tài liệu ở bản nháp + chép vào bản xuất bản); `sessions::session_lesson` |
| Frontend | `utils/lessonPlan.js` (mẫu, khối, payload), `utils/lessonPlanRender.js` (giáo án → DOM dùng chung cho trang in và Word), `utils/docx/{zip, omml, fromDom, document}.js`, `utils/slides.js`; `components/teaching/{LessonSlides, AttachmentList}.vue`; trang `teaching/{LessonPlansPage, LessonPlanEditorPage, LessonPlanPrintPage, LessonPresentPage}.vue`, `learning/SessionLessonPage.vue`; sửa `LessonEditorPage` (nút Trình chiếu, gợi ý `---`, tài liệu), `LessonViewPage` + `SessionPage` (tài liệu, link bài của buổi), `LiveRoom.vue` (chế độ Slide), menu "Giáo án" |

| API | Mô tả |
|---|---|
| `GET\|POST /api/teaching/lesson-plans` | Danh sách giáo án của mình (lọc `q` không dấu, `subjectId`, `gradeId`; có số khối) / tạo |
| `GET\|PUT\|DELETE /api/teaching/lesson-plans/{id}` | Xem (kèm tên bài liên kết, tài liệu) / sửa / lưu trữ — giáo viên khác nhận 404 |
| `POST /api/teaching/lesson-plans/{id}/copy` | Sao chép thành giáo án mới "(bản sao)", ghi `copiedFrom` |
| `POST /api/files` (`purpose: "attachment"`) | Ký link upload tài liệu; bài học / giáo án gửi `attachmentIds` |
| `GET /api/sessions/{id}/lessons/{lessonId}` | Bản đã xuất bản (kèm tài liệu) của bài gắn vào buổi — chỉ người xem được buổi; bài không thuộc buổi / chưa xuất bản → 404 |

**Quyết định khi làm:**

| Chủ đề | Quyết định |
|---|---|
| Giáo án tách khỏi bài học | Giáo án là hồ sơ nộp tổ chuyên môn (thông tin hành chính, cấu trúc theo công văn), bài học là nội dung học sinh xem → bảng riêng, liên kết tới bài học khi cần. Mẫu là khung khối ban đầu viết ở frontend → thêm / sửa mẫu không cần migration |
| Khối câu hỏi | Lưu `questionIds`; trang soạn / in / Word tải từng câu qua API câu hỏi có sẵn (có cache) → không cần API "xem trước câu hỏi" riêng như dự kiến |
| Word ở trình duyệt | Dùng đúng bộ vẽ Markdown + MathML của H-UI → Word giống màn hình, backend không phải viết lại bộ đọc Markdown / LaTeX. Zip kiểu STORE (không nén) — file lớn hơn một chút nhưng không cần thư viện nén; danh sách dùng ký tự đầu dòng (chưa dùng numbering của Word). Bộ xuất dùng lại cho mã đề ở đợt 9 |
| Slide từ bài học | Không tạo loại nội dung mới: một bài vừa đọc được vừa chiếu được, tách ở `---` (hoặc tiêu đề). Slide dài hơn khung thì cuộn trong slide thay vì bị cắt |
| Slide trong phòng live | Nội dung bài gửi một lần qua text stream (có thể dài, chứa công thức / đồ thị), mỗi lần chuyển trang chỉ gửi `{t: 'slide', index}` qua kênh dữ liệu; học sinh chỉ nhận lệnh từ giáo viên / trợ giảng; snapshot cho người vào sau có thêm `slides` |
| Tài liệu theo bản xuất bản | Sửa tài liệu ở bản nháp không làm học sinh mất / thấy trước tài liệu; xuất bản mới chép sang. Link có chữ ký (như bài làm) vì tài liệu chỉ cho người được xem bài |

| Kiểm tra | Kết quả |
|---|---|
| Backend unit test (`sls-core`) | 78 / 78 đạt (1 bỏ qua: test Garage thật) — thêm 2 cho giáo án: khối đọc đúng từ JSON của giao diện (cả 5 loại), giới hạn khối / độ dài |
| Backend integration test | 101 / 101 đạt — thêm 2: giáo án (tạo theo mẫu, sửa, tìm không dấu, sao chép kèm tài liệu, lưu trữ; giáo viên khác 404; câu hỏi / tài liệu của người khác, mẫu lạ, quá nhiều khối bị từ chối; link tài liệu có chữ ký); bài của buổi + tài liệu (upload qua kho trong bộ nhớ; bài chưa xuất bản 404; người không được mời 404; bài không gắn vào buổi 404; thêm tài liệu ở bản nháp thì học sinh vẫn thấy tài liệu của bản đã xuất bản, xuất bản lại mới thấy; ảnh nội dung không dùng làm tài liệu được) |
| `cargo fmt --check`, `cargo clippy -D warnings` | Đạt (chạy `cargo fmt` cho file mới); `cargo test` chạy với `-j 2` vì máy dev hết bộ nhớ ảo khi biên dịch song song nhiều file test |
| Frontend (Vitest) | 60 / 60 đạt (thêm: mẫu giáo án tạo đúng khối, payload; tách slide ở `---` / tiêu đề, bỏ qua `---` trong khối code; zip đọc ngược + CRC32, MathML → OMML (phân số, căn, mũ / chỉ số, hệ phương trình, ma trận), DOM → Word (tiêu đề, bảng, công thức, ảnh); loại file tài liệu); `npm run build` đạt |
| H-UI | `hui-slides`: 5 unit test + 3 story test. Chạy đủ các bước CI ở máy: build, types, CDN, trang mẫu, 222 file / 1609 test (gồm story), kích thước — đạt |
| Trình duyệt | **Giáo án:** tạo theo mẫu CV 5512 → soạn có công thức, đồ thị, khối câu hỏi → "Đã lưu lúc …"; trang in hiện đúng khổ A4; file Word tạo trong trình duyệt có đủ 8 phần, 1 ảnh (đồ thị), 2 công thức OMML, XML hợp lệ. **Trình chiếu:** bài 4 slide (tiêu đề, công thức, đồ thị, luyện tập) → phím → / End / Home chuyển đúng. **Tài liệu:** chọn file ở trang soạn bài → .docx được thêm, .exe bị từ chối kèm thông báo; học sinh mở bài của buổi → thấy "Phiếu học tập số 1.pdf", link có chữ ký trả `application/pdf`. **Phòng live (LiveKit thật):** giáo viên chọn "📑 Slide" → chọn bài; khách vào sau (link khách, đã duyệt) nhận đủ 4 slide, không có nút chuyển; giáo viên sang slide 3 → tab khách sang slide 3, khách bấm phím không đổi trang. Sửa khi thử: `hui-slides` đặt trong grid / flex làm trang tràn ngang (khung 1280px đòi bề rộng tối thiểu + viền 2px) → khung đặt `position: absolute`, `box-sizing: border-box`, thêm test |

**Chưa thử được:** nút "Toàn màn hình" (pane của ứng dụng chặn `requestFullscreen`: "Permissions check failed") — thử
trên Chrome / Edge thường. Mở file Word xuất ra bằng Microsoft Word / LibreOffice thật (máy có Word, cần bạn đồng ý mới
mở ứng dụng của bạn) — đã kiểm tra cấu trúc file trong trình duyệt. Tài khoản học sinh thật trong phòng live (đã thử
bằng khách; học sinh và khách dùng chung đường nhận slide).

**Dữ liệu thử thêm trong DB dev:** `gv.lan` có 1 giáo án mẫu CV 5512 và bài "Bài 3. Phương trình bậc hai (trình chiếu)"
(đã xuất bản, 4 slide, 1 PDF đính kèm). Lớp "Lớp thử trình chiếu (đợt 8)" đã lưu trữ, buổi thử đã hủy.

**Để sau:** đánh số danh sách kiểu Word (numbering) trong file xuất; xuất bài học / phiếu bài tập ra Word (bộ xuất đã
có, chỉ cần nút); ghi chú của giáo viên cho từng slide (presenter notes); đồ thị trong Markdown tự chọn khung theo đồ thị
(hiện khung mặc định có thể cắt mất đỉnh parabol — viết `x: -3 .. 5` / khung trong khối `plot` để chỉnh).

## Đợt 9 — Đề & chấm, không dùng AI (bắt đầu 2026-11-19)

> Theo lộ trình `analyst.md` mục 9.3. Trắc nghiệm / đúng sai / trả lời ngắn chấm bằng thuật toán; tự luận có hướng dẫn
> chấm + lời giải mẫu, giáo viên chấm trên ảnh bài làm. **Câu chùm** và **câu hỏi có tham số** chuyển sang đợt sau: ít
> gặp ở môn Toán (ưu tiên hiện tại), cần đổi cách rút / trộn câu theo khối nên tách riêng.

| Module | Nội dung | Trạng thái |
|---|---|---|
| **M38** | **Câu tự luận + hướng dẫn chấm theo bước**: dạng câu `essay`, các bước chấm (nội dung + điểm, kiểu "Tính Δ … 0,25 đ"), lời giải mẫu. Điểm riêng cho từng câu cố định trong đề (mặc định câu tự luận = tổng điểm các bước). **Trả lời ngắn nâng cao**: sai số cho phép (`|x − đáp án| ≤ sai số`); so **biểu thức tương đương** (`2(x+1)` = `2x + 2`, `x^2-1` = `(x-1)(x+1)`) bằng bộ đọc biểu thức viết bằng Rust ở server, thử ở nhiều giá trị của biến | ✅ Xong (2026-11-20) |
| **M39** | **Chấm tự luận bằng ảnh**: học sinh gõ bài hoặc chụp / chọn ảnh bài làm cho từng câu tự luận; nộp xong bài ở trạng thái "chờ chấm". Giáo viên (và trợ giảng của lớp) mở trang chấm: ảnh bài làm (vẽ nhận xét lên ảnh như bài tập), lời giải mẫu + từng bước chấm bên cạnh, chọn điểm từng bước, nhận xét; lưu rồi sang bài kế. **Nhận xét mẫu** dùng chung cho chấm đề và chấm bài tập. Chấm xong → điểm cập nhật, học sinh nhận thông báo, xem điểm từng bước + ảnh đã vẽ | ✅ Xong (2026-11-20) |
| **M40** | **Trộn mã đề + xuất Word**: tạo N mã đề (101, 102, …) từ một đề — mỗi mã rút câu theo ma trận, trộn câu trong phần và trộn phương án; lưu lại để in / chấm sau (phiếu tô ở đợt 12). Xuất Word tất cả mã đề (tiêu đề kiểu THPT, "Họ, tên thí sinh … Số báo danh …", PHẦN I / II / III, phương án xếp 4 / 2 / 1 cột theo độ dài), bảng đáp án (câu × mã đề), trang in | ✅ Xong (2026-11-20) |
| **M41** | **Bộ đề + giao nhanh**: gom đề thành bộ ("20 đề ôn thi vào 10"); **giao nhanh** đề / bài tập cho lớp, nhóm hoặc từng học sinh với hạn làm — thêm thẳng vào lộ trình đã xuất bản (không xuất bản kèm bản nháp đang sửa dở), học sinh nhận thông báo. Bước đề có hạn làm: quá hạn không bắt đầu làm được | ✅ Xong (2026-11-20) |

### Thiết kế đợt 9

Migration `20261120000001_essay_papers_sets.sql`:
- `questions.kind` thêm `essay`. Nội dung phiên bản (`question_versions.body`) thêm `rubric: [{id, content, points}]`,
  `tolerance`, `matchMode` (`auto` | `expression`) — bản cũ đọc ra giá trị mặc định, không cần đổi dữ liệu.
- `exam_section_questions.points` (NULL = điểm mỗi câu của phần).
- `attempt_items`: `manual` (câu chấm tay), `marks` (điểm từng bước), `feedback`, `annotations` (`[{fileId, annotatedFileId}]`),
  `graded_by`, `graded_at`. Câu trả lời (`answer`) thêm `files` (ảnh / PDF bài làm, mục đích `homework`).
- `comment_snippets` (nhận xét mẫu của giáo viên), `exam_papers` (mã đề: thứ tự câu + phương án đã trộn của từng mã),
  `exam_sets` + `exam_set_items` (bộ đề).

| Chủ đề | Quyết định |
|---|---|
| Điểm câu tự luận | Điểm câu trong đề = điểm riêng của câu (nếu đặt) / tổng điểm các bước chấm (câu tự luận cố định) / điểm mỗi câu của phần. Điểm chấm = điểm câu × (tổng điểm các bước đạt / tổng điểm các bước); câu không có bước chấm thì giáo viên nhập thẳng điểm |
| Biểu thức tương đương | Chấm ở server (học sinh không sửa được kết quả). Bộ đọc biểu thức riêng bằng Rust (giống cách viết của `hui-plot`: nhân ngầm `2x`, dấu phẩy thập phân, `^`, `√`, `sqrt`, `sin`…, `π`), so hai biểu thức ở 8 bộ giá trị ngẫu nhiên của các biến; có dấu `=` thì so `vế trái − vế phải` (cho phép đổi dấu) |
| Bài chờ chấm | Nộp bài → câu trắc nghiệm chấm ngay, câu tự luận 0 điểm "chờ chấm"; tổng điểm cập nhật mỗi lần giáo viên lưu một câu. Học sinh thấy "Đang chờ giáo viên chấm n câu" |
| Mã đề lưu lại | Mỗi mã lưu thứ tự câu, phiên bản câu và thứ tự phương án → in lại / xuất lại vẫn đúng, đợt 12 chấm phiếu tô theo đúng mã. Tạo lại bộ mã đề thì thay bộ cũ |
| Giao nhanh | Không gọi "xuất bản lộ trình" (sẽ xuất bản luôn phần giáo viên đang sửa dở): thêm bước vào bản nháp **và** tạo phiên bản xuất bản mới = bản đã xuất bản gần nhất + bước mới |
| Hạn làm của bước đề | Quá hạn không bắt đầu lượt mới (`STEP_OVERDUE`); lượt đang làm hết giờ đúng hạn (giờ nộp = sớm nhất của thời gian làm bài / giờ đóng đề / hạn làm). Bài tập giữ cách cũ: nộp muộn vẫn nhận, đánh dấu muộn |
| Nhập Word / Excel | Câu tự luận nhập được (phần "TỰ LUẬN" của đề Word, cột loại câu "Tự luận" trong Excel): nội dung + lời giải; các ý a) b) … giữ trong nội dung câu. Hướng dẫn chấm theo bước thêm sau ở trang sửa câu |

### Kết quả đợt 9 (2026-11-20)

| Phần | Mã |
|---|---|
| Backend | `questions/expr.rs` (bộ đọc biểu thức + so tương đương, không `eval`), `questions` (dạng `essay`, `rubric` / `tolerance` / `matchMode`, `grading::{matches_with, manual_score}`, nhập Word / Excel câu tự luận), `exams` (điểm riêng từng câu, `plan_items` dùng chung cho lượt làm và mã đề, câu chấm tay, `grade_item`, `grading_queue`, `generate_papers` / `papers`, `sets.rs` bộ đề, hạn làm trong `AttemptContext`), module mới `snippets` (nhận xét mẫu), `classes::path::quick_assign`, thông báo `attempt_graded` / `work_assigned` |
| Frontend | `QuestionEditorModal` (bước chấm, sai số, cách so đáp án), `QuestionView` (gõ bài / chụp ảnh câu tự luận, điểm từng bước + nhận xét), `AttemptPage` (chờ chấm, nút chấm), `ExamEditorPage` (điểm riêng từng câu, chưa chấm, Trộn mã / Giao nhanh), trang mới `GradingQueuePage`, `GradeAttemptPage`, `ExamPapersPage`, `ExamSetsPage`; component `CommentSnippets`, `QuickAssignModal`; `utils/examPaperRender.js`; `docx/fromDom.js` (ngắt trang, bảng không viền, đoạn căn giữa); `PathEditor` (hạn làm cho bước đề) |

| API | Mô tả |
|---|---|
| `GET /api/teaching/grading` | Bài đã nộp còn câu tự luận chưa chấm (đề của mình + lớp làm trợ giảng), bài nộp trước lên trước |
| `PUT /api/teaching/attempts/{id}/items/{position}/grade` | Chấm một câu tự luận: `{ marks: {stepId: điểm}, score?, feedback, annotations: [{fileId, annotatedFileId}] }` — giáo viên chủ đề hoặc trợ giảng của lớp; chấm xong câu cuối thì báo học sinh |
| `GET\|POST /api/teaching/comment-snippets` · `DELETE …/{id}` | Nhận xét mẫu (tối đa 100 / giáo viên) |
| `GET\|POST /api/teaching/exams/{id}/papers` | Xem / tạo lại bộ mã đề `{ count: 1–24, firstCode: 101 }` (kèm nội dung câu + đáp án để in / xuất Word) |
| `GET\|POST /api/teaching/exam-sets` · `GET\|PUT\|DELETE …/{id}` | Bộ đề (danh sách đề có thứ tự, cùng môn) |
| `POST /api/teaching/classes/{id}/quick-assign` | Giao nhanh `{ kind: exam\|homework, examId\|assignmentId, title, dueAt, groupIds, userIds }` → `{ stepId, version, assigned }` |
| `PUT /api/attempts/{id}/items/{position}` | (đổi) câu tự luận nhận `{ text, files: [fileId] }` — ảnh / PDF mục đích `homework` của chính học sinh, tối đa 10 |

| Kiểm tra | Kết quả |
|---|---|
| Backend unit test (`sls-core`) | 85 / 85 đạt (1 bỏ qua: test Garage thật) — thêm 7: biểu thức tương đương (đa thức, căn, trị tuyệt đối, lượng giác, phương trình tỉ lệ, khác tập xác định, thứ tự phép tính, chuỗi lồng quá sâu), sai số + biểu thức trong chấm trả lời ngắn, điểm tự luận theo bước (quy đổi thang, kẹp điểm, không có bước), nhập Excel "Tự luận" |
| Backend integration test | 107 / 107 đạt — thêm 6: câu tự luận + trả lời ngắn nâng cao (bước chấm giữ ID khi sửa, lỗi kiểm tra, điểm riêng từng câu, chấm tự động biểu thức / sai số, tự luận chờ chấm); học sinh nộp ảnh → giáo viên / trợ giảng chấm theo bước (quyền, bước lạ, quá điểm, ảnh vẽ không thuộc bài, thông báo một lần, chấm lại, "chấm lại" ở ngân hàng không xóa điểm tay, đề không cho xem đáp án vẫn thấy nhận xét); nhận xét mẫu; trộn mã đề (4 mã khác nhau cùng bộ câu, đánh số lại từng phần, tạo lại thay bộ cũ, quyền); bộ đề; giao nhanh (chỉ nhóm được giao thấy, bản nháp không bị xuất bản theo, hạn làm chặn lượt mới, giao bài tập cả lớp, các lỗi) |
| `cargo fmt --check`, `cargo clippy -D warnings` | Đạt |
| Frontend (Vitest) | 69 / 69 đạt (thêm: điểm câu trong đề / tổng bước / câu tự luận đã làm; mã đề: xếp phương án 4 / 2 / 1 cột, ký hiệu đáp án, gom phần, đầu đề + mã đề + HẾT, ngắt trang + bảng không viền trong Word, bảng đáp án câu × mã + hướng dẫn chấm); `npm run build` đạt |
| Trình duyệt | **Soạn câu tự luận** qua hộp thoại (3 bước chấm, tổng 1 điểm). **Học sinh** (`hs.thu`): đề 2 câu / 3 điểm (câu tự luận đặt 2 điểm) → trả lời ngắn gõ `x·x` được chấm đúng (biểu thức tương đương `x^2`), câu tự luận gõ bài + tải ảnh bài làm → nộp: "điểm tạm 1/3, chờ chấm". **Giáo viên**: trang "Chấm tự luận" → chọn điểm từng bước (xem trước 1/2), lưu nhận xét mẫu, vẽ nhận xét lên ảnh, lưu → 2/3; học sinh nhận thông báo "đã chấm xong", thấy điểm từng bước + nhận xét + lời giải. **Mã đề**: tạo 4 mã → xem từng mã / tất cả / đáp án; file Word dựng trong trình duyệt hợp lệ (4 mã, 3 ngắt trang, 8 công thức OMML). **Bộ đề** tạo qua giao diện → **Giao nhanh** đề cho lớp với hạn làm → lộ trình phiên bản mới có bước đề kèm hạn. Sửa khi thử: ảnh đang tải lên không biến mất khỏi danh sách (so sánh proxy của Vue), hướng dẫn chấm trong bảng đáp án ghi rõ "quy về … điểm" khi tổng bước khác điểm câu |

**Chưa thử được:** vẽ nhận xét khi giáo viên mở trang ở địa chỉ khác `APP_BASE_URL` (dev: `localhost` thay vì `127.0.0.1`) — ảnh khác origin nên trình duyệt không cho xuất canvas; production dùng một tên miền nên không gặp. Mở file Word mã đề bằng Microsoft Word thật. Nhắc hạn làm cho bước đề (mới nhắc hạn nộp bài tập).

**Dữ liệu thử thêm trong DB dev:** của `gv.lan` — câu tự luận "Giải phương trình x² − 5x + 6 = 0" (3 bước chấm), câu trả lời ngắn biểu thức, đề "Kiểm tra 15 phút — phương trình bậc hai (tự luận)" (4 mã đề 101–104), bộ đề "Bộ đề ôn tập chương phương trình bậc hai", lớp "Lớp thử chấm tự luận (đợt 9)" (`hs.thu`, bài đã chấm 2/3, thêm 1 bước giao nhanh có hạn), 1 nhận xét mẫu.

**Để sau:** câu chùm (một đoạn dẫn nhiều câu) và câu hỏi có tham số; nhập hướng dẫn chấm theo bước từ Word; nhắc hạn làm bước đề; gia hạn riêng cho từng học sinh; nhập điểm bài làm trên giấy; rubric theo tiêu chí cho bài tập về nhà.

## Đợt 10 — Bảng dạy Toán WB2 (bắt đầu 2026-11-20)

> Theo lộ trình `analyst.md` mục 9.3: bảng trong phòng học trực tuyến đủ để dạy Toán — không mất bảng, nền ô ly /
> hệ trục, đưa ảnh / PDF đề lên bảng, chọn – di chuyển, công thức, hình học phẳng, quiz từ ngân hàng, bảng riêng
> từng học sinh. Phần bảng nằm trong `hui-whiteboard` (H-UI), phần phòng học ở app.

| Module | Nội dung | Trạng thái |
|---|---|---|
| **M42** | **Bảng không mất + nền + ảnh / PDF**: giáo viên vẽ đến đâu bảng tự lưu nháp lên server đến đó (từng trang, gom 3 giây) → tải lại trang / mất mạng / đổi máy vẫn mở lại đúng bảng. Nền từng trang: trắng, ô vuông, ô ly, dòng kẻ, chấm, hệ trục tọa độ. Chèn ảnh (chọn file / dán Ctrl + V); nhập PDF đề → mỗi trang PDF thành một trang bảng có ảnh nền khóa (vẽ đè lên, không xê dịch nhầm) | ✅ Xong (2026-11-20; nhập PDF làm bổ sung cùng ngày) |
| **M43** | **Chọn – di chuyển – đổi cỡ**: công cụ chọn (chạm một phần tử, Shift chọn thêm, kéo khung chọn nhiều), kéo để di chuyển, kéo góc để phóng to / thu nhỏ (ảnh, đồ thị, hình, công thức), Delete để xóa, Ctrl + D nhân bản; hoàn tác được và đồng bộ sang máy khác | ✅ Xong (2026-11-20) |
| **M44** | **Công thức + hình học**: H-UI thêm `hui-math-input` (ô gõ công thức có bàn phím: phân số, căn, mũ, chỉ số, π, ≤ ≥ ≠, ∞, Δ, ∈ ℝ…, xem trước). Công cụ công thức trên bảng (LaTeX, vẽ bằng MathML). Hình học phẳng: điểm có nhãn tự đặt (A, B, C…), đoạn thẳng bắt dính vào điểm, đường tròn tâm – bán kính, đa giác, giữ Shift để thẳng góc 15°. Ô trả lời ngắn kiểu biểu thức của học sinh có bàn phím công thức | ✅ Xong (2026-11-20) |
| **M45** | **Quiz từ ngân hàng + bảng riêng**: giáo viên chọn câu trong ngân hàng (một / nhiều đáp án, đúng sai, trả lời ngắn) để hỏi nhanh cả lớp, chấm tự động (trả lời ngắn chấm ở server, có sai số / biểu thức), lưu kết quả. **Bảng riêng từng học sinh**: "Mỗi em một bảng" (có thể chép trang đang giảng làm đề), giáo viên xem lưới bảng của cả lớp, mở to một bảng, chiếu bảng của một em cho cả lớp | ✅ Xong (2026-11-20) |

### Thiết kế đợt 10

| Chủ đề | Quyết định |
|---|---|
| Lưu nháp bảng | Bảng `live_board_drafts (session_id, page, data, background, updated_at)`. Mỗi trang gửi riêng (`PUT …/live/board/draft/{page}`) để không vượt giới hạn 1 MB của API; chỉ trang vừa đổi mới gửi lại. Giáo viên / trợ giảng vào phòng: có nháp thì mở nháp, không thì mở bảng đã lưu lúc kết thúc. Học sinh vẫn nhận ảnh chụp bảng từ giáo viên như cũ |
| Dữ liệu trang | Ảnh chụp bảng thêm `backgrounds` (nền từng trang) — ảnh chụp cũ không có thì nền trắng. Thao tác mới: `page-bg`, `update` nhiều phần tử một lần (di chuyển / đổi cỡ nhiều phần tử) |
| Ảnh trên bảng | Phần tử `image` giữ link ảnh (file mục đích `content`, cùng tên miền) chứ không nhúng dữ liệu → thao tác nhỏ, đồng bộ nhanh. Ảnh nền trang PDF có cờ `locked`: công cụ chọn / tẩy bỏ qua |
| PDF → trang bảng (hybrid) | **Trình duyệt đọc trước** bằng `pdfjs-dist` (chỉ trong app, H-UI không thêm thư viện; tải riêng khi giáo viên chọn PDF) → mỗi trang vẽ ra ảnh cao 1800 px (WebP, trình duyệt không có thì JPEG) → tải lên như ảnh nội dung → máy chủ không tốn gì. **Trình duyệt không đọc được** (máy / trình duyệt cũ, lỗi tải thư viện, lỗi vẽ) → gửi PDF lên (file tạm `import`), **máy chủ đọc bằng `hayro`** (Rust thuần, chỉ CPU): vẽ lần lượt từng trang ra PNG, trang xong lưu ngay rồi mới vẽ trang sau (RAM chỉ giữ ~1 trang), cùng lúc chỉ đọc 1 PDF, dừng sau 90 giây. PDF có mật khẩu / hỏng: báo luôn, không gửi máy chủ. Tối đa 30 trang mỗi lần, bảng tối đa 50 trang |
| Công thức trên bảng | LaTeX → MathML (`hui-math`) → ảnh SVG (`foreignObject`) → vẽ lên canvas: công thức nằm trong ảnh chụp / ảnh xuất như nét vẽ, không cần lớp DOM phủ. Đã thử trên Chromium: vẽ được, không làm "bẩn" canvas khi xuất PNG |
| Hình học | Không làm hình học động kiểu GeoGebra (kéo điểm hình đi theo) ở đợt này: điểm / đoạn / đường tròn / đa giác là phần tử tĩnh, có bắt dính vào điểm có sẵn — đủ vẽ hình minh họa khi chữa bài |
| Bảng riêng | Nét vẽ của học sinh chỉ gửi cho người mở bảng riêng (giáo viên / trợ giảng), không gửi cả lớp; giáo viên giữ bảng của từng em trong máy, xem bằng các `hui-whiteboard` chỉ xem thu nhỏ. Bảng riêng giữ nguyên (chỉ ẩn) khi giáo viên chuyển khung khác → chiếu bảng một em không làm các em khác mất bài |
| Trả lời ngắn có biểu thức | Ô trả lời của học sinh dùng `hui-math-input` (bàn phím công thức) và gửi LaTeX; server đổi `\frac`, `\sqrt`, `\cdot`, `\left( \right)`, `^{}`… về cách viết thường trước khi so biểu thức tương đương |

### Kết quả đợt 10 (2026-11-20)

Migration `20261121000001_board_drafts_live_questions.sql`: bảng `live_board_drafts`; `live_board_pages.background`;
`live_polls` thêm loại `question` + `question_id`, `question_kind`, `right_count`.

| Phần | Mã |
|---|---|
| H-UI | Component mới `hui-math-input` (bàn phím ký hiệu, chèn mẫu tại con trỏ / bọc chữ đang chọn, xem trước MathML, `hui-submit`). `hui-whiteboard`: tách `elements.js` (chạm, khung bao, dời, co giãn, bắt dính, nhãn điểm, nền trang, vẽ) và `math-image.js` (công thức → SVG → canvas, bộ nhớ ảnh); công cụ chọn / điểm / đường tròn / đa giác / công thức; nền trang; `addImage` + ảnh nền khóa; `toDataURLAsync`; thao tác `page-bg`, `update` nhiều phần tử, `add` có `under`. Story "MathLesson", changeset `whiteboard-math.md` (minor) |
| Backend | `live`: `save_draft`, `board_data` trả `{ pages, backgrounds, draft }` (ưu tiên nháp), nền khi lưu bảng, `save_poll` loại `question`; `questions::check_answers`; `expr::latex_to_plain`; câu trong bài làm có `expression` |
| Frontend | `LiveRoom.vue`: tự lưu nháp (gom 3 giây, từng trang), mở lại bảng khi vào phòng, chèn ảnh (nút 🖼 / dán), "Mỗi em một bảng" + lưới bảng của lớp + "Chiếu cho cả lớp", câu hỏi ngân hàng; component mới `LiveQuestion.vue`; `QuestionView` (ô biểu thức có bàn phím công thức); `QuestionPickerModal` (chọn một câu, lọc dạng câu); `LiveRecords` (kết quả câu hỏi ngân hàng); `utils/live.js` (`publicQuestion`, `gradeChoice`, `choiceCounts`) |

| API | Mô tả |
|---|---|
| `PUT /api/sessions/{id}/live/board/draft/{page}` | Tự lưu nháp một trang `{ data, background, pageCount }` (giáo viên / trợ giảng); trang nháp ≥ `pageCount` bị xóa |
| `GET /api/sessions/{id}/live/board` | (đổi) `{ pages, backgrounds, draft }` — nháp nếu có, không thì bảng đã lưu |
| `PUT /api/sessions/{id}/live/board` | (đổi) thêm `background` |
| `POST /api/sessions/{id}/live/polls` | (đổi) loại `question`: `questionId` (câu của chính người hỏi), `questionKind`, `rightCount`; trả lời ngắn không cần phương án |
| `POST /api/teaching/questions/{id}/check` | Chấm nhanh `{ answers: [text] }` → `{ results: [bool] }` theo đáp án, sai số, biểu thức tương đương |

| Kiểm tra | Kết quả |
|---|---|
| Backend unit test (`sls-core`) | 86 / 86 đạt (1 bỏ qua: test Garage thật) — thêm: LaTeX từ bàn phím công thức (`\frac`, `\sqrt[3]`, `\cdot`, `\left( \right)`, lệnh lạ báo lỗi) |
| Backend integration test | 108 / 108 đạt — thêm 1 (`live`): nháp từng trang + nền, trang thiếu ở giữa, xóa trang thừa, quyền, nền lạ / dữ liệu sai; chấm nhanh trả lời ngắn (biểu thức), câu của giáo viên khác 404; lưu câu hỏi ngân hàng + lỗi (câu của người khác, số đúng vượt số trả lời); bảng đã lưu trả `{ pages, backgrounds, draft }` |
| `cargo fmt --check`, `cargo clippy -D warnings` | Đạt |
| Frontend (Vitest) | 71 / 71 đạt (thêm: câu gửi học sinh không lộ đáp án, chấm trắc nghiệm / nhiều đáp án / đúng sai, đếm lượt chọn); `npm run build` đạt |
| H-UI | `hui-whiteboard` 6 test cũ giữ nguyên + 6 test mới (chọn – dời – đổi cỡ – nhân bản – xóa – khung chọn, nền trang, hình học + bắt dính + Shift 15°, công thức qua `hui-math-input`, ảnh / ảnh nền khóa / xóa trang, hàm hình học); `hui-math-input` 3 test. Chạy đủ các bước CI ở máy: build, types, CDN, trang mẫu, 225 file / 1622 test (gồm story), kích thước — đạt |
| Trình duyệt (LiveKit thật, 2 tài khoản) | Giáo viên: nền hệ trục, điểm A, B (tự đặt tên), đoạn AB bắt dính, công thức nghiệm qua ô có bàn phím, đồ thị → "💾 tự lưu"; **tải lại trang → bảng mở lại đúng**. Học sinh vào → thấy đủ bảng + nền. Công cụ chọn dời đồ thị, dán ảnh đề (Ctrl + V) → học sinh thấy ngay, ảnh tải được. **Câu hỏi ngân hàng** (trả lời ngắn kiểu biểu thức): học sinh gõ `x^{2}` bằng bàn phím công thức → server chấm khớp `x^2` → học sinh thấy "✓ Bạn trả lời đúng", giáo viên 1/1 đúng. **Mỗi em một bảng**: học sinh nhận trang đề (nền + 6 phần tử), vẽ đường tròn bắt dính điểm A + nét bút → ô của em bên giáo viên cập nhật → "Chiếu cho cả lớp" thành trang 2 của bảng chung, bảng riêng của em vẫn còn. **Kết thúc buổi**: lưu 2 trang bảng (ảnh có công thức, ảnh chèn, đồ thị, nền), kết quả câu hỏi hiện ở trang buổi học. Sửa khi thử: bảng riêng của học sinh trống đề (tin "mở bảng riêng" tới trước tin đổi khung) và bị hủy khi giáo viên chuyển khung — giờ nạp đề khi bảng xuất hiện, giữ bảng (chỉ ẩn) |

#### Bổ sung đợt 10: nhập đề PDF thành trang bảng (hybrid, 2026-11-20)

Theo ý người dùng: trình duyệt đọc được thì để trình duyệt làm cho nhẹ máy chủ, không được thì máy chủ đọc; H-UI không
thêm thư viện ngoài (H-UI không đổi — dùng `addImage(…, { locked: true })` có sẵn).

| Phần | Mã |
|---|---|
| Frontend | `pdfjs-dist` 6.5.438 (chỉ ở app); `utils/pdfPages.js` (`renderPdfInBrowser`, `importPdfPages` — thử trình duyệt, lỗi đọc thì gọi máy chủ; lỗi tải ảnh lên / PDF có mật khẩu / hỏng thì báo luôn); `LiveRoom.vue`: nút 🖼 nhận cả PDF, chữ "📄 Đang đọc PDF: trang 3/10" trên thanh đầu phòng, bảng mới tinh thì dùng luôn trang trống đầu tiên, chuyển cả lớp tới trang đề đầu; `vite.config.js`: plugin phục vụ `/pdfjs/*` (font chuẩn, cmap, wasm giải mã ảnh scan) lúc dev, chép vào `dist/pdfjs/` lúc build; `upload.js`: file tạm `import` nhận thêm PDF |
| Backend | `hayro` 0.9; `live/pdf.rs` (`render_pages` vẽ từng trang ra PNG, PNG quá 5 MB thì vẽ nhỏ lại; `import_pdf` lấy file tạm rồi xóa, vẽ trong `spawn_blocking`, mỗi trang gửi qua kênh để lưu ngay → RAM ~1 trang, 1 PDF cùng lúc, chờ lượt tối đa 60 giây, dừng sau 90 giây); `files`: `Purpose::Import` nhận PDF, `take_import` kiểm tra đúng loại file |

| API | Mô tả |
|---|---|
| `POST /api/sessions/{id}/live/board/pdf` | Dự phòng: `{ fileId, maxPages? }` (file tạm `import`, đọc xong xóa) → `{ total, pages: [{ url, width, height }] }`; giáo viên / trợ giảng của buổi |

| Kiểm tra | Kết quả |
|---|---|
| Backend unit test (`sls-core`) | 90 / 90 đạt (1 bỏ qua) — thêm 4: tỉ lệ trang dọc / ngang, vẽ 2/3 trang ra PNG cao 1800 px, dừng khi bên nhận đóng + vẽ nhỏ lại khi PNG quá lớn, file không phải PDF |
| Backend integration test | 109 / 109 đạt — thêm 1 (`live`): nhập PDF 3 trang lấy 2, tên ảnh "… - trang N.png", file PDF tạm đã xóa, dùng lại id → 404, học sinh 403, PDF hỏng → lỗi trường `file` |
| `cargo fmt`, `cargo clippy -D warnings` | Đạt |
| Frontend (Vitest) | 75 / 75 đạt — thêm 4 (`pdfPages`): tỉ lệ / tên / nhận PDF; trình duyệt đọc được không gọi máy chủ; trình duyệt lỗi → máy chủ (giữ giới hạn 30 trang); mật khẩu / lỗi tải ảnh lên không gửi máy chủ. `npm run build` đạt (pdfjs tách file riêng 444 kB + worker 1,3 MB, chỉ tải khi chọn PDF) |
| Trình duyệt (LiveKit thật, 2 tài khoản) | PDF tiếng Việt 10 trang: **trình duyệt đọc** → 10 trang bảng có ảnh nền khóa (WebP ~340 kB/trang, phóng to 3 lần chữ vẫn nét). **Đường máy chủ** chạy thật ở lần thử đầu (lỗi `doc.destroy` của pdfjs 6 lúc dọn dẹp → tự chuyển máy chủ): `hayro` đọc 10 trang trong 4,6 giây, tiếng Việt / chữ đậm đúng. Học sinh đang trong phòng nhận ngay các trang mới và đi theo trang đề; học sinh vào sau thấy đủ; giáo viên tải lại trang → đủ 30 trang + ảnh đề. Sửa khi thử: pdfjs 6 bỏ `PDFDocumentProxy.destroy()` → dùng `loadingTask.destroy()`, lỗi lúc dọn dẹp không còn làm hỏng trang đã đọc |

**Dữ liệu thử thêm trong DB dev:** buổi "Buổi thử nhập đề PDF" (lớp "Lớp thử chấm tự luận (đợt 9)", 30 trang bảng nháp) và
30 ảnh trang đề.

**Chưa thử được:** công thức trên bảng ở Safari (Safari không cho xuất canvas có ảnh SVG chứa HTML → "Lưu bảng" có thể
lỗi khi trang có công thức; hiển thị vẫn đúng). Bảng riêng khi học sinh tải lại trang (nét đã vẽ của em mất ở máy em,
giáo viên vẫn giữ).

**Dữ liệu thử thêm trong DB dev:** buổi "Buổi thử bảng WB2" (đã kết thúc, 2 trang bảng, 1 câu hỏi ngân hàng) của lớp
"Lớp thử chấm tự luận (đợt 9)"; 1 ảnh nội dung "de-bai.png".

**Để sau:** trang PDF dọc (A4) hiện khá nhỏ trên khung bảng 16:9 → phóng to / kéo bảng, hoặc chia đôi trang dọc; hình học động (kéo điểm, hình đi theo), đo góc / độ dài; hình khối
(`hui-solid`); bảng riêng lưu ở server (học sinh tải lại không mất); giáo viên chấm điểm bảng riêng; nhắc hạn làm bước đề.

### Triển khai production, một hoặc nhiều máy (2026-11-08)

Thiết kế: `analyst.md` mục "Tách việc nặng, AI, live và chạy nhiều máy". Cách chạy: `README.md` mục "Triển khai".

| Việc | Trạng thái |
|---|---|
| `backend/Dockerfile`: một lần build ra image `sls-api` + `sls-worker` (Rust 1.99.1, distroless Debian 13, user không phải root) | ✅ |
| `deploy/docker-compose.yml` dùng chung mọi máy: mỗi service một profile trùng tên; địa chỉ service khác đọc từ `*_HOST` / `*_PORT`; cổng mở trên `PUBLISH_IP`; không dùng `depends_on` để `up` không kéo service của máy khác | ✅ |
| `deploy/.env.example` (cấu hình từng máy) + `deploy/Caddyfile` (SPA + `/api`, cache `/assets`, HTTPS tự động khi có tên miền) | ✅ |
| Worker gửi email qua SMTP có TLS: `SMTP_TLS` = `starttls` (mặc định, cổng 587) · `tls` (cổng 465) · `none` (Mailpit khi dev); kiểm tra chứng chỉ bằng bộ CA gói sẵn (`webpki-roots`), thêm CA riêng bằng `SMTP_CA_FILE`. Test: 8 unit test cấu hình + gửi thật qua Mailpit bật STARTTLS và TLS với CA tự tạo | ✅ (2026-11-08) |
| Sao lưu Postgres theo lịch (`pg_dump`) | Chưa làm |
| Garage (lưu file): profile `garage`, `deploy/garage/init.sh prod`, Caddy chuyển `/files/*` | ✅ (2026-11-13, đợt 5) |
| LiveKit (phòng học trực tuyến): profile `livekit`, Caddy chuyển `/rtc*`, mở 7881/tcp + 7882/udp | ✅ (2026-11-13, đợt 6) |
| `import-worker`, `media-worker` (ghi hình → HLS), `ai-service`: thêm vào compose khi làm các module tương ứng | Để sau |

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

**M6.2 (đã xong 2026-11-05)** — `core/src/modules/sso/oidc.rs` (client OIDC), `sso/flow.rs` (start / callback), migration `20261106000001_sso_flow_session_limit.sql` (`sso_login_states`, cột `refresh_tokens.revoked_reason`). Cấu hình: `SSO_GOOGLE_CLIENT_ID/SECRET`, `SSO_MICROSOFT_CLIENT_ID/SECRET/TENANT`, `SSO_REDIRECT_BASE`, `MAX_SESSIONS_PER_USER` (mẫu ở `backend/.env.example`, cách tạo ứng dụng OAuth ở `README.md`).

| API | Mô tả |
|---|---|
| `GET /api/auth/sso/providers` | Nhà cung cấp đã cấu hình → frontend chỉ hiện nút của các nhà cung cấp này |
| `POST /api/auth/sso/{provider}/start` | Body `{ mode: "login" \| "link", next }` → `{ authorizeUrl }` + cookie `sls_sso_state` (HttpOnly, SameSite=Lax, 10 phút). `link` cần đăng nhập |
| `GET /api/auth/sso/{provider}/callback` | Kiểm tra `state` (khớp cookie, dùng một lần), đổi code (kèm PKCE) lấy id_token, kiểm tra chữ ký JWKS / `iss` / `aud` / `exp` / `nonce`, rồi chuyển hướng 303 về: `/sso/xong?next=…&new=1` · `/sso/lien-ket#token=…` · `/ho-so?ssoLinked=…` · `/dang-nhap?ssoError=MÃ` (hoặc `/ho-so?ssoError=…` khi đang liên kết) |
| `POST /api/auth/set-password` | Tài khoản tạo từ SSO đặt mật khẩu lần đầu (đã có mật khẩu → 409 `PASSWORD_ALREADY_SET`); thiết bị khác bị đăng xuất. `/api/auth/me` có thêm `hasPassword` |

**Quyết định khi làm:**

| Chủ đề | Quyết định | Lý do |
|---|---|---|
| Thư viện OIDC | Tự viết phần nhỏ cần dùng bằng `reqwest` + `jsonwebtoken` thay vì crate `openidconnect` (analyst.md) | Chỉ cần Authorization Code + PKCE với 2 nhà cung cấp; ít phụ thuộc, dễ đọc khi học Rust. Cấu hình + JWKS giữ trong bộ nhớ 1 giờ, gặp `kid` lạ thì tải lại (tối đa 1 lần / phút) |
| Lưu `state` | Bảng `sso_login_states` (PostgreSQL), không dùng Valkey như dự kiến | Callback lấy-và-xóa trong một câu `DELETE … RETURNING` (chắc chắn dùng một lần); test chạy được không cần Valkey |
| Chống ép đăng nhập (login CSRF) | `state` vừa lưu server vừa nằm trong cookie của trình duyệt; callback phải khớp cả hai | Link callback bị gửi sang trình duyệt khác thì không dùng được |
| Token liên kết | Đặt ở phần `#` của URL, trang `/sso/lien-ket` đọc xong xóa khỏi thanh địa chỉ | Phần `#` không gửi lên server và không lộ qua `Referer` |
| Email Microsoft | Tin email khi: có `email_verified`, hoặc tài khoản cá nhân (tenant `9188040d-…`), hoặc có claim `xms_edov` = true. Còn lại → `SSO_EMAIL_UNVERIFIED` | Email tài khoản trường / công ty do quản trị tenant tự đặt, không được Microsoft xác minh (lỗ hổng kiểu "nOAuth"). Cần bật optional claim `xms_edov` trong Entra |
| Giới hạn thiết bị | Mặc định 2, áp dụng mọi vai trò. Đăng nhập mới (mật khẩu, OTP, SSO) giữ lại 2 phiên **dùng gần nhất**; xoay vòng refresh token không tính là thiết bị mới. Thiết bị bị đăng xuất nhận `401 SESSION_LIMIT` khi refresh, trang đăng nhập báo lý do | Đúng mục đích chống chia sẻ tài khoản; đăng xuất thiết bị lâu không dùng ít phiền hơn đăng xuất thiết bị đăng nhập sớm nhất |
| Liên kết khi đã đăng nhập | Không cần nhập lại mật khẩu; email Google / Microsoft được phép khác email tài khoản | Người dùng đã chứng minh sở hữu tài khoản bằng phiên đang dùng |

| # | Ca test M6.2 (`api/tests/sso_oidc.rs` 9 test, `sessions.rs` 4 test — nhà cung cấp OIDC giả `tests/common/oidc.rs`) | Kết quả |
|---|---|---|
| 1 | Danh sách nhà cung cấp; nhà cung cấp chưa bật / không tồn tại | `[]` khi chưa cấu hình; 404 |
| 2 | URL đăng nhập | Có `client_id`, `redirect_uri`, `state` = cookie, `nonce`, PKCE S256; cookie HttpOnly + SameSite=Lax + Path `/api/auth/sso` + 10 phút |
| 3 | Email mới | Tạo học sinh `hasPassword=false`, chuyển `/sso/xong?next=%2Flop-hoc&new=1`; lần sau đăng nhập luôn; `next=//evil.com` bị bỏ |
| 4 | `state` | Thiếu cookie / cookie lượt khác / dùng lại / dùng cho nhà cung cấp khác → `SSO_STATE_INVALID` |
| 5 | Bấm Hủy ở nhà cung cấp | `SSO_CANCELLED` |
| 6 | id_token sai `nonce` / `aud` / `iss` / hết hạn / sửa nội dung (chữ ký sai) | `SSO_FAILED`, không tạo tài khoản; email chưa xác minh → `SSO_EMAIL_UNVERIFIED` |
| 7 | Trùng email tài khoản đã có | `/sso/lien-ket#token…&email=l***@…&password=1` → xác nhận mật khẩu → lần sau vào thẳng tài khoản |
| 8 | Liên kết khi đã đăng nhập | Không token → 401; thành công `/ho-so?ssoLinked=google` (không tạo phiên mới); liên kết lại cùng danh tính vẫn OK; Google thứ hai → `SSO_PROVIDER_ALREADY_LINKED`; danh tính của người khác → `SSO_ALREADY_LINKED`; nhật ký `identity.link` |
| 9 | Microsoft | Tài khoản cá nhân → tạo được; tenant trường không có `xms_edov` → `SSO_EMAIL_UNVERIFIED`, có → tạo được; `iss` khác tenant `tid` → `SSO_FAILED` |
| 10 | Giới hạn 2 thiết bị | Thiết bị thứ 3 đăng xuất thiết bị lâu không dùng nhất (`SESSION_LIMIT`, cookie bị xóa); 2 thiết bị còn lại dùng tiếp; `MAX_SESSIONS_PER_USER=0` → không giới hạn |
| 11 | Đặt mật khẩu lần đầu | Mật khẩu yếu 422; đặt xong đăng nhập bằng mật khẩu được, `hasPassword=true`; đặt lần 2 / tài khoản đã có mật khẩu → 409; không đăng nhập → 401 |
| U | `pkce_challenge` theo ví dụ RFC 7636; đọc `email_verified` dạng bool / chuỗi; `sanitize_next`; cấu hình SSO + giới hạn thiết bị | Unit (`sls-core`) |

**Giao diện:** nút "Đăng nhập / Đăng ký với Google · Microsoft" (`components/SsoButtons.vue`), `/sso/xong`, `/sso/lien-ket` (mật khẩu hoặc mã qua email), Hồ sơ: "Đặt mật khẩu" cho tài khoản SSO + "Tài khoản liên kết" (liên kết / gỡ), admin Người dùng → "Liên kết SSO" (xem / gỡ). Trang đăng nhập báo lỗi `ssoError` và lý do bị đăng xuất (`SESSION_LIMIT`). Unit test `utils/sso.test.js` (4).

**Đã thử trên trình duyệt** (API trỏ tới một nhà cung cấp OIDC giả chạy ở máy): đăng ký bằng Google → Hồ sơ có lời chào + form đặt mật khẩu → liên kết thêm Microsoft → đặt mật khẩu → đăng xuất → Google với email của admin → trang xác nhận (token đã bị xóa khỏi URL) → nhận mã qua Mailpit → vào tài khoản admin và quay đúng trang `redirect` ban đầu → admin xem / gỡ liên kết của user → đăng nhập admin ở 2 nơi khác → trình duyệt bị đăng xuất kèm thông báo → bấm Hủy ở nhà cung cấp → thông báo "Bạn đã hủy đăng nhập".

**Còn lại của M6:** thử với Google / Microsoft thật (cần bạn tạo ứng dụng OAuth, xem `README.md`). Chưa làm (analyst.md mục 3, chưa xếp lịch): admin khóa SSO của một tài khoản; "chỉ cho đăng nhập bằng SSO" với giáo viên.

### Kết quả kiểm tra G1 (cập nhật sau M6.2, 2026-11-05)

| Hạng mục | Kết quả |
|---|---|
| Backend unit test (`sls-core`) | 31 / 31 đạt |
| Backend integration test (`sls-api/tests`, PostgreSQL thật, mỗi test một database) | 63 / 63 đạt — health 3, auth 18, rbac 3, admin_users 7, catalog 6, teachers 3, sso 10, sso_oidc 9, sessions 4 |
| `cargo fmt --check`, `cargo clippy -D warnings` | Đạt |
| Frontend unit test (Vitest) | 23 / 23 đạt — API client 9, menu + guard 7, danh sách phân trang 3, SSO 4 |
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
| `hui-modal` có cả property `open` và method `open()` → Vue `:open` không mở được modal | Đã sửa trong H-UI và **đã push** (2026-11-05, commit `3556006` → `c336342`): 20 overlay dùng chung `open` (boolean) + `show()` / `hide()` / `toggle()` + sự kiện `hui-open` / `hui-after-open` / `hui-close` / `hui-after-close`. Quy ước của H-UI: `.agents/rules/naming_rule.md`. Release bản 0.5.2 qua PR "chore: version packages" do CI mở. App dùng lại `:open="x"`, ~~`:open.attr="x \|\| null"`~~ |
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
