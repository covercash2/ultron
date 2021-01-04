use diesel::{prelude::*, Connection};

use crate::error::{Error, Result};

use crate::schema::{self, items::dsl::*};

use crate::model::{Item, UpdateItem};
use crate::Backend;

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

pub fn get<C: Connection<Backend = Backend>>(connection: &C, item_id: &i32) -> Result<Item> {
    items.find(item_id).first(connection).map_err(Into::into)
}
