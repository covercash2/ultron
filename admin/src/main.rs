use std::env;

use clap::Clap;
use dotenv::dotenv;

use db::{self, model::BankAccount};

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
    #[clap(version = "0.1")]
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
    Insert(Insert)
}

/// insert 
#[derive(Clap)]
struct Insert {
    server_id: u64,
    channel_id: u64,
    user_id: u64,
}

enum Record {
    ChannelUser,
    Balance
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
		.ok_or(env::var("TEST_DB_URL"))
		.or(env::var("PROD_DB_URL"))
		.expect("pass --db_url <path> or set TEST_DB_URL or PROD_DB_URL");

	    if opts.verbose > 0 {
		println!("using database file: {}", database_url);
	    }

	    let db = db::Db::open(&database_url).expect("unable to open database");
        }
    }
}
