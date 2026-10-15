use serde::{Deserialize, Deserializer};

use crate::application::spec_list::{KindFilter, ListQuery, Order};

/// The search, filters and order of the overview, as kept in the url.
///
/// Unknown values fall back to the default, so an edited url never fails.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub(super) struct ListParams {
    #[serde(rename = "q")]
    pub search: String,
    #[serde(deserialize_with = "lenient_kind")]
    pub kind: KindFilter,
    #[serde(deserialize_with = "lenient_order")]
    pub order: Order,
}

fn lenient_kind<'de, D: Deserializer<'de>>(deserializer: D) -> Result<KindFilter, D::Error> {
    Ok(match String::deserialize(deserializer)?.as_str() {
        "published" => KindFilter::Published,
        "draft" => KindFilter::Draft,
        _ => KindFilter::All,
    })
}

fn lenient_order<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Order, D::Error> {
    Ok(match String::deserialize(deserializer)?.as_str() {
        "oldest" => Order::Oldest,
        _ => Order::Newest,
    })
}

impl ListParams {
    pub fn query(&self) -> ListQuery {
        ListQuery {
            search: self.search.clone(),
            kind: self.kind,
            order: self.order,
        }
    }

    /// The `value` of the option of the kind select.
    pub fn kind_value(&self) -> &'static str {
        match self.kind {
            KindFilter::All => "all",
            KindFilter::Published => "published",
            KindFilter::Draft => "draft",
        }
    }

    /// The `value` of the option of the order select.
    pub fn order_value(&self) -> &'static str {
        match self.order {
            Order::Newest => "newest",
            Order::Oldest => "oldest",
        }
    }

    /// The url query of this list, without the defaults so urls stay short.
    pub fn query_string(&self) -> String {
        self.query_string_with_page_token(None)
    }

    /// The url query of the page of this list starting at `page_token`.
    pub fn query_string_with_page_token(&self, page_token: Option<&str>) -> String {
        let search = self.search.trim();
        let mut pairs = Vec::new();
        if !search.is_empty() {
            pairs.push(("q", search));
        }
        if self.kind != KindFilter::default() {
            pairs.push(("kind", self.kind_value()));
        }
        if self.order != Order::default() {
            pairs.push(("order", self.order_value()));
        }
        if let Some(page_token) = page_token {
            pairs.push(("page_token", page_token));
        }
        serde_urlencoded::to_string(pairs).expect("string pairs always encode")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(query: &str) -> ListParams {
        serde_urlencoded::from_str(query).unwrap()
    }

    #[test]
    fn parses_the_url_query() {
        assert_eq!(
            parse("q=bank&kind=draft&order=oldest"),
            ListParams {
                search: "bank".into(),
                kind: KindFilter::Draft,
                order: Order::Oldest,
            }
        );
    }

    #[test]
    fn falls_back_to_the_defaults_for_missing_and_unknown_values() {
        assert_eq!(parse(""), ListParams::default());
        assert_eq!(parse("kind=everything&order=random"), ListParams::default());
    }

    #[test]
    fn leaves_the_defaults_out_of_the_query_string() {
        assert_eq!(parse("q=+&kind=all&order=newest").query_string(), "");
        assert_eq!(
            parse("q=a b&kind=published&order=oldest").query_string_with_page_token(Some("7")),
            "q=a+b&kind=published&order=oldest&page_token=7"
        );
    }

    #[test]
    fn converts_to_the_query_of_the_application() {
        assert_eq!(
            parse("q=bank&kind=published").query(),
            ListQuery {
                search: "bank".into(),
                kind: KindFilter::Published,
                order: Order::Newest,
            }
        );
    }
}
