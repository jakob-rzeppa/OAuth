# Spec Provider

The server holding and managing the specs.

## ListSpecs

Returns one page of published specs and/or drafts as overviews.

- **Filter:** `filter.kind` selects `PUBLISHED`, `DRAFT` or `BOTH` (the default). New filters are added as fields of `ListSpecsFilter`.
- **Search:** `search` matches specs whose title or abbreviation contains the text, ignoring case. The spec-provider decides how to match, clients only send the text.
- **Order:** `order` defaults to `created_at` descending.
- **Pagination:** `page_size` defaults to 50 and is capped at 100. Pass the `next_page_token` of a response as the `page_token` of the next request, it is empty on the last page. Keep filter, search and order the same while paging.

> **Note:** There is no authentication yet, so `ListSpecs` returns drafts of every visibility, including `PRIVATE` and `HIDDEN` ones. This has to change once auth is implemented: only return the drafts the caller is allowed to see.
