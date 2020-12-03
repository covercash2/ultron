use std::env;
use dotenv::dotenv;
use db;

fn main() {
    dotenv().ok();
    let database_url = env::var("PROD_DB_URL").or(env::var("TEST_DB_URL"))
        .expect("set TEST_DB_URL or PROD_DB_URL");
    let db = db::Db::open(&database_url).expect("unable to open database");

    let results = db.show_accounts().expect("unable to load accounts");

    println!("Displaying {} accounts", results.len());
    for account in results {
        println!("{:?}", account.user_id());
        println!("----------\n");
        println!("{:?}", account.balance);
    }
}
