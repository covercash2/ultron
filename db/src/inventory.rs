use diesel::{prelude::*, Connection};

use crate::error::Result;

use crate::schema::{self, inventory::dsl::*};

use crate::Backend;
use crate::model::InventoryItem;

// dump all inventory information. admin command only
pub fn show_all<C: Connection<Backend = Backend>>(connection: &C) -> Result<Vec<InventoryItem>> {
    inventory.load::<InventoryItem>(connection).map_err(Into::into)
}

pub fn add_item<C: Connection<Backend = Backend>>(connection: &C, inventory_item: InventoryItem) -> Result<usize> {
    diesel::insert_into(schema::inventory::table)
        .values(&inventory_item)
        .execute(connection)
        .map_err(Into::into)
}
