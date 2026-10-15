use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    application::contract::{ListQuery, SortDirection, SpecOrderField},
    domain::{
        entity::spec::{Spec, draft::Draft},
        search::matches_search,
    },
};

pub mod draft_repository;
pub mod spec_repository;

/// What the in-memory repositories need to know about an item to list it.
trait Listable {
    fn id(&self) -> &Uuid;
    fn title(&self) -> &str;
    fn abbreviation(&self) -> &str;
    fn created_at(&self) -> &DateTime<Utc>;
}

impl Listable for Spec {
    fn id(&self) -> &Uuid {
        Spec::id(self)
    }
    fn title(&self) -> &str {
        Spec::title(self)
    }
    fn abbreviation(&self) -> &str {
        Spec::abbreviation(self)
    }
    fn created_at(&self) -> &DateTime<Utc> {
        Spec::created_at(self)
    }
}

impl Listable for Draft {
    fn id(&self) -> &Uuid {
        Draft::id(self)
    }
    fn title(&self) -> &str {
        Draft::title(self)
    }
    fn abbreviation(&self) -> &str {
        Draft::abbreviation(self)
    }
    fn created_at(&self) -> &DateTime<Utc> {
        Draft::created_at(self)
    }
}

/// Applies the search, order, cursor and limit of the query to the items.
fn select_page<'a, T: Listable + Clone + 'a>(
    items: impl Iterator<Item = &'a T>,
    query: &ListQuery,
) -> Vec<T> {
    // Ordering by a new field needs its own sort key; this fails to compile until it has one.
    let SpecOrderField::CreatedAt = query.order.field;
    let key = |item: &T| (*item.created_at(), *item.id());

    let mut page: Vec<&T> = items
        .filter(|item| {
            query
                .search
                .as_deref()
                .is_none_or(|term| matches_search(item.title(), item.abbreviation(), term))
        })
        .filter(|item| {
            query.after.is_none_or(|cursor| {
                let cursor = (cursor.created_at, cursor.id);
                match query.order.direction {
                    SortDirection::Ascending => key(item) > cursor,
                    SortDirection::Descending => key(item) < cursor,
                }
            })
        })
        .collect();

    page.sort_by_key(|item| key(item));
    if query.order.direction == SortDirection::Descending {
        page.reverse();
    }

    page.into_iter().take(query.limit).cloned().collect()
}
