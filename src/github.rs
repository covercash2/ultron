use hubcaps::{Credentials, Github};

use crate::error::Result;

fn init<S: Into<String>>(token: S) -> Result<Github> {
    Github::new(
	"ultron-bot-user-agent/0.1.0",
	Credentials::Token(token.into()),
    )
        .map_err(Into::into)
}
