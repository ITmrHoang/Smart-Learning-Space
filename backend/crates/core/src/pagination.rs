//! Phân trang dùng chung cho các API danh sách.

use serde::Serialize;

pub const DEFAULT_PAGE_SIZE: u32 = 20;
pub const MAX_PAGE_SIZE: u32 = 100;

/// Trang đã chuẩn hóa: `page` ≥ 1, `page_size` trong khoảng 1..=100.
#[derive(Debug, Clone, Copy)]
pub struct PageRequest {
    pub page: u32,
    pub page_size: u32,
}

impl PageRequest {
    pub fn new(page: Option<u32>, page_size: Option<u32>) -> Self {
        Self {
            page: page.unwrap_or(1).max(1),
            page_size: page_size.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, MAX_PAGE_SIZE),
        }
    }

    pub fn limit(&self) -> i64 {
        i64::from(self.page_size)
    }

    pub fn offset(&self) -> i64 {
        i64::from(self.page - 1) * i64::from(self.page_size)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, total: i64, req: PageRequest) -> Self {
        Self { items, total, page: req.page, page_size: req.page_size }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chuan_hoa_trang() {
        let p = PageRequest::new(Some(0), Some(1000));
        assert_eq!((p.page, p.page_size), (1, MAX_PAGE_SIZE));
        let p = PageRequest::new(Some(3), Some(10));
        assert_eq!((p.limit(), p.offset()), (10, 20));
        let p = PageRequest::new(None, None);
        assert_eq!((p.page, p.page_size), (1, DEFAULT_PAGE_SIZE));
    }
}
