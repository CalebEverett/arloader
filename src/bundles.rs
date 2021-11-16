use crate::error::Error;
use crate::transaction::{Base64, Tag};
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

    let schema = Schema::parse_str(schema).unwrap();

    schema
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DataItem {
    signature_type: u16,
    signature: Base64,
    owner: Base64,
    target: Option<Base64>,
    anchor: Option<Base64>,
    tags: Vec<Tag>,
    data: Base64,
}

impl Default for DataItem {
    fn default() -> Self {
        Self {
            signature_type: 1,
            signature: Base64(vec![0; 512]),
            owner: Base64(vec![0; 512]),
            target: None,
            anchor: None,
            tags: Vec::new(),
            data: Base64::default(),
        }
    }
}

impl DataItem {
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        let mut buf = Vec::new().writer();
        buf.write(&self.signature_type.to_le_bytes())?;
        buf.write(&self.signature.0)?;
        buf.write(&self.owner.0)?;
        if let Some(target) = &self.target {
            buf.write(&[1])?;
            buf.write(&target.0)?;
        } else {
            buf.write(&[0])?;
        }
        if let Some(anchor) = &self.anchor {
            buf.write(&[1])?;
            buf.write(&anchor.0)?;
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

        data_item.signature.0 = data_item
            .signature
            .0
            .iter()
            .map(|_| iter.next().unwrap())
            .collect();

        data_item.owner.0 = data_item
            .owner
            .0
            .iter()
            .map(|_| iter.next().unwrap())
            .collect();

        data_item.target = match iter.next().unwrap() {
            0 => None,
            1 => Some(Base64([(); 32].map(|_| iter.next().unwrap()).to_vec())),
            _ => unreachable!(),
        };

        data_item.anchor = match iter.next().unwrap() {
            0 => None,
            1 => Some(Base64([(); 32].map(|_| iter.next().unwrap()).to_vec())),
            _ => unreachable!(),
        };

        let result = [(); 8].map(|_| iter.next().unwrap());
        let number_of_tags = u64::from_le_bytes(result);
        let result = [(); 8].map(|_| iter.next().unwrap());
        let number_of_tag_bytes = u64::from_le_bytes(result) as usize;
        println!("{:?}", &number_of_tag_bytes);

        data_item.tags = if number_of_tags > 0 {
            let schema = get_tags_schema();
            let mut reader = Vec::<u8>::with_capacity(number_of_tag_bytes);

            for _ in 0..number_of_tag_bytes {
                reader.push(iter.next().unwrap());
            }

            let value = avro_rs::from_avro_datum::<&[u8]>(&schema, &mut &*reader, None)?;
            let tags: Vec<Tag> = avro_rs::from_value(&value)?;
            tags
        } else {
            Vec::<Tag>::new()
        };

        data_item.data.0 = iter.collect();

        Ok(data_item)
    }
}

#[cfg(test)]
mod tests {
    use super::DataItem;
    use crate::{
        crypto::Provider,
        transaction::{Base64, ConvertUtf8, FromStrs, Tag},
    };
    use std::path::PathBuf;

    async fn get_test_data_item() -> DataItem {
        let crypto = Provider::from_keypair_path(PathBuf::from("tests/fixtures/test_key0.json"))
            .await
            .unwrap();

        let tags = vec![
            Tag::from_utf8_strs(&"e".to_string().repeat(56), "testvalue").unwrap(),
            Tag::from_utf8_strs(&"e".to_string().repeat(56), "testvalue").unwrap(),
        ];
        let owner = crypto.keypair_modulus().unwrap();
        let anchor = Some(Base64::from_utf8_str("TWF0aC5hcHQnI11nbmcoMzYpLnN1YnN0").unwrap());
        let data = Base64::from_utf8_str("tasty").unwrap();

        DataItem {
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

        let expected_bytes: Vec<u8> = vec![
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 232, 198, 246, 244, 49,
            105, 123, 28, 223, 239, 90, 240, 124, 139, 75, 157, 113, 26, 141, 150, 198, 202, 113,
            248, 196, 220, 207, 158, 154, 5, 189, 26, 25, 188, 59, 67, 105, 63, 55, 133, 189, 123,
            158, 253, 50, 255, 247, 134, 22, 115, 101, 211, 178, 254, 131, 106, 214, 43, 176, 242,
            168, 154, 9, 107, 44, 52, 254, 216, 15, 52, 12, 23, 174, 195, 236, 59, 52, 103, 241,
            167, 70, 78, 52, 247, 48, 11, 243, 99, 198, 75, 16, 65, 101, 48, 124, 98, 226, 12, 195,
            52, 160, 250, 166, 208, 211, 136, 184, 13, 18, 168, 178, 224, 75, 223, 92, 227, 55,
            181, 109, 144, 119, 131, 108, 106, 0, 23, 0, 74, 113, 247, 221, 14, 98, 186, 113, 176,
            108, 197, 79, 133, 254, 59, 151, 243, 39, 124, 225, 191, 171, 174, 40, 200, 255, 70,
            90, 172, 223, 66, 56, 124, 202, 198, 176, 125, 198, 84, 139, 180, 89, 48, 207, 239,
            131, 66, 110, 86, 120, 114, 182, 208, 95, 174, 185, 253, 112, 255, 105, 176, 95, 219,
            129, 55, 218, 84, 200, 153, 196, 145, 184, 180, 95, 38, 86, 91, 54, 122, 127, 131, 218,
            12, 205, 213, 111, 1, 136, 253, 236, 123, 232, 54, 109, 251, 33, 82, 59, 97, 195, 6,
            111, 183, 225, 54, 244, 53, 130, 151, 10, 39, 40, 197, 43, 39, 133, 168, 213, 254, 68,
            150, 67, 95, 39, 131, 249, 201, 208, 82, 151, 48, 146, 24, 139, 161, 157, 90, 164, 64,
            21, 151, 138, 166, 34, 109, 215, 73, 229, 181, 89, 134, 233, 104, 89, 238, 251, 141,
            19, 58, 112, 116, 190, 125, 23, 171, 134, 242, 80, 104, 190, 132, 150, 48, 204, 71,
            155, 94, 195, 52, 158, 84, 72, 137, 156, 214, 202, 52, 208, 201, 146, 104, 138, 250,
            28, 187, 226, 93, 191, 246, 71, 116, 230, 25, 233, 190, 18, 212, 78, 168, 59, 153, 209,
            226, 144, 197, 249, 89, 95, 100, 195, 11, 90, 23, 114, 104, 194, 102, 45, 246, 105, 90,
            192, 122, 2, 92, 206, 41, 30, 244, 250, 131, 195, 105, 233, 92, 200, 2, 164, 27, 208,
            239, 73, 143, 187, 58, 100, 170, 53, 96, 2, 250, 120, 135, 198, 196, 229, 136, 194, 36,
            235, 10, 208, 26, 111, 145, 189, 234, 124, 156, 37, 201, 10, 29, 123, 162, 231, 112,
            172, 82, 249, 65, 253, 237, 24, 99, 178, 52, 45, 208, 157, 212, 126, 221, 29, 148, 5,
            188, 121, 51, 67, 34, 99, 22, 104, 94, 157, 142, 115, 53, 157, 12, 147, 133, 24, 41,
            202, 141, 232, 19, 223, 113, 25, 22, 121, 144, 152, 9, 221, 160, 18, 254, 27, 186, 120,
            250, 173, 187, 132, 13, 203, 206, 176, 86, 83, 119, 176, 182, 36, 91, 65, 243, 193,
            241, 124, 79, 123, 71, 38, 238, 172, 58, 152, 47, 130, 205, 106, 101, 36, 43, 5, 155,
            159, 170, 247, 37, 0, 1, 84, 87, 70, 48, 97, 67, 53, 104, 99, 72, 81, 110, 73, 49, 49,
            110, 98, 109, 99, 111, 77, 122, 89, 112, 76, 110, 78, 49, 89, 110, 78, 48, 2, 0, 0, 0,
            0, 0, 0, 0, 182, 0, 0, 0, 0, 0, 0, 0, 4, 150, 1, 90, 87, 86, 108, 90, 87, 86, 108, 90,
            87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108, 90,
            87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108, 90,
            87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108, 90,
            87, 86, 108, 90, 87, 85, 24, 100, 71, 86, 122, 100, 72, 90, 104, 98, 72, 86, 108, 150,
            1, 90, 87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108,
            90, 87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108,
            90, 87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108,
            90, 87, 86, 108, 90, 87, 86, 108, 90, 87, 86, 108, 90, 87, 85, 24, 100, 71, 86, 122,
            100, 72, 90, 104, 98, 72, 86, 108, 0, 116, 97, 115, 116, 121,
        ];

        assert_eq!(&bytes, &expected_bytes);
    }

    #[tokio::test]
    async fn test_deserialize_data_item() {
        let data_item = get_test_data_item().await;

        let bytes = data_item.serialize().unwrap();

        let de_data_item = DataItem::deserialize(bytes).unwrap();

        assert_eq!(data_item, de_data_item)
    }
}
