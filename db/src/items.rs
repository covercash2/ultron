use diesel::{prelude::*, Connection};

use crate::error::{Error, Result};

use crate::schema::{self, items::dsl::*};

use crate::model::{Item, UpdateItem};
use crate::Backend;

pub const ID_MEMBER_CARD: i32 = 1;
pub const ID_BEGGAR_CUP: i32 = 2;

pub fn show_all<C: Connection<Backend = Backend>>(connection: &C) -> Result<Vec<Item>> {
    items.load::<Item>(connection).map_err(Into::into)
}

pub fn create<C>(connection: &C, item: Item) -> Result<()>
where
    C: Connection<Backend = Backend>,
{
    let num_records = diesel::insert_into(schema::items::table)
        .values(&item)
        .execute(connection)?;

    if num_records == 0 {
        Err(Error::RecordExists)
    } else {
        Ok(())
    }
}

pub fn update<C>(connection: &C, item: UpdateItem) -> Result<()>
where
    C: Connection<Backend = Backend>,
{
    let num_records = diesel::update(schema::items::table)
        .set(&item)
        .execute(connection)?;

    if num_records == 0 {
        Err(Error::NotFound("item not found".to_owned()))
    } else {
        Ok(())
    }
}

pub fn delete<C: Connection<Backend = Backend>>(connection: &C, item_id: &i32) -> Result<()> {
    let item = items.find(item_id);
    match diesel::delete(item).execute(connection) {
	Ok(n) => if n == 0 {
	    Ok(())
	} else {
	    Err(Error::NotFound("no item found".to_owned()))
	}
	Err(err) => Err(err.into())
    }
}

pub fn get<C: Connection<Backend = Backend>>(connection: &C, item_id: &i32) -> Result<Item> {
    items.find(item_id).first(connection).map_err(Into::into)
}
