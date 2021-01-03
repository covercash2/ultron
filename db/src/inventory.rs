use diesel::{prelude::*, Connection};

use crate::error::Result;

use crate::schema::inventory::dsl::*;

use crate::Backend;
use crate::model::InventoryItem;

// dump all inventory information. admin command only
pub fn show_all<C: Connection<Backend = Backend>>(connection: &C) -> Result<Vec<InventoryItem>> {
    inventory.load::<InventoryItem>(connection).map_err(Into::into)
}
