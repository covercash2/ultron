use diesel::{Connection, result::Error as ResultError, prelude::*};

use crate::{error::{Error, Result}, model::Item};

use crate::schema::{self, inventory::dsl::*};

use crate::model::InventoryItem;
use crate::Backend;

// dump all inventory information. admin command only
pub fn show_all<C: Connection<Backend = Backend>>(connection: &C) -> Result<Vec<InventoryItem>> {
    inventory
        .load::<InventoryItem>(connection)
        .map_err(Into::into)
}

pub fn add_item<C: Connection<Backend = Backend>>(
    connection: &C,
    inventory_item: InventoryItem,
) -> Result<usize> {
    diesel::insert_into(schema::inventory::table)
        .values(&inventory_item)
        .execute(connection)
        .map_err(|err| match err {
            ResultError::DatabaseError(diesel::result::DatabaseErrorKind::UniqueViolation, _) => {
		Error::RecordExists
	    }
	    _ => err.into()
        })
        .map_err(Into::into)
}

pub fn user_inventory<C: Connection<Backend = Backend>>(
    connection: &C,
    server: String,
    user: String,
) -> Result<Vec<Item>> {
    let item_ids = inventory.select(item_id)
        .filter(server_id.eq(&server))
        .filter(user_id.eq(&user));
    schema::items::table
        .filter(schema::items::dsl::id.eq_any(item_ids))
        .load::<Item>(connection)
        .map_err(Into::into)
}

pub fn user_has_item<C: Connection<Backend = Backend>>(
    connection: &C,
    server: String,
    user: String,
    item: i32,
) -> Result<bool> {
    match inventory.find((server, user, item)).first::<InventoryItem>(connection) {
	Ok(_) => Ok(true),
	Err(e) => {
	    match e {
		ResultError::NotFound => Ok(false),
		_ => Err(e.into())
	    }
	}
    }
}

pub fn delete_item<C: Connection<Backend = Backend>>(
    connection: &C,
    inventory_item: InventoryItem
) -> Result<usize> {
    let server = inventory_item.server_id()?.to_string();
    let user = inventory_item.user_id()?.to_string();
    let item = inventory_item.item_id;
    let item = inventory.find((&server, &user, &item));
    diesel::delete(item)
        .execute(connection)
        .map_err(Into::into)
}
