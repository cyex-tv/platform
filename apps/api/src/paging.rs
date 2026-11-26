//! The one page-size rule (ADR 0008, API review 2026-11-26, item 3).
//!
//! Every paged list, GraphQL field or REST route, reads its page size through [`PageParams`]:
//! `perPage` (`per_page` over REST, `limit` for the feeds) is 1 to **100**, **50** when it is
//! not given. The few lists that always had other numbers keep them in [`OVERRIDES`], the only
//! place a size differs. The schema descriptions repeat the figures; a test fails when one drifts.

/// The deepest numbered page a list serves. A page past it is empty (the total is still
/// counted), so a huge page number never makes the database step over that many rows. Cursor
/// paging (ADR 0030) has no such limit.
pub const MAX_PAGE: u32 = 1000;

/// Default and largest page size of a list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageParams {
    /// Rows when the client names no size.
    pub default: u32,
    /// The most rows a client may ask for; more is clamped, 0 is 1.
    pub max: u32,
}

impl PageParams {
    /// The rule: 50 rows, at most 100.
    pub const STANDARD: Self = Self::new(50, 100);

    pub const fn new(default: u32, max: u32) -> Self {
        Self { default, max }
    }

    /// What the schema descriptions say, for example `` `perPage` is 1 to 100, default 50``.
    pub fn describe(&self, argument: &str) -> String {
        format!(
            "`{argument}` is 1 to {}, default {}",
            self.max, self.default
        )
    }

    /// `per_page` clamped to 1 to [`Self::max`], or the default when `None`.
    pub fn size(&self, per_page: Option<u32>) -> u32 {
        per_page.unwrap_or(self.default).clamp(1, self.max)
    }

    /// `page` (from 1, default 1) and `per_page` as the rows to read.
    pub fn paging(&self, page: Option<u32>, per_page: Option<u32>) -> Paging {
        let per_page = self.size(per_page);
        let page = page.unwrap_or(1).max(1);
        if page > MAX_PAGE {
            return Paging {
                page,
                per_page,
                limit: 0,
                offset: 0,
            };
        }
        Paging {
            page,
            per_page,
            limit: i64::from(per_page),
            offset: i64::from(page - 1) * i64::from(per_page),
        }
    }
}

/// One page of a list: the page and size as answered, and the rows to read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Paging {
    /// From 1, as asked.
    pub page: u32,
    /// Clamped to 1 to the list's maximum.
    pub per_page: u32,
    /// Rows to read: `per_page`, or 0 past [`MAX_PAGE`].
    pub limit: i64,
    pub offset: i64,
}

impl Paging {
    /// Past [`MAX_PAGE`]: no rows are read.
    pub fn past_end(&self) -> bool {
        self.page > MAX_PAGE
    }
}

/// Every paged list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paged {
    SearchEmotes,
    SearchAccounts,
    /// `setEmotes` and `GET /v1/sets/{id}/emotes`.
    SetEmotes,
    MyEmotes,
    AccountEmotes,
    ReviewQueue,
    /// The moderation lists that page by number.
    ModerationQueue,
    ReportQueue,
    MyOrders,
    Orders,
    CodeBatches,
    MyFavourites,
    CosmeticEntitlements,
    /// The channels using an emote (`emoteChannels`).
    EmoteChannels,
    /// The first members of a set shown on its card (`preview`).
    SetPreview,
    /// The activity feeds (`limit`).
    Activity,
    /// The audit log (`limit`).
    Audit,
}

/// The lists whose page size is not [`PageParams::STANDARD`], and why. Everything else uses the
/// rule. Changing a number here changes behaviour for clients (the extension and the deck read
/// these), so a number only moves with an ADR.
pub const OVERRIDES: &[(Paged, PageParams)] = &[
    // Account search: short pages, and the rate limit counts every call.
    (Paged::SearchAccounts, PageParams::new(20, 50)),
    // A set page: chat clients read whole sets, which can hold up to 1.000 emotes.
    (Paged::SetEmotes, PageParams::new(120, 200)),
    // A profile or emote page shows the latest few dozen feed lines.
    (Paged::Activity, PageParams::new(30, 100)),
    // The channels on an emote page, and the members shown on a set card.
    (Paged::EmoteChannels, PageParams::new(8, 50)),
    (Paged::SetPreview, PageParams::new(12, 50)),
];

impl Paged {
    /// This list's page size.
    pub fn params(self) -> PageParams {
        OVERRIDES
            .iter()
            .find(|(list, _)| *list == self)
            .map_or(PageParams::STANDARD, |(_, params)| *params)
    }

    /// `page` and `per_page` as the rows to read.
    pub fn paging(self, page: Option<u32>, per_page: Option<u32>) -> Paging {
        self.params().paging(page, per_page)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paging_clamps_and_stops_past_the_last_page() {
        let rule = PageParams::STANDARD;
        let p = rule.paging(None, None);
        assert_eq!((p.page, p.per_page, p.limit, p.offset), (1, 50, 50, 0));
        let p = rule.paging(Some(0), Some(500));
        assert_eq!((p.page, p.per_page, p.limit, p.offset), (1, 100, 100, 0));
        let p = rule.paging(Some(3), Some(0));
        assert_eq!((p.page, p.per_page, p.limit, p.offset), (3, 1, 1, 2));
        let p = rule.paging(Some(MAX_PAGE), Some(100));
        assert_eq!(p.offset, 99_900);
        assert!(!p.past_end());
        let p = rule.paging(Some(1_000_000_000), Some(100));
        assert!(p.past_end());
        assert_eq!(
            (p.page, p.per_page, p.limit, p.offset),
            (1_000_000_000, 100, 0, 0)
        );
    }

    #[test]
    fn the_rule_is_50_and_100_and_only_the_table_differs() {
        assert_eq!(PageParams::STANDARD, PageParams::new(50, 100));
        assert_eq!(Paged::SearchEmotes.params(), PageParams::STANDARD);
        assert_eq!(Paged::Audit.params(), PageParams::STANDARD);
        // The numbers each list had before the rule: no behaviour change.
        assert_eq!(Paged::SearchAccounts.params(), PageParams::new(20, 50));
        assert_eq!(Paged::SetEmotes.params(), PageParams::new(120, 200));
        assert_eq!(Paged::Activity.params(), PageParams::new(30, 100));
        assert_eq!(Paged::EmoteChannels.params(), PageParams::new(8, 50));
        assert_eq!(Paged::SetPreview.params(), PageParams::new(12, 50));
        // A list appears in the table once, and every entry really differs from the rule.
        for (i, (a, _)) in OVERRIDES.iter().enumerate() {
            assert!(
                OVERRIDES[i + 1..].iter().all(|(b, _)| b != a),
                "{a:?} twice"
            );
        }
        assert!(OVERRIDES.iter().all(|(_, p)| *p != PageParams::STANDARD));
    }

    /// The description right before `field(` in the SDL, whitespace folded.
    fn description_of(sdl: &str, field: &str) -> String {
        let at = sdl
            .find(&format!("\n\t{field}("))
            .unwrap_or_else(|| panic!("no field {field} in the schema"));
        let head = &sdl[..at];
        let end = head.rfind("\"\"\"").expect("a description");
        let start = head[..end].rfind("\"\"\"").expect("a description");
        head[start + 3..end]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn the_schema_descriptions_say_each_lists_page_size() {
        let sdl = crate::gql::sdl();
        let lists = [
            ("searchEmotes", Paged::SearchEmotes, "perPage"),
            ("searchAccounts", Paged::SearchAccounts, "perPage"),
            ("setEmotes", Paged::SetEmotes, "perPage"),
            ("myEmotes", Paged::MyEmotes, "perPage"),
            ("accountEmotes", Paged::AccountEmotes, "perPage"),
            ("reviewQueue", Paged::ReviewQueue, "perPage"),
            ("reportQueue", Paged::ReportQueue, "perPage"),
            ("myOrders", Paged::MyOrders, "perPage"),
            ("orders", Paged::Orders, "perPage"),
            ("codeBatches", Paged::CodeBatches, "perPage"),
            ("myFavourites", Paged::MyFavourites, "perPage"),
            (
                "cosmeticEntitlements",
                Paged::CosmeticEntitlements,
                "perPage",
            ),
            ("auditLog", Paged::Audit, "limit"),
            ("accountActivity", Paged::Activity, "limit"),
            ("emoteActivity", Paged::Activity, "limit"),
            ("emoteChannels", Paged::EmoteChannels, "limit"),
        ];
        for (field, list, argument) in lists {
            let said = list.params().describe(argument);
            let doc = description_of(&sdl, field);
            assert!(doc.contains(&said), "{field}: expected {said:?} in {doc:?}");
        }
    }
}
