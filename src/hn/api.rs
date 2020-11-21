use reqwest;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::error::Result;

const TOP_STORIES_URL: &str = "https://hacker-news.firebaseio.com/v0/topstories.json";
// const NEW_STORIES_URL: &str = "https://hacker-news.firebaseio.com/v0/newstories.json";
// const ASK_HN_URL: &str = "https://hacker-news.firebaseio.com/v0/askhn.json";
// const SHOW_HN_URL: &str = "https://hacker-news.firebaseio.com/v0/showhn.json";

// #[derive(Debug)]
// enum HNFeed {
//     Top { num_results: i64 },
//     New { num_results: i64 },
//     Ask { num_results: i64 },
//     Show { num_results: i64 }
// }

/// Represents a single hacker news "Item" by its unique uuid
/// Example Items include submissions, users, comments, etc.
#[derive(Debug, Deserialize)]
struct Item {
    item_id: u64,
    hn_type: String,
    by: String,
    time: DateTime<Utc>,
    text: String,
    url: String,
    score: i64,
    title: String,
    descendants: u64, // dead: bool, // parent: u64,
}

async fn get_item_from_id(item_id: &u64) -> Result<Item> {
    let hn_response = reqwest::get(format!(
        "https://hacker-news.firebaseio.com/v0/item/{}",
        item_id
    ))
    .await?
    .json()
    .await?;
    Ok(Item {
        item_id: hn_response.item_id,
        hn_type: hn_response.hn_type,
        by: hn_response.by,
        time: hn_response.time,
        text: hn_response.text,
        url: hn_response.url,
        score: hn_response.score,
        title: hn_response.title,
        descendants: hn_response.descendants,
    })
}

pub async fn hn_top_links() -> Result<String> {
    Ok(
        reqwest::get("https://hacker-news.firebaseio.com/v0/topstories.json")
            .await?
            .text()
            .await?,
    )
}
