use crate::crypto::Provider;
use crate::error::Error;
use crate::transaction::Tag;
use data_item::DataItem;

pub mod data_item;

pub struct Bundle {
    crypto: Provider,
    data_items: Vec<DataItem>,
}

impl Bundle {
    pub fn create_data_item(data: Vec<u8>, tags: Vec<Tag<String>>) -> Result<DataItem, Error> {
        Ok(DataItem::default())
    }
}
