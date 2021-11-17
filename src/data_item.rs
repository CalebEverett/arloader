use crate::error::Error;
use crate::transaction::{Base64, DeepHashItem, Tag, ToItems};
use avro_rs::Schema;
use bytes::BufMut;
use serde::{Deserialize, Serialize};
use std::io::Write;

pub fn get_tags_schema() -> Schema {
    let schema = r#"
        {
            "type": "array",
            "items": {
                "type": "record",
                "name": "tag",
                "fields": [
                    {"name": "name", "type": "string"},
                    {"name": "value", "type": "string"}
                ]
            }
        }
    "#;

    Schema::parse_str(schema).unwrap()
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct DataItem {
    pub id: Base64,
    pub signature_type: u16,
    pub signature: Base64,
    pub owner: Base64,
    pub target: Base64,
    pub anchor: Base64,
    pub tags: Vec<Tag<String>>,
    pub data: Base64,
}

impl Default for DataItem {
    fn default() -> Self {
        Self {
            id: Base64(Vec::with_capacity(32)),
            signature_type: 1,
            signature: Base64(Vec::with_capacity(512)),
            owner: Base64(Vec::with_capacity(512)),
            target: Base64(Vec::with_capacity(32)),
            anchor: Base64(Vec::with_capacity(32)),
            tags: Vec::new(),
            data: Base64::default(),
        }
    }
}

impl DataItem {
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        if self.signature.0.len() != 512 {
            return Err(Error::UnsignedTransaction);
        }
        let mut buf = Vec::new().writer();
        buf.write(&self.signature_type.to_le_bytes())?;
        buf.write(&self.signature.0)?;
        buf.write(&self.owner.0)?;

        if self.target.0.len() > 0 {
            buf.write(&[1])?;
            buf.write(&self.target.0)?;
        } else {
            buf.write(&[0])?;
        }
        if self.anchor.0.len() > 0 {
            buf.write(&[1])?;
            buf.write(&self.anchor.0)?;
        } else {
            buf.write(&[0])?;
        }

        if self.tags.len() > 0 {
            let number_of_tags = self.tags.len() as u64;
            let schema = get_tags_schema();
            let value = avro_rs::to_value(&self.tags)?;
            let tags_bytes = avro_rs::to_avro_datum(&schema, value)?;
            let number_of_tag_bytes = tags_bytes.len() as u64;

            buf.write(&number_of_tags.to_le_bytes())?;
            buf.write(&number_of_tag_bytes.to_le_bytes())?;
            buf.write(&tags_bytes)?;
        } else {
            buf.write(&[0; 16])?;
        };

        buf.write(&self.data.0)?;

        Ok(buf.into_inner())
    }
    pub fn deserialize(bytes_vec: Vec<u8>) -> Result<Self, Error> {
        let mut iter = bytes_vec.into_iter();
        let mut data_item = DataItem::default();

        let result = [(); 2].map(|_| iter.next().unwrap());
        data_item.signature_type = u16::from_le_bytes(result);

        for _ in 0..512 {
            data_item.signature.0.push(iter.next().unwrap());
        }

        for _ in 0..512 {
            data_item.owner.0.push(iter.next().unwrap());
        }

        data_item.target = {
            if iter.next().unwrap() == 0 {
                Base64::default()
            } else {
                let mut result = Base64(Vec::with_capacity(32));
                for _ in 0..32 {
                    result.0.push(iter.next().unwrap());
                }
                result
            }
        };

        data_item.anchor = {
            if iter.next().unwrap() == 0 {
                Base64::default()
            } else {
                let mut result = Base64(Vec::with_capacity(32));
                for _ in 0..32 {
                    result.0.push(iter.next().unwrap());
                }
                result
            }
        };

        let number_of_tags = u64::from_le_bytes([(); 8].map(|_| iter.next().unwrap()));
        let number_of_tag_bytes =
            u64::from_le_bytes([(); 8].map(|_| iter.next().unwrap())) as usize;
        data_item.tags = if number_of_tags > 0 {
            let schema = get_tags_schema();
            let mut reader = Vec::<u8>::with_capacity(number_of_tag_bytes);

            for _ in 0..number_of_tag_bytes {
                reader.push(iter.next().unwrap());
            }

            let value = avro_rs::from_avro_datum::<&[u8]>(&schema, &mut &*reader, None)?;
            let tags: Vec<Tag<String>> = avro_rs::from_value(&value)?;
            tags
        } else {
            Vec::<Tag<String>>::new()
        };

        data_item.data.0 = iter.collect();

        Ok(data_item)
    }

    pub fn to_bundle_item(&self) -> Result<(Vec<u8>, Vec<u8>), Error> {
        let binary = self.serialize()?;
        let binary_len = binary.len();
        let mut header = Vec::<u8>::with_capacity(64);

        for b in (binary_len as u64).to_le_bytes() {
            header.push(b)
        }
        header.extend(&[0u8; 24]);
        header.extend(&self.id.0);

        println!("{}", header.len());

        Ok((header, binary))
    }
}

impl<'a> ToItems<'a, DataItem> for DataItem {
    fn to_deep_hash_item(&'a self) -> Result<DeepHashItem, Error> {
        let schema = get_tags_schema();
        let value = avro_rs::to_value(&self.tags)?;
        let tags_bytes = avro_rs::to_avro_datum(&schema, value)?;

        let children: Vec<DeepHashItem> = vec![
            "dataitem".as_bytes(),
            "1".as_bytes(),
            self.signature_type.to_string().as_bytes(),
            &self.owner.0,
            &self.target.0,
            &self.anchor.0,
            &tags_bytes,
            &self.data.0,
        ]
        .into_iter()
        .map(DeepHashItem::from_item)
        .collect();

        Ok(DeepHashItem::List(children))
    }
}

#[cfg(test)]
mod tests {
    use super::DataItem;
    use crate::{
        transaction::{Base64, Tag},
        Arweave,
    };
    use std::{path::PathBuf, str::FromStr};
    use tokio::fs;

    async fn get_test_data_item() -> DataItem {
        let arweave =
            Arweave::from_keypair_path(PathBuf::from("tests/fixtures/test_key0.json"), None)
                .await
                .unwrap();

        let tags = vec![
            Tag::<String>::from_utf8_strs(
                &"ZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWU",
                &"dGVzdHZhbHVl",
            )
            .unwrap(),
            Tag::<String>::from_utf8_strs(
                &"ZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWVlZWU",
                &"dGVzdHZhbHVl",
            )
            .unwrap(),
        ];
        let owner = arweave.crypto.keypair_modulus().unwrap();
        let anchor = Base64::from_utf8_str("TWF0aC5hcHQnI11nbmcoMzYpLnN1YnN0").unwrap();
        let data = Base64::from_utf8_str("tasty").unwrap();
        let signature = Base64(vec![0; 512]);

        DataItem {
            signature,
            owner,
            anchor,
            tags,
            data,
            ..Default::default()
        }
    }

    async fn get_test_data_items() -> Vec<DataItem> {
        let arweave =
            Arweave::from_keypair_path(PathBuf::from("tests/fixtures/test_key0.json"), None)
                .await
                .unwrap();

        let tags = vec![Tag::<String>::from_utf8_strs(&"x", &"y").unwrap()];
        let owner = arweave.crypto.keypair_modulus().unwrap();
        let anchor = Base64::from_utf8_str("Math.randomgng(36).substring(30)").unwrap();
        let target = Base64::from_str("pFwvlpz1x_nebBPxkK35NZm522XPnvUSveGf4Pz8y4A").unwrap();
        let data = Base64::from_utf8_str("tasty").unwrap();

        let data_item = DataItem {
            owner,
            target,
            anchor,
            tags,
            data,
            ..Default::default()
        };

        let data_item = arweave.sign_data_item(data_item).unwrap();

        vec![data_item.clone(), data_item]
    }

    #[tokio::test]
    async fn test_serialize_data_item() {
        let data_item = get_test_data_item().await;

        let bytes = data_item.serialize().unwrap();

        let expected_bytes = fs::read_to_string("tests/fixtures/data_item_ser.json")
            .await
            .unwrap();
        let expected_bytes: Vec<u8> = serde_json::from_str(&expected_bytes).unwrap();

        assert_eq!(&bytes, &expected_bytes);
    }

    #[tokio::test]
    async fn test_deserialize_data_item() {
        let data_item = get_test_data_item().await;

        let bytes = data_item.serialize().unwrap();

        let de_data_item = DataItem::deserialize(bytes).unwrap();

        assert_eq!(data_item, de_data_item)
    }

    #[tokio::test]
    async fn test_data_item_to_json() {
        let data_item = get_test_data_item().await;
        assert_eq!(
            data_item.data.to_utf8_string().unwrap(),
            "tasty".to_string()
        );
    }

    #[tokio::test]
    async fn test_data_item_to_bundle_item() {
        let arweave =
            Arweave::from_keypair_path(PathBuf::from("tests/fixtures/test_key0.json"), None)
                .await
                .unwrap();

        let data_item = get_test_data_item().await;
        let data_item = arweave.sign_data_item(data_item).unwrap();

        let bundle = arweave
            .create_bundle_from_data_items(vec![data_item.clone(), data_item])
            .unwrap();
        let expected_bytes = fs::read_to_string("tests/fixtures/bundle_ser.json")
            .await
            .unwrap();
        let expected_bytes: Vec<u8> = serde_json::from_str(&expected_bytes).unwrap();

        let slice: std::ops::Range<usize> = 32..160;
        assert_eq!(bundle[slice.clone()], expected_bytes[slice]);
    }
}
