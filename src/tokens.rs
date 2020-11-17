use std::env;

use crate::error::Result;

//pub const GITHUB_TOKEN: &'static str = "GITHUB_TOKEN";
pub const DISCORD_TOKEN: &'static str = "DISCORD_TOKEN";

pub fn load_token<K: AsRef<std::ffi::OsStr>>(key: K) -> Result<String> {
    // TODO something better than env vars
    env::var(key).map_err(Into::into)
}
