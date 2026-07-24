/// Normalized pagination values for one-based HTTP query parameters.
///
/// SeaORM pages are zero-based, so handlers should use [`Self::offset`] when
/// passing a client-supplied page value to `fetch_page`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pagination {
    page: u64,
    size: u64,
}

impl Pagination {
    pub const MAX_PAGE_SIZE: u64 = 100;

    pub fn new(page: Option<u64>, size: Option<u64>, default_size: u64) -> Self {
        Self {
            page: page.unwrap_or(1).max(1),
            size: size.unwrap_or(default_size).clamp(1, Self::MAX_PAGE_SIZE),
        }
    }

    pub fn page(self) -> u64 {
        self.page
    }

    pub fn size(self) -> u64 {
        self.size
    }

    pub fn offset(self) -> u64 {
        self.page - 1
    }

    pub fn total_pages(self, total: u64) -> u64 {
        total.div_ceil(self.size)
    }
}

#[cfg(test)]
mod tests {
    use super::Pagination;

    #[test]
    fn uses_defaults_for_missing_values() {
        let pagination = Pagination::new(None, None, 20);

        assert_eq!(pagination.page(), 1);
        assert_eq!(pagination.size(), 20);
        assert_eq!(pagination.offset(), 0);
        assert_eq!(pagination.total_pages(0), 0);
        assert_eq!(pagination.total_pages(21), 2);
    }

    #[test]
    fn normalizes_invalid_and_excessive_values() {
        let zero_page = Pagination::new(Some(0), Some(0), 20);
        let oversized_page = Pagination::new(Some(4), Some(101), 20);

        assert_eq!(zero_page.page(), 1);
        assert_eq!(zero_page.size(), 1);
        assert_eq!(oversized_page.offset(), 3);
        assert_eq!(oversized_page.size(), Pagination::MAX_PAGE_SIZE);
    }
}
