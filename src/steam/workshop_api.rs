use anyhow::Result;

pub const APP_ID: u64 = 294100;

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";

// ─── Типы ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WorkshopItem {
    pub id: u64,
    pub title: String,
    pub author: String,
    pub preview_url: String,
    pub subscribers: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortOrder {
    Trending,
    Latest,
    MostSubscribed,
    RecentlyUpdated,
}

impl SortOrder {
    pub fn as_param(self) -> &'static str {
        match self {
            Self::Trending        => "trend",
            Self::Latest          => "mostrecent",
            Self::MostSubscribed  => "totaluniquesubscriptions",
            Self::RecentlyUpdated => "lastupdated",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Trending        => "Популярные",
            Self::Latest          => "Новые",
            Self::MostSubscribed  => "По подпискам",
            Self::RecentlyUpdated => "Обновлённые",
        }
    }

    pub const ALL: [SortOrder; 4] = [
        Self::Trending,
        Self::Latest,
        Self::MostSubscribed,
        Self::RecentlyUpdated,
    ];
}

// ─── Запрос к Steam Workshop ─────────────────────────────────────────────────

/// Возвращает список модов и флаг наличия следующей страницы.
pub fn fetch_workshop_page(
    query: &str,
    page: u32,
    sort: SortOrder,
) -> Result<(Vec<WorkshopItem>, bool)> {
    let encoded = url_encode(query);
    let url = format!(
        "https://steamcommunity.com/workshop/browse/?appid={}&searchtext={}&section=readytouseitems&actualsort={}&p={}",
        APP_ID, encoded, sort.as_param(), page
    );

    let html = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .set("Accept-Language", "en-US,en;q=0.9")
        .call()
        .map_err(|e| anyhow::anyhow!("HTTP ошибка: {e}"))?
        .into_string()?;

    parse_page(&html)
}

fn parse_page(html: &str) -> Result<(Vec<WorkshopItem>, bool)> {
    use scraper::{Html, Selector};

    let doc = Html::parse_document(html);

    let item_sel   = Selector::parse(".workshopItem").unwrap();
    let link_sel   = Selector::parse("a.ugc").unwrap();
    let img_sel    = Selector::parse(".workshopItemPreviewImage").unwrap();
    let title_sel  = Selector::parse(".workshopItemTitle").unwrap();
    let author_sel = Selector::parse(".workshopItemAuthorName a").unwrap();
    let stat_sel   = Selector::parse(".workshopItemStat").unwrap();
    let next_sel   = Selector::parse("a.pagebtn").unwrap();

    let mut items = Vec::new();

    for el in doc.select(&item_sel) {
        let href = el
            .select(&link_sel)
            .next()
            .and_then(|a| a.value().attr("href"))
            .unwrap_or("");

        let id: u64 = href
            .split("id=")
            .nth(1)
            .and_then(|s| s.split('&').next())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        if id == 0 {
            continue;
        }

        let preview_url = el
            .select(&img_sel)
            .next()
            .and_then(|img| img.value().attr("src"))
            .unwrap_or("")
            .to_string();

        let title = el
            .select(&title_sel)
            .next()
            .map(|n| n.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let author = el
            .select(&author_sel)
            .next()
            .map(|n| n.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let subscribers = el
            .select(&stat_sel)
            .last()
            .map(|n| {
                n.text()
                    .collect::<String>()
                    .split_whitespace()
                    .last()
                    .unwrap_or("")
                    .to_string()
            })
            .unwrap_or_default();

        items.push(WorkshopItem { id, title, author, preview_url, subscribers });
    }

    let has_next = doc.select(&next_sel).any(|btn| btn.text().collect::<String>().trim() == ">");

    Ok((items, has_next))
}

// ─── Вспомогательное ──────────────────────────────────────────────────────────

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b' '                                          => out.push('+'),
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~'                  => out.push(b as char),
            _                                             => { let _ = out.write_fmt(format_args!("%{b:02X}")); }
        }
    }
    out
}

// Need fmt::Write for the macro
use std::fmt::Write as _;
