use std::env;

use clap::Clap;
use dotenv::dotenv;

use db::{self, Db as Database, model::BankAccount, model::{ChannelUser, Item}};

/// This doc string acts as a help message when the user runs '--help'
/// as do all doc strings on fields
#[derive(Clap)]
#[clap(version = "0.1", author = "Chris Overcash covercash2@gmail.com")]
struct Opts {
    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
    /// subcommands
    #[clap(subcommand)]
    subcmd: SubCommand,
}

/// subcommands
#[derive(Clap)]
enum SubCommand {
    /// database commands
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

/// a command for the database
#[derive(Clap)]
enum DbCommand {
    /// insert a record into the database
    Create(Create),
    /// read from the database
    Read(Read),
    /// update the database
    Update(Update),
}

/// insert a record into the database
#[derive(Clap)]
struct Create {
    #[clap(subcommand)]
    record: Record,
}

#[derive(Clap)]
struct Read {
    #[clap(subcommand)]
    op: ReadOp,
}

/// read from the database
#[derive(Clap)]
enum ReadOp {
    /// show all balances from all servers
    AllBalances,
    /// show user balance
    UserBalance { server_id: u64, user_id: u64 },
    /// show all users and their associated channels
    AllChannelUsers,
    /// show users in a channel
    ChannelUsers { server_id: u64, channel_id: u64 },
    /// show balances for users in a channel
    ChannelUserBalances { server_id: u64, channel_id: u64 },
    /// show all items
    AllItems,
}

/// update a database
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

/// a user in the channel user log
#[derive(Clap)]
struct User {
    server_id: u64,
    channel_id: u64,
    user_id: u64,
}

/// a user bank account
#[derive(Clap)]
struct Account {
    server_id: u64,
    user_id: u64,
    balance: i64,
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
                let server_id = &channel_user.server_id;
                let channel_id = &channel_user.channel_id;
                let user_id = &channel_user.user_id;
                db.insert_channel_user(server_id, channel_id, user_id)
                    .expect("unable to insert channel user");
                println!(
                    "user inserted: #s{} #c{} #u{}",
                    server_id, channel_id, user_id
                );
            }
        }
    }
}

impl ReadOp {
    fn handle(self, db: &Database) {
        match self {
            ReadOp::AllBalances => {
                let accounts = db.show_accounts().expect("unable to retrieve accounts");
                print_accounts(accounts);
            }
            ReadOp::AllChannelUsers => {
                let users = db
                    .show_channel_users()
                    .expect("unable to retrieve channel users");
                print_channel_users(users);
            }
            ReadOp::ChannelUsers {
                server_id,
                channel_id,
            } => {
                let users = db
                    .channel_users(&server_id, &channel_id)
                    .expect("unable to get channel users from db");
                print_channel_users(users);
            }
            ReadOp::ChannelUserBalances {
                server_id,
                channel_id,
            } => {
                let accounts = db
                    .channel_user_balances(&server_id, &channel_id)
                    .expect("unable to get user accounts from db");
                print_accounts(accounts);
            }
            ReadOp::UserBalance { server_id, user_id } => {
                let account = db
                    .user_account(&server_id, &user_id)
                    .expect("unable to get user balance");
                print_account(account);
            }
            ReadOp::AllItems => {
		let items = db.all_items()
		    .expect("unable to get items");
		print_items(items);
	    }
        }
    }
}

fn print_items(items: Vec<Item>) {
    if items.len() == 0 {
        println!("no items retrieved");
    }
    for item in items {
	println!("{} {} {}:\n{}", item.emoji, item.name, item.price, item.description);
    }
}

fn print_channel_users(users: Vec<ChannelUser>) {
    if users.len() == 0 {
        println!("no users retrieved");
    }
    for user in users {
        let server_id = user
            .server_id()
            .expect("unable to parse server id from db output");
        let channel_id = user
            .channel_id()
            .expect("unable to parse server id from db output");
        let user_id = user
            .user_id()
            .expect("unable to parse user id from db output");

        println!("s#{} c#{} u#{}", server_id, channel_id, user_id);
    }
}

fn print_accounts(accounts: Vec<BankAccount>) {
    if accounts.len() == 0 {
        println!("no accounts returned");
    }
    for account in accounts {
        print_account(account)
    }
}

fn print_account(account: BankAccount) {
    let server_id = account
        .server_id()
        .expect("unable to parse server id from db output");
    let user_id = account
        .user_id()
        .expect("unable to parse user id from db output");
    let balance = account.balance;
    println!("#s{} #u{} ${}", server_id, user_id, balance)
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
                DbCommand::Read(read) => read.op.handle(&db),
                DbCommand::Update(update) => {
                    match update.record {
                        Record::ChannelUser(_) => {
                            // shouldn't happen
                            println!("users cannot be updated");
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
