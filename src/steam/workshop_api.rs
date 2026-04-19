use anyhow::Result;
use std::fmt::Write as _;

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
}

#[derive(Debug, Clone)]
pub struct CollectionItem {
    pub id: u64,
    pub title: String,
    pub author: String,
    pub preview_url: String,
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
        "https://steamcommunity.com/workshop/browse/?appid={}&searchtext={}&section=readytouseitems&browsesort={}&p={}",
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

        items.push(WorkshopItem { id, title, author, preview_url });
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

// ─── Сборки (Collections) ────────────────────────────────────────────────────

pub fn fetch_collections_page(
    query: &str,
    page: u32,
    sort: SortOrder,
) -> Result<(Vec<CollectionItem>, bool)> {
    let search_part = if query.is_empty() {
        String::new()
    } else {
        format!("&searchtext={}", url_encode(query))
    };
    let url = format!(
        "https://steamcommunity.com/workshop/browse/?appid={}{}&section=collections&browsesort={}&p={}",
        APP_ID, search_part, sort.as_param(), page
    );
    let html = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .set("Accept-Language", "en-US,en;q=0.9")
        .call()
        .map_err(|e| anyhow::anyhow!("HTTP ошибка: {e}"))?
        .into_string()?;
    parse_collections_page(&html)
}

fn parse_collections_page(html: &str) -> Result<(Vec<CollectionItem>, bool)> {
    use scraper::{Html, Selector};
    let doc = Html::parse_document(html);
    // Collections page wraps everything in <a class="workshopItemCollection ugc ...">
    // The <a> is the parent of .workshopItem, not a child — so we select the <a> directly.
    let item_sel   = Selector::parse("a.workshopItemCollection").unwrap();
    let img_sel    = Selector::parse(".workshopItemPreviewImage").unwrap();
    let title_sel  = Selector::parse(".workshopItemTitle").unwrap();
    let author_sel = Selector::parse(".workshopItemAuthorName").unwrap();
    let next_sel   = Selector::parse("a.pagebtn").unwrap();

    let mut items = Vec::new();
    for el in doc.select(&item_sel) {
        let id: u64 = el.value().attr("data-publishedfileid")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        if id == 0 { continue; }
        let preview_url = el.select(&img_sel).next()
            .and_then(|img| img.value().attr("src")).unwrap_or("").to_string();
        let title = el.select(&title_sel).next()
            .map(|n| n.text().collect::<String>().trim().to_string()).unwrap_or_default();
        let author = el.select(&author_sel).next()
            .map(|n| n.text().collect::<String>().trim().to_string()).unwrap_or_default();
        items.push(CollectionItem { id, title, author, preview_url });
    }
    let has_next = doc.select(&next_sel).any(|btn| btn.text().collect::<String>().trim() == ">");
    Ok((items, has_next))
}

/// Возвращает (название сборки, список модов).
pub fn fetch_collection_mods(collection_id: u64) -> Result<(String, Vec<WorkshopItem>)> {
    use std::fmt::Write as _;

    // Шаг 1: получить дочерние ID через Steam Web API (ключ не нужен)
    let body = format!("collectioncount=1&publishedfileids[0]={}", collection_id);
    let resp = ureq::post("https://api.steampowered.com/ISteamRemoteStorage/GetCollectionDetails/v1/")
        .set("Content-Type", "application/x-www-form-urlencoded")
        .set("User-Agent", USER_AGENT)
        .send_string(&body)
        .map_err(|e| anyhow::anyhow!("HTTP ошибка: {e}"))?
        .into_string()?;

    let json: serde_json::Value = serde_json::from_str(&resp)?;
    let detail = &json["response"]["collectiondetails"][0];
    let coll_title = detail["title"].as_str().unwrap_or("Collection").to_string();

    let children = detail["children"].as_array()
        .ok_or_else(|| anyhow::anyhow!("Сборка пустая или недоступна"))?;

    let ids: Vec<u64> = children.iter()
        .filter_map(|c| c["publishedfileid"].as_str())
        .filter_map(|s| s.parse::<u64>().ok())
        .collect();

    if ids.is_empty() {
        return Ok((coll_title, Vec::new()));
    }

    // Шаг 2: получить детали каждого мода
    let mut body = format!("itemcount={}", ids.len());
    for (i, id) in ids.iter().enumerate() {
        let _ = write!(body, "&publishedfileids[{}]={}", i, id);
    }

    let resp = ureq::post("https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1/")
        .set("Content-Type", "application/x-www-form-urlencoded")
        .set("User-Agent", USER_AGENT)
        .send_string(&body)
        .map_err(|e| anyhow::anyhow!("HTTP ошибка: {e}"))?
        .into_string()?;

    let json: serde_json::Value = serde_json::from_str(&resp)?;
    let details = json["response"]["publishedfiledetails"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Нет данных о модах сборки"))?;

    let items = details.iter()
        .filter_map(|d| {
            let id = d["publishedfileid"].as_str()?.parse::<u64>().ok()?;
            let title = d["title"].as_str().unwrap_or("").to_string();
            let preview_url = d["preview_url"].as_str().unwrap_or("").to_string();
            Some(WorkshopItem { id, title, author: String::new(), preview_url })
        })
        .collect();

    Ok((coll_title, items))
}
