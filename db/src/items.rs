use diesel::{prelude::*, Connection};

use crate::error::Result;

use crate::schema::items::dsl::*;

use crate::Backend;
use crate::model::Item;

pub fn show_all<C: Connection<Backend = Backend>>(connection: &C) -> Result<Vec<Item>> {
    items.load::<Item>(connection).map_err(Into::into)
}
