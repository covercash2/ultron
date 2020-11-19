extern crate reqwest;
use chrono::{DateTime, Utc}

// TODO figure out formatting
const GET_ITEM_URL: &str = "https://hacker-news.firebaseio.com/v0/item/{id}"
const TOP_STORIES_URL: &str = "https://hacker-news.firebaseio.com/v0/topstories.json";
const NEW_STORIES_URL: &str = "https://hacker-news.firebaseio.com/v0/newstories.json"; 
const ASK_HN_URL: &str = "https://hacker-news.firebaseio.com/v0/askhn.json"; 
const SHOW_HN_URL: &str = "https://hacker-news.firebaseio.com/v0/showhn.json"; 

#[derive(Debug)]
pub enum HNFeed {
    Top { num_results: i64 } 
    New { num_results: i64 } 
    Ask { num_results: i64 }  
    Show { num_results: i64 } 
} 

/// Represents a single hacker news "Item" by its unique uuid
/// Example Items include submissions, users, comments, etc.
#[derive(Debug)]
pub struct Item {
    pub item_id: u64
    pub type: str
    pub by: str
    pub time: DateTime<Utc>
    pub text: str
    pub dead: bool
    pub parent: u64 // TODO double check this, but I think this is an item_id
    pub kids: [u64] 
    pub url: str
    pub score: i64 // pretty sure this can be negative
    pub title: str
    pub descendants: u64 // Total comment count. "Number of descendants."
    // TODO pub poll: ???
    // TODO this would be cool, but it needs lazy eval 
    //      pub kids: Vec<Item> // list of 
    // TODO pub parts: ???
}

// TODO private by default?
async fn get_item_from_id(item_id: u64) -> [i64] { // TODO dont judge me i didnt actually look up how raw arrays work. or lists. or vectors.
    let hn_response = reqwest::get(GET_ITEM_URL + item_id.to_str())
        .await?
        .text()
        .await? 
    Item {
        item_id: hn_response.item_id
        type: hn_response.type
        by: hn_response.by
        time: hn_response.time
        text: hn_response.text
        dead: hn_response.dead
        parent: hn_response.parent
        kids: hn_response.kids
        url: hn_response.url
        score: hn_response.score
        title: hn_response.title
        descendants: hn_response.descendants
    }
}

async fn get_items_from_ids(ids: Vec<u64>) -> Vec<Item> {
    ids.map((id) -> get_item_from_id(id))
}

async fn get_ids_for_feed(feed: HNFeed) -> Vec<u64> {
    reqwest::get(feed)
        .await?
        .text()
        .await?;
}

pub async fn request_feed(&mut self, feed: Feed) -> str {
    match feed {// TODO add the url parameter for num_results if > 0
        HNFeed::Top { num_results } => {
            get_items_from_ids(
                get_ids_for_feed(feed)
            )
         }
        HNFeed::New{ num_results } => { api_call_pp(NEW_STORIES) }
        HNFeed::Ask{ num_results } => { api_call_pp(ASK_HN) }
        HNFeed::Show{ num_results } => { api_call_pp(SHOW_HN) }
        // Default case
        _ => { }
    }
}