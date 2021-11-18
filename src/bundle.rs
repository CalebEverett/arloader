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
        if data_item.signature_type != 1 {
            println!("invalid signature_type");
            return Err(Error::InvalidDataItem);
        }

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
        if number_of_tag_bytes > 2048 {
            return Err(Error::InvalidDataItem);
        }

        data_item.tags = if number_of_tags > 0 {
            let schema = get_tags_schema();
            let mut reader = Vec::<u8>::with_capacity(number_of_tag_bytes);

            for _ in 0..number_of_tag_bytes {
                reader.push(iter.next().unwrap());
            }

            let value = avro_rs::from_avro_datum::<&[u8]>(&schema, &mut &*reader, None)?;
            let tags: Vec<Tag<String>> = avro_rs::from_value(&value)?;
            if tags.len() != number_of_tags as usize {
                return Err(Error::InvalidDataItem);
            }
            tags
        } else {
            Vec::<Tag<String>>::new()
        };

        data_item.data.0 = iter.collect();

        Ok(data_item)
    }

    /// Header is 64 bytes with first 32 for the size of the bytes le. Second
    /// 32 is id - hashed signature.
    pub fn to_bundle_item(&self) -> Result<(Vec<u8>, Vec<u8>), Error> {
        let binary = self.serialize()?;
        let binary_len = binary.len();
        let mut header = Vec::<u8>::with_capacity(64);

        header.extend((binary_len as u64).to_le_bytes());
        header.extend(&[0u8; 24]);
        header.extend(&self.id.0);

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
        transaction::{Base64, FromUtf8Strs, Tag, ToItems},
        Arweave,
    };
    use std::path::PathBuf;
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
        let id = Base64(vec![0; 32]);

        DataItem {
            id,
            signature,
            owner,
            anchor,
            tags,
            data,
            ..Default::default()
        }
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

        let mut de_data_item = DataItem::deserialize(bytes).unwrap();
        de_data_item.id.0 = vec![0; 32];

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
    async fn test_data_item_deep_hash() {
        let arweave =
            Arweave::from_keypair_path(PathBuf::from("tests/fixtures/test_key0.json"), None)
                .await
                .unwrap();

        let data_item = get_test_data_item().await;
        let deep_hash_item = data_item.to_deep_hash_item().unwrap();
        let deep_hash = arweave.crypto.deep_hash(deep_hash_item).unwrap();
        println!("deep_hash: {:#?}", deep_hash);
        assert_eq!(
            vec![
                29, 28, 37, 35, 175, 82, 189, 135, 213, 51, 252, 26, 145, 181, 187, 1, 17, 143,
                217, 152, 169, 208, 44, 36, 226, 59, 74, 90, 10, 218, 106, 216, 58, 210, 94, 10,
                65, 74, 91, 185, 205, 198, 117, 220, 242, 169, 67, 224
            ],
            &deep_hash
        );
    }

    #[tokio::test]
    async fn test_data_items_to_bundle() {
        let arweave =
            Arweave::from_keypair_path(PathBuf::from("tests/fixtures/test_key0.json"), None)
                .await
                .unwrap();

        let data_item = get_test_data_item().await;
        let data_item_ser = data_item.serialize().unwrap();

        let bundle = arweave
            .create_bundle_from_data_items(vec![data_item.clone(), data_item])
            .unwrap();
        let expected_bytes = fs::read_to_string("tests/fixtures/bundle_ser.json")
            .await
            .unwrap();
        let expected_bytes: Vec<u8> = serde_json::from_str(&expected_bytes).unwrap();

        // number of items in bundle is the same
        assert_eq!(u32::from_le_bytes(bundle[0..4].try_into().unwrap()), 2);

        // 1263 bytes in the first item
        assert_eq!(
            u32::from_le_bytes(bundle[32..36].try_into().unwrap()),
            data_item_ser.len() as u32
        );

        // 1263 bytes in the second item
        assert_eq!(
            u32::from_le_bytes(bundle[96..100].try_into().unwrap()),
            data_item_ser.len() as u32
        );

        // signature type is 1
        assert_eq!(
            u16::from_le_bytes(bundle[160..162].try_into().unwrap()),
            1u16
        );

        // no target is present
        assert_eq!(bundle[160 + 2 + 1024], 0);

        // anchor is present
        assert_eq!(bundle[160 + 2 + 1024 + 1], 1);

        // number of tags is 2
        assert_eq!(
            u64::from_le_bytes(
                bundle[(160 + 2 + 1024 + 1 + 1 + 32)..(160 + 2 + 1024 + 1 + 1 + 32 + 8)]
                    .try_into()
                    .unwrap()
            ),
            2u64
        );

        // number of tag bytes is 182
        assert_eq!(
            u64::from_le_bytes(
                bundle[(160 + 2 + 1024 + 1 + 1 + 32 + 8)..(160 + 2 + 1024 + 1 + 1 + 32 + 8 + 8)]
                    .try_into()
                    .unwrap()
            ),
            182u64
        );

        // sig type of second item is 1
        assert_eq!(
            u16::from_le_bytes(
                bundle[(160 + 2 + 1024 + 1 + 1 + 32 + 8 + 8 + 182 + 5)
                    ..(160 + 2 + 1024 + 1 + 1 + 32 + 8 + 8 + 182 + 5 + 2)]
                    .try_into()
                    .unwrap()
            ),
            1u16
        );

        // bytes are the same
        assert_eq!(bundle, expected_bytes);
    }
}
