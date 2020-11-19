use reqwest;
use request;
use log;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use tokio::stream::{self, StreamExt};

use crate::error::Result;

const TOP_STORIES_URL: &str = "https://hacker-news.firebaseio.com/v0/topstories.json";
const NEW_STORIES_URL: &str = "https://hacker-news.firebaseio.com/v0/newstories.json"; 
const ASK_HN_URL: &str = "https://hacker-news.firebaseio.com/v0/askhn.json"; 
const SHOW_HN_URL: &str = "https://hacker-news.firebaseio.com/v0/showhn.json"; 

#[derive(Debug)]
enum HNFeed {
    Top { num_results: i64 },
    New { num_results: i64 },
    Ask { num_results: i64 },
    Show { num_results: i64 } 
} 

/// Represents a single hacker news "Item" by its unique uuid
/// Example Items include submissions, users, comments, etc.
#[derive(Debug, Deserialize)]
struct Item {
    item_id: u64,
    hn_type: String,
    by: String,
    time: DateTime<Utc>,
    text: String,
    dead: bool,
    parent: u64,
    url: String,
    score: i64,
    title: String,
    descendants: u64, 
}

async fn get_item_from_id(item_id: &u64) -> Result<Item, reqwest::Error> { 
    let hn_response = reqwest::get(format!("https://hacker-news.firebaseio.com/v0/item/{}", item_id))
        .await?
        .json()
        .await?; 
    Ok(
        Item {
            item_id: hn_response.item_id,
            hn_type: hn_response.hn_type,
            by: hn_response.by,
            time: hn_response.time,
            text: hn_response.text,
            dead: hn_response.dead,
            parent: hn_response.parent,
            url: hn_response.url,
            score: hn_response.score,
            title: hn_response.title,
            descendants: hn_response.descendants,
        }
    )
}

async fn get_items_from_ids(ids: Vec<u64>) -> Result<Vec<u64>> { 
    ids.map(|id| get_item_from_id(id));
}

pub async fn request_feed(&mut self, feed: Feed) -> str {
    match feed {// TODO add the url parameter for num_results if > 0
        HNFeed::Top { num_results } => {
            get_items_from_ids(
                get_ids_for_feed(feed)
            )
         }
        HNFeed::New{ num_results } => { api_call_pp(NEW_STORIES) 
        HNFeed::Ask{ num_results } => { api_call_pp(ASK_HN) }
        HNFeed::Show{ num_results } => { api_call_pp(SHOW_HN) }
        // Default case
        _ => { }
    }
}