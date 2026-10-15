use chrono::{DateTime, Utc};
use uuid::Uuid;

pub mod draft_repository;
pub mod spec_repository;

#[derive(Debug, PartialEq)]
pub enum RepositoryError {
    /// `register` was called with an ID that already exists.
    AlreadyExists,
    /// `save` or `delete_by_id` was called with an ID that doesn't exist.
    NotFound,
    DatabaseError,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SortDirection {
    Ascending,
    #[default]
    Descending,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SpecOrderField {
    #[default]
    CreatedAt,
}

/// How listed specs and drafts are ordered. Newest first by default.
///
/// Ties are broken by ID in the same direction, so the order is total and stable.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpecOrder {
    pub field: SpecOrderField,
    pub direction: SortDirection,
}

/// The position of an item in a listing. The next page starts after it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageCursor {
    pub created_at: DateTime<Utc>,
    pub id: Uuid,
}

/// Selects one page of specs or drafts.
#[derive(Clone, Debug)]
pub struct ListQuery {
    /// Only items whose title or abbreviation match this term. Matching is up to the repository.
    pub search: Option<String>,
    pub order: SpecOrder,
    /// Only items that come after this position in the order.
    pub after: Option<PageCursor>,
    /// The maximum number of items to return.
    pub limit: usize,
}
