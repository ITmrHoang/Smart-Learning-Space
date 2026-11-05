# Smart-Learning-Space — Yêu cầu nghiệp vụ

> File này chỉ ghi các yêu cầu nghiệp vụ lớn. Không xóa yêu cầu cũ: yêu cầu thay đổi được đánh dấu `[UPDATED]`, phần cũ gạch bằng `~~...~~`.
> Phân tích kỹ thuật cho từng yêu cầu nằm trong `analyst.md`.
>
> Trạng thái: **[Chốt]** đã thống nhất · **[Đề xuất]** chờ bạn duyệt · **[Sau]** làm sau MVP

---

## v0.1 — 2026-11-05

### R1. Mục đích & phạm vi

- **[Chốt]** Hệ thống học và dạy trực tuyến cho giáo viên và học sinh, đối tượng chính là học sinh ~~**THCS và THPT** (khối 6–12)~~. **[UPDATED v0.3]** Lớp 1–12 và các lớp riêng tự thêm — xem R3 ở v0.3.
- **[Chốt]** Là **nền tảng độc lập**, không phải hệ thống quản lý của một trường. "Trường" chỉ là thông tin trong hồ sơ học sinh (tên trường, tỉnh/thành, khối lớp).
- **[Chốt]** Có nhiều giáo viên, dạy nhiều môn khác nhau. Một giáo viên dạy được nhiều môn, nhiều lớp; một học sinh học được nhiều lớp.
- **[Chốt]** Quy mô ban đầu khoảng vài nghìn học sinh, có thể mở rộng.

### R2. Tài khoản

- **[Chốt]** Vai trò: admin, giáo viên, học sinh. Không có tài khoản phụ huynh; thông tin phụ huynh chỉ dùng để gửi báo cáo học tập qua email.
- **[Chốt]** Có đăng nhập SSO (Google / Microsoft); SSO phải được liên kết đúng với username hoặc email đã đăng ký của người dùng đó.
- **[Đề xuất]** Học sinh tự đăng ký (xác minh email). Giáo viên đăng ký kèm hồ sơ, và phải được admin duyệt mới được dạy.
- **[Đề xuất]** Khi học sinh đăng ký, thu thập email phụ huynh và gửi email để phụ huynh xác nhận đồng ý (học sinh chưa đủ 18 tuổi).

### R3. Nội dung học

- **[Chốt]** Giáo viên tạo bài học, bài giảng, khóa học.
- **[Chốt]** Video bài giảng: tự host (upload) hoặc link YouTube.
- ~~**[Đề xuất]** Tổ chức nội dung theo: danh mục chung **Môn × Khối × Chủ đề** → **Khóa học** (gói nội dung) → **Lớp học** (đợt học có học sinh và lịch).~~ **[UPDATED v0.2]** Đổi tên gọi, bỏ từ "chủ đề" — xem R3 ở v0.2.
- **[Đề xuất]** Lịch học gắn với **lớp**, không gắn với khóa học. **[UPDATED v0.2]** Mở rộng thành lên lịch kiểu Google Meet — xem R9 ở v0.2.

### R4. Hiển thị & ghi danh

- **[Chốt]** Khóa học có thể công khai (public) hoặc riêng tư (private). Muốn học phải đăng nhập và đăng ký; giáo viên có thể thêm học sinh hoặc duyệt yêu cầu đăng ký.
- **[Chốt]** Lớp live cũng vậy. Có những buổi mở, không cần đăng nhập, nhưng phải có cơ chế bảo mật.

### R5. Câu hỏi, ôn tập, kiểm tra

- **[Chốt]** Giáo viên tạo bộ câu hỏi cho học sinh ôn tập, và tạo bộ đề kiểm tra.
- **[Chốt]** Học sinh vào học, ôn tập, làm bài kiểm tra và bài ôn luyện.
- **[Chốt]** Import bộ câu hỏi bằng **Excel**, nâng cấp lên **Word** và **PDF**. Câu hỏi có thể có nhiều hình ảnh, công thức (Toán, hình học…).
- **[Chốt]** **Trộn đề**: cùng một bộ đề, nhưng mỗi học sinh nhận thứ tự câu khác nhau (trộn trong từng loại câu) và thứ tự đáp án khác nhau. Điểm vẫn phải chấm đúng.

### R6. Dạy trực tiếp (live)

- **[Chốt]** Lớp live khoảng 30–60 người, có video và tương tác.
- **[Chốt]** **Bảng thông minh (smartboard)** cho giáo viên:
  - Vẽ tay; thêm phần tử hình học, hệ trục tọa độ bằng chuột và bàn phím.
  - Nhập công thức thì vẽ ra đồ thị tương ứng.
  - Có chế độ cho học sinh làm bài trên bảng, cả lớp cùng xem. Nếu giáo viên cho phép, người khác trong lớp cũng có thể tương tác.
- Chi tiết thêm về tương tác trong buổi live: bạn sẽ mô tả sau.

### R7. Thanh toán

- **[Sau]** Học sinh trả học phí cho giáo viên tương ứng, theo số buổi học hoặc theo tháng.

### R8. Đề xuất bổ sung (chờ duyệt — phân tích ở `analyst.md` mục 4)

| # | Đề xuất | Lý do |
|---|---|---|
| D1 | Duyệt giáo viên + trang hồ sơ giáo viên công khai | Nền tảng mở, cần kiểm soát ai được dạy học sinh vị thành niên |
| D2 | Bài tập về nhà: học sinh nộp ảnh/PDF bài làm trên giấy, giáo viên viết nhận xét đè lên ảnh | Cách học Toán, Lý, Hóa phổ biến nhất khi học online |
| D3 | Hỏi đáp theo từng bài học (có công thức, ảnh) | Học sinh hỏi bài ngoài giờ live |
| D4 | Ghi hình buổi live, tự động thành video bài học; lưu lại nội dung bảng | Học sinh vắng học bù, ôn lại |
| D5 | Video bài giảng có câu hỏi chèn giữa video | Biến video xem thụ động thành video có tương tác |
| D6 | Chống chia sẻ tài khoản và bảo vệ nội dung có phí: giới hạn số thiết bị, watermark tên học sinh trên video | Cần thiết khi đã thu học phí |
| D7 | Kiểm duyệt và báo cáo vi phạm; không có chat riêng giữa hai học sinh | Người dùng là trẻ vị thành niên |
| D8 | Tuân thủ quy định bảo vệ dữ liệu cá nhân của trẻ em; thông báo khi ghi hình | Bắt buộc về pháp lý |
| D9 | Nhắc lịch học và hạn nộp bài (email + trong app) | Giữ học sinh học đều |
| D10 | Giao diện tốt trên điện thoại / máy tính bảng (PWA) | Phần lớn học sinh học bằng điện thoại |
| D11 | Báo cáo cho giáo viên: điểm theo chủ đề, câu sai nhiều, học sinh chậm; xuất bảng điểm ra Excel | Phần "thông minh" giúp giáo viên dạy đúng chỗ yếu |
| D12 | Trợ giảng với quyền hạn chế (chấm bài, duyệt phòng chờ, quản lý chat) | Lớp live 60 người, một giáo viên khó vừa dạy vừa quản lý |
| D13 | Đề theo cấu trúc thi THPT từ 2025 (3 phần: nhiều lựa chọn, đúng/sai 4 ý, trả lời ngắn) và **ma trận đề** (rút câu ngẫu nhiên theo ~~chủ đề~~ chương/bài × mức độ) — **[Chốt v0.2]** | Đúng nhu cầu ôn thi của học sinh THPT |
| D14 | Trang khóa học và trang giáo viên công khai được SEO | Nền tảng mở cần học sinh tìm thấy khóa học |

---

## v0.2 — 2026-11-05 (bổ sung sau trao đổi)

### R3. Nội dung học [UPDATED]

- **[Chốt]** Không dùng từ "chủ đề". Tên gọi thống nhất:
  - **Môn học** (Toán, Ngữ văn, Tiếng Anh…) và **Lớp** (Lớp 6 → Lớp 12): danh sách chung của hệ thống.
  - **Danh mục**: nhóm khóa học theo mục tiêu học, để học sinh tìm khóa học. Ví dụ: Ôn thi vào 10, Ôn thi tốt nghiệp THPT, Học sinh giỏi, Lấy lại gốc.
  - **Khóa học** → **Chương / Phần** (giáo viên tự đặt tên) → **Bài**.
  - **Lớp học** (một nhóm học sinh học cùng giáo viên) → **Buổi** (một lần học live theo lịch). Mỗi buổi gắn các bài sẽ dạy.
- **[Chốt]** Giáo viên tạo **lộ trình học**: chọn bài, bộ ôn tập, đề kiểm tra theo thứ tự; đổi thứ tự, thêm, bớt, sửa nội dung bất cứ lúc nào.
- **[Đề xuất]** Nội dung đang sửa lưu thành bản nháp, giáo viên bấm "Xuất bản" thì học sinh mới thấy bản mới. Đổi thứ tự hay sửa nội dung không làm mất tiến độ của học sinh.

### R5. Câu hỏi, đề, import [UPDATED]

- **[Chốt]** Có **ma trận đề** (D13).
- **[Chốt]** Import Word: công thức **MathType lấy luôn thành ảnh**, vì giáo viên đã soạn sẵn trên Word.
- **[Chốt]** Import PDF, gồm hai loại: **PDF có chữ** (xuất từ Word) và **PDF ảnh** (scan, chụp). Phần công thức và hình vẽ giữ nguyên thành ảnh.
- **[Đề xuất]** Giáo viên sửa đáp án đúng của câu đã có học sinh làm → hệ thống **chấm lại** các bài đã nộp.

### R6. Bảng thông minh [UPDATED]

- **[Chốt]** Có phần tử **hình 2D** (đoạn thẳng, tam giác, đa giác, đường tròn…) và **hình khối** (lập phương, hộp chữ nhật, lăng trụ, chóp, trụ, nón, cầu…).
- **[Chốt]** Có phần tử **hệ trục tọa độ**: nhập công thức như `y = 2x + 1` thì đồ thị hiện ngay trên hệ trục đó.

### R9. Lịch & mời học (mới)

- **[Chốt]** Giáo viên lên lịch buổi học vào một khung giờ trên lịch và gửi lời mời cho **cả lớp học, một nhóm, hoặc từng học sinh**, giống Google Meet.
- **[Đề xuất]** Lời mời gửi trong app và qua email kèm file lịch `.ics` (tự thêm vào Google Calendar, Outlook, lịch điện thoại). Học sinh trả lời tham gia / không tham gia. Dời hoặc hủy buổi thì người được mời nhận cập nhật.
- **[Đề xuất]** Giáo viên tạo **nhóm học sinh** (ví dụ "Nhóm ôn hình học", "Nhóm học sinh giỏi") để mời học, giao bài, giao lộ trình.

### R8. Đề xuất thêm

| # | Đề xuất | Lý do |
|---|---|---|
| D15 | Giáo viên mở các khung giờ rảnh, học sinh tự đặt lịch học kèm 1-1, giáo viên duyệt | Học kèm cá nhân rất phổ biến; đỡ phải nhắn tin hẹn giờ qua lại |

---

## v0.3 — 2026-11-05 (bổ sung sau trao đổi)

### R3. Cấu trúc nội dung [UPDATED — thay R3 của v0.2]

- **[Chốt]** **Môn học** là trục chính.
- **[Chốt]** **Lớp**: Lớp 1 → Lớp 12, cộng thêm **lớp riêng** tự thêm ngoài danh sách (ví dụ "Ôn thi IELTS", "Đại học").
- **[Chốt]** **Danh mục** là nhãn (tag) nằm trong một môn, ví dụ Toán: Đại số, Hình học, Ôn thi vào 10.
- **[Chốt]** **Lớp học** và **môn học** là chính. **Bài** và **buổi** chỉ là các bước trong **lộ trình** của lớp học. Trong lộ trình có các **buổi live** học trực tuyến, được lên lịch.
- ~~Khóa học → Chương / Phần → Bài là một tầng riêng (v0.2).~~ Không còn tầng "khóa học" riêng.

### R6. Tương tác trong buổi live [UPDATED]

- **[Chốt]** Học sinh **giơ tay**; giáo viên chọn học sinh → trên màn hình học sinh bật **popup bảng** để viết lời giải; cả lớp cùng xem và tương tác.
- Câu "live stream có video tương tác. Ngoài…" ở v0.1 chỉ là xuống dòng, nối tiếp với đoạn bảng thông minh — không thiếu nội dung.

### R10. Triển khai (mới)

- **[Chốt]** Bắt đầu làm theo từng module, theo thứ tự: cấu trúc dự án → đăng nhập → phân quyền → admin quản lý user → master data; mỗi phần có kế hoạch test riêng. Kế hoạch chi tiết: `docs/implementation-plan.md`.
