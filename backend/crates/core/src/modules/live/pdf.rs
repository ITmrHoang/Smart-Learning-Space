//! Nhập đề PDF thành trang bảng — đường dự phòng ở máy chủ.
//!
//! Bình thường trình duyệt của giáo viên tự đọc PDF (pdfjs-dist, chỉ tải khi cần) nên máy chủ không tốn gì.
//! Chỉ khi trình duyệt không đọc được (máy / trình duyệt cũ, lỗi thư viện) thì PDF mới được gửi lên đây:
//! `hayro` (Rust thuần, chỉ dùng CPU) vẽ **lần lượt từng trang** thành PNG, mỗi trang xong là lưu ngay
//! thành ảnh nội dung rồi mới vẽ trang sau → bộ nhớ chỉ giữ khoảng một trang. Cùng lúc chỉ đọc một PDF.

use std::{
    sync::LazyLock,
    time::{Duration, Instant},
};

use hayro::{
    PixmapSettings, RenderCache, RenderSettings,
    hayro_interpret::InterpreterSettings,
    hayro_syntax::{LoadPdfError, Pdf},
    render,
    vello_cpu::color::palette::css::WHITE,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{Semaphore, mpsc};
use uuid::Uuid;

use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    modules::{
        files::{self, CONTENT_MAX_BYTES, PDF, Purpose},
        sessions::service as sessions,
    },
    security::Actor,
};

/// Tối đa số trang mỗi lần nhập.
pub const PDF_MAX_PAGES: usize = 30;
/// Chiều cao ảnh mỗi trang (px): gấp đôi chiều cao trang bảng (900) để chữ vẫn rõ trên màn hình nét cao.
pub const PAGE_HEIGHT_PX: f32 = 1800.0;
/// Trang ngang rất rộng: giới hạn chiều rộng ảnh.
pub const PAGE_MAX_WIDTH_PX: f32 = 3200.0;
/// Quá thời gian này thì dừng, trả các trang đã vẽ xong.
const RENDER_BUDGET: Duration = Duration::from_secs(90);
/// Chờ lượt đọc PDF (đang có người khác đọc) tối đa chừng này.
const SLOT_WAIT: Duration = Duration::from_secs(60);
/// Cùng lúc chỉ đọc một PDF để API vẫn nhẹ RAM / CPU.
static RENDER_SLOTS: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(1));

/// Tỉ lệ phóng (1 = 1 pt / px) để ảnh trang cao `PAGE_HEIGHT_PX`, nhưng không rộng quá `PAGE_MAX_WIDTH_PX`.
pub fn page_scale(width: f32, height: f32) -> f32 {
    if !(width > 0.0 && height > 0.0) {
        return 1.0;
    }
    (PAGE_HEIGHT_PX / height).min(PAGE_MAX_WIDTH_PX / width)
}

#[derive(Debug)]
pub struct RenderedPage {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfError {
    /// Không phải PDF hoặc file hỏng.
    Invalid,
    /// PDF có mật khẩu.
    Encrypted,
}

/// Vẽ lần lượt tối đa `max_pages` trang đầu; mỗi trang xong gọi `on_page` (trả `false` để dừng).
/// PNG lớn hơn `max_png` byte (trang scan nhiều chi tiết) thì vẽ lại nhỏ hơn. Quá `budget` thì dừng sớm.
/// Trả tổng số trang của file.
pub fn render_pages(
    bytes: Vec<u8>,
    max_pages: usize,
    max_png: usize,
    budget: Duration,
    mut on_page: impl FnMut(RenderedPage) -> bool,
) -> Result<usize, PdfError> {
    let pdf = Pdf::new(bytes).map_err(|e| match e {
        LoadPdfError::Decryption(_) => PdfError::Encrypted,
        _ => PdfError::Invalid,
    })?;
    let pages = pdf.pages();
    let total = pages.len();
    let started = Instant::now();
    let cache = RenderCache::new();
    let interpreter = InterpreterSettings::default();
    let settings = RenderSettings::default();
    for page in pages.iter().take(max_pages) {
        if started.elapsed() > budget {
            break;
        }
        let (w, h) = page.render_dimensions();
        let mut scale = page_scale(w, h);
        let rendered = loop {
            let pixmap = render(
                page,
                &cache,
                &interpreter,
                &settings,
                &PixmapSettings { x_scale: scale, y_scale: scale, bg_color: WHITE },
            );
            let (width, height) = (u32::from(pixmap.width()), u32::from(pixmap.height()));
            let png = pixmap.into_png().map_err(|_| PdfError::Invalid)?;
            if png.len() <= max_png || scale < 0.4 {
                break RenderedPage { png, width, height };
            }
            scale *= 0.7;
        };
        if !on_page(rendered) {
            break;
        }
    }
    Ok(total)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfImportInput {
    /// File PDF đã upload với mục đích `import` (đọc xong là xóa).
    pub file_id: Uuid,
    /// Số trang tối đa muốn lấy (bảng còn chỗ bao nhiêu trang); mặc định / tối đa `PDF_MAX_PAGES`.
    pub max_pages: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfPageView {
    pub url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfImportView {
    /// Tổng số trang của file (có thể nhiều hơn số trang đã lấy).
    pub total: usize,
    pub pages: Vec<PdfPageView>,
}

fn pdf_error(e: PdfError) -> AppError {
    match e {
        PdfError::Encrypted => {
            AppError::field("file", "PDF có mật khẩu — hãy mở bằng trình đọc PDF, bỏ mật khẩu rồi thử lại.")
        }
        PdfError::Invalid => AppError::field("file", "Không đọc được file PDF (file hỏng hoặc không phải PDF)."),
    }
}

/// Giáo viên / trợ giảng của buổi: đọc PDF vừa upload thành ảnh từng trang (ảnh nội dung) để đặt làm nền trang bảng.
pub async fn import_pdf(
    ctx: &AppContext,
    actor: &Actor,
    session_id: Uuid,
    input: PdfImportInput,
) -> AppResult<PdfImportView> {
    sessions::live_staff(ctx, actor, session_id).await?;
    let (name, bytes) = files::take_import(ctx, actor, input.file_id, PDF).await?;
    let max_pages = input.max_pages.unwrap_or(PDF_MAX_PAGES).clamp(1, PDF_MAX_PAGES);
    let _slot = tokio::time::timeout(SLOT_WAIT, RENDER_SLOTS.acquire())
        .await
        .map_err(|_| {
            AppError::new(
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "PDF_BUSY",
                "Máy chủ đang đọc một PDF khác — hãy thử lại sau ít phút.",
            )
        })?
        .map_err(AppError::internal)?;

    let stem = name.rsplit_once('.').map_or(name.as_str(), |(s, _)| s).to_string();
    let (tx, mut rx) = mpsc::channel::<RenderedPage>(1);
    let max_png = CONTENT_MAX_BYTES as usize;
    let render_task = tokio::task::spawn_blocking(move || {
        render_pages(bytes, max_pages, max_png, RENDER_BUDGET, |page| tx.blocking_send(page).is_ok())
    });
    let mut pages = Vec::new();
    let mut failed = None;
    while let Some(page) = rx.recv().await {
        let file_name = format!("{stem} - trang {}.png", pages.len() + 1);
        match files::save_bytes(ctx, actor.id, Purpose::Content, &file_name, "image/png", &page.png).await {
            Ok(row) => pages.push(PdfPageView {
                url: files::file_url(ctx, row.id, &row.purpose),
                width: page.width,
                height: page.height,
            }),
            Err(e) => {
                failed = Some(e);
                break;
            }
        }
    }
    // Đóng kênh → nếu đang vẽ dở thì dừng ở trang kế.
    drop(rx);
    // Thư viện panic (file lạ) → coi như không đọc được.
    let rendered = render_task.await.map_err(|_| pdf_error(PdfError::Invalid))?;
    if let Some(e) = failed {
        return Err(e);
    }
    let total = rendered.map_err(pdf_error)?;
    if pages.is_empty() {
        return Err(AppError::field("file", "PDF không có trang nào."));
    }
    Ok(PdfImportView { total, pages })
}

/// PDF mẫu `pages` trang A4 (mỗi trang một hình chữ nhật đen) — dùng cho test.
#[doc(hidden)]
pub fn sample_pdf(pages: usize) -> Vec<u8> {
    let kids: Vec<String> = (0..pages).map(|i| format!("{} 0 R", 3 + i * 2)).collect();
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        format!("<< /Type /Pages /Kids [{}] /Count {pages} >>", kids.join(" ")),
    ];
    for i in 0..pages {
        let content = format!("0 0 0 rg 100 {} 200 100 re f", 600 - i * 20);
        objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents {} 0 R >>", 4 + i * 2));
        objects.push(format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()));
    }
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in offsets {
        out.extend(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend(
        format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1).as_bytes(),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUDGET: Duration = Duration::from_secs(60);

    #[test]
    fn ti_le_trang_doc_va_ngang() {
        // A4 dọc 595 × 842 pt → cao 1800 px
        let s = page_scale(595.0, 842.0);
        assert!((842.0 * s - PAGE_HEIGHT_PX).abs() < 0.01);
        // Trang ngang rất dài → giới hạn theo chiều rộng
        let s = page_scale(5000.0, 400.0);
        assert!((5000.0 * s - PAGE_MAX_WIDTH_PX).abs() < 0.01);
        assert_eq!(page_scale(0.0, 100.0), 1.0);
    }

    #[test]
    fn ve_tung_trang_thanh_png() {
        let mut got = Vec::new();
        let total = render_pages(sample_pdf(3), 2, usize::MAX, BUDGET, |p| {
            got.push(p);
            true
        })
        .unwrap();
        assert_eq!(total, 3);
        assert_eq!(got.len(), 2, "chỉ lấy 2 trang đầu");
        for p in &got {
            assert!(p.png.starts_with(&[0x89, b'P', b'N', b'G']));
            assert_eq!(p.height, 1800);
            assert!((1270..=1274).contains(&p.width), "{}", p.width);
        }
    }

    #[test]
    fn dung_khi_khong_nhan_nua_va_anh_qua_lon_thi_ve_nho_lai() {
        let mut count = 0;
        render_pages(sample_pdf(3), 30, usize::MAX, BUDGET, |_| {
            count += 1;
            false
        })
        .unwrap();
        assert_eq!(count, 1);

        // Giới hạn PNG rất nhỏ → vẽ lại nhỏ dần
        let mut small = None;
        render_pages(sample_pdf(1), 1, 2_000, BUDGET, |p| {
            small = Some(p);
            true
        })
        .unwrap();
        assert!(small.unwrap().height < 1800);
    }

    #[test]
    fn file_khong_phai_pdf() {
        assert_eq!(render_pages(b"<html>".to_vec(), 5, usize::MAX, BUDGET, |_| true).unwrap_err(), PdfError::Invalid);
    }
}
