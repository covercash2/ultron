use diesel::{prelude::*, Connection};

use crate::error::{Error, Result};

use crate::schema::{self, items::dsl::*};

use crate::Backend;
use crate::model::{Item, UpdateItem};

// dump all inventory information. admin command only
pub fn show_all<C: Connection<Backend = Backend>>(connection: &C) -> Result<Vec<Item>> {
    items.load::<Item>(connection).map_err(Into::into)
}
