use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};

use crate::error::Error;

#[derive(Clone)]
pub struct ServerId(u64);
#[derive(Clone)]
pub struct ChannelId(u64);
#[derive(Clone)]
pub struct UserId(u64);
#[derive(Clone)]
pub struct ItemId(u64);

/// This macro is mostly copied from the serenity Discord API
macro_rules! id_u64 {
    ($($name:ident;)*) => {
        $(
            impl $name {
                #[inline]
                pub fn as_u64(&self) -> &u64 {
                    &self.0
                }
            }

            impl AsRef<$name> for $name {
                fn as_ref(&self) -> &Self {
                    self
                }
            }

            impl<'a> From<&'a $name> for $name {
                fn from(id: &'a $name) -> $name {
                    id.clone()
                }
            }

            impl From<u64> for $name {
                fn from(id_as_u64: u64) -> $name {
                    $name(id_as_u64)
                }
            }

            impl PartialEq<u64> for $name {
                fn eq(&self, u: &u64) -> bool {
                    self.0 == *u
                }
            }

            impl Display for $name {
                fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
                    Display::fmt(&self.0, f)
                }
            }

            impl From<$name> for u64 {
                fn from(id: $name) -> u64 {
                    id.0 as u64
                }
            }

            impl From<$name> for i64 {
                fn from(id: $name) -> i64 {
                    id.0 as i64
                }
            }

	    impl FromStr for $name {
		type Err = Error;

		fn from_str(s: &str) -> Result<Self, Self::Err> {
		    s.parse::<u64>()
			.map(|id| id.into())
			.map_err(Into::into)
		}
	    }

        )*
    }
}

id_u64! {
    ServerId;
    ChannelId;
    UserId;
    ItemId;
}
