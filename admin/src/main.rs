use std::env;

use clap::Clap;
use dotenv::dotenv;

use db::{self, Db as Database};

/// This doc string acts as a help message when the user runs '--help'
/// as do all doc strings on fields
#[derive(Clap)]
#[clap(version = "0.1", author = "Chris Overcash covercash2@gmail.com")]
struct Opts {
    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    Db(Db),
}

/// A subcommand for controlling testing
#[derive(Clap)]
struct Db {
    /// path to sqlite database file
    db_url: Option<String>,
    #[clap(subcommand)]
    subcmd: DbCommand,
}

#[derive(Clap)]
enum DbCommand {
    Create(Create),
    Read(Read),
    Update(Update),
}

/// insert
#[derive(Clap)]
struct Create {
    #[clap(subcommand)]
    record: Record,
}

#[derive(Clap)]
enum Read {
    AllBalances,
}

#[derive(Clap)]
struct Update {
    #[clap(subcommand)]
    record: Record,
}

#[derive(Clap)]
enum Record {
    /// server_id, channel_id, and user_id record
    ChannelUser(User),
    /// server_id, user_id, balance record
    Balance(Account),
}

#[derive(Clap)]
struct User {
    server_id: u64,
    channel_id: u64,
    user_id: u64,
}

#[derive(Clap)]
struct Account {
    server_id: u64,
    user_id: u64,
    balance: i32,
}

impl Create {
    fn handle(self, db: &Database) {
        match self.record {
            Record::Balance(account) => {
                let server_id = &account.server_id;
                let user_id = &account.user_id;
                let amount = &account.balance;
                db.insert_bank_account(server_id, user_id, amount)
                    .expect("unable to insert balance into db");
                println!("balance created: #{} #{} ${}", server_id, user_id, amount);
            }
            Record::ChannelUser(channel_user) => {
                // create channel user
            }
        }
    }
}

fn main() {
    // load .env file
    dotenv().ok();

    // parse command line opts
    let opts: Opts = Opts::parse();

    // You can handle information about subcommands by requesting their matches by name
    // (as below), requesting just the name used, or both at the same time
    match opts.subcmd {
        SubCommand::Db(db_command) => {
            // get sqlite database file path
            let database_url = db_command
                .db_url
                .ok_or(String::from("no db url specified"))
                .or(env::var("TEST_DB_URL"))
                .or(env::var("PROD_DB_URL"))
                .expect("pass --db_url <path> or set TEST_DB_URL or PROD_DB_URL");

            if opts.verbose > 0 {
                println!("using database file: {}", database_url);
            }

            let db = db::Db::open(&database_url).expect("unable to open database");

            match db_command.subcmd {
                DbCommand::Create(create_command) => create_command.handle(&db),
                DbCommand::Read(read) => match read {
                    Read::AllBalances => {
                        let accounts = db.show_accounts().expect("unable to retrieve accounts");
                        if accounts.len() == 0 {
                            println!("no accounts returned");
                        }
                        for account in accounts {
                            let server_id = account
                                .server_id()
                                .expect("unable to parse server id from db output");
                            let user_id = account
                                .user_id()
                                .expect("unable to parse user id from db output");
                            let balance = account.balance;
                            println!(
                                "server_id: {}, user_id: {}, balance: {}",
                                server_id, user_id, balance
                            )
                        }
                    }
                },
                DbCommand::Update(update) => {
                    match update.record {
                        Record::ChannelUser(user) => {
                            // shouldn't happen
                        }
                        Record::Balance(balance) => {
                            let server_id = &balance.server_id;
                            let user_id = &balance.user_id;
                            let new_balance = &balance.balance;
                            let records = db
                                .update_balance(server_id, user_id, new_balance)
                                .expect("unable to update balance");
                            println!("updated {} record to ${}", records, new_balance);
                        }
                    }
                }
            }
        }
    }
}
