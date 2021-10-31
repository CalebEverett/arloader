use crate::{
    error::ArweaveError,
    merkle::{Node, Proof},
};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

type Error = ArweaveError;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Transaction {
    pub format: u8,
    pub id: Base64,
    pub last_tx: Base64,
    pub owner: Base64,
    pub tags: Vec<Tag>,
    pub target: Base64,
    #[serde(with = "stringify")]
    pub quantity: u64,
    pub data_root: Base64,
    pub data: Base64,
    #[serde(with = "stringify")]
    pub data_size: u64,
    #[serde(with = "stringify")]
    pub reward: u64,
    pub signature: Base64,
    #[serde(skip)]
    pub chunks: Vec<Node>,
    #[serde(skip)]
    pub proofs: Vec<Proof>,
}

pub mod stringify {
    use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::fmt::Display,
    {
        String::deserialize(deserializer)?
            .parse::<T>()
            .map_err(|e| D::Error::custom(format!("{}", e)))
    }

    pub fn serialize<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: std::fmt::Display,
    {
        format!("{}", value).serialize(serializer)
    }
}

pub trait ToItems<'a, T> {
    // fn to_slices(&'a self) -> Result<Vec<Vec<&'a [u8]>>, Error>;
    fn to_deep_hash_item(&'a self) -> Result<DeepHashItem, Error>;
    fn to_slices(&'a self) -> Result<Vec<Vec<&'a [u8]>>, Error>;
}

impl<'a> ToItems<'a, Transaction> for Transaction {
    fn to_slices(&'a self) -> Result<Vec<Vec<&'a [u8]>>, Error> {
        Ok(vec![vec![&[0]]])
    }
    fn to_deep_hash_item(&'a self) -> Result<DeepHashItem, Error> {
        match &self.format {
            1 => {
                let mut children: Vec<DeepHashItem> = vec![
                    &self.owner.0[..],
                    &self.target.0,
                    &self.data.0,
                    self.quantity.to_string().as_bytes(),
                    self.reward.to_string().as_bytes(),
                    &self.last_tx.0,
                ]
                .into_iter()
                .map(DeepHashItem::from_item)
                .collect();
                children.push(self.tags.to_deep_hash_item()?);

                Ok(DeepHashItem::from_children(children))
            }
            2 => {
                let mut children: Vec<DeepHashItem> = vec![
                    self.format.to_string().as_bytes(),
                    &self.owner.0,
                    &self.target.0,
                    self.quantity.to_string().as_bytes(),
                    self.reward.to_string().as_bytes(),
                    &self.last_tx.0,
                ]
                .into_iter()
                .map(DeepHashItem::from_item)
                .collect();
                children.push(self.tags.to_deep_hash_item()?);
                children.push(DeepHashItem::from_item(
                    self.data_size.to_string().as_bytes(),
                ));
                children.push(DeepHashItem::from_item(&self.data_root.0));

                Ok(DeepHashItem::from_children(children))
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tag {
    pub name: Base64,
    pub value: Base64,
}

pub trait FromStrs<T> {
    fn from_utf8_strs(name: &str, value: &str) -> Result<T, Error>;
}

impl FromStrs<Tag> for Tag {
    fn from_utf8_strs(name: &str, value: &str) -> Result<Self, Error> {
        let b64_name = Base64::from_utf8_str(name)?;
        let b64_value = Base64::from_utf8_str(value)?;

        Ok(Self {
            name: b64_name,
            value: b64_value,
        })
    }
}

impl<'a> ToItems<'a, Vec<Tag>> for Vec<Tag> {
    fn to_slices(&'a self) -> Result<Vec<Vec<&'a [u8]>>, Error> {
        let result = self
            .iter()
            .map(|t| vec![&t.name.0[..], &t.value.0[..]])
            .collect();
        Ok(result)
    }
    fn to_deep_hash_item(&'a self) -> Result<DeepHashItem, Error> {
        if self.len() > 0 {
            Ok(DeepHashItem {
                blob: None,
                list: Some(
                    self.iter()
                        .map(|t| t.to_deep_hash_item().unwrap())
                        .collect(),
                ),
            })
        } else {
            Ok(DeepHashItem {
                blob: Some(Vec::<u8>::new()),
                list: None,
            })
        }
    }
}

impl<'a> ToItems<'a, Tag> for Tag {
    fn to_slices(&'a self) -> Result<Vec<Vec<&'a [u8]>>, Error> {
        Ok(vec![vec![&[0]]])
    }
    fn to_deep_hash_item(&'a self) -> Result<DeepHashItem, Error> {
        Ok(DeepHashItem {
            blob: None,
            list: Some(vec![
                DeepHashItem {
                    blob: Some(self.name.0.to_vec()),
                    list: None,
                },
                DeepHashItem {
                    blob: Some(self.value.0.to_vec()),
                    list: None,
                },
            ]),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Base64(pub Vec<u8>);

impl Default for Base64 {
    fn default() -> Self {
        Base64(vec![])
    }
}

impl std::fmt::Display for Base64 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let string = &base64::display::Base64Display::with_config(&self.0, base64::URL_SAFE_NO_PAD);
        write!(f, "{}", string)
    }
}

/// Converts a base64url encoded string to a Base64 struct.
impl FromStr for Base64 {
    type Err = base64::DecodeError;
    fn from_str(str: &str) -> Result<Self, Self::Err> {
        let result = base64::decode_config(str, base64::URL_SAFE_NO_PAD)?;
        Ok(Self(result))
    }
}

/// Handles conversion of unencoded strings through to base64url and back to bytes.
pub trait ConvertUtf8<T> {
    fn from_utf8_str(str: &str) -> Result<T, Error>;
    fn to_utf8_string(&self) -> Result<String, Error>;
}

impl ConvertUtf8<Base64> for Base64 {
    fn from_utf8_str(str: &str) -> Result<Self, Error> {
        let enc_string = base64::encode_config(str.as_bytes(), base64::URL_SAFE_NO_PAD);
        let dec_bytes = base64::decode_config(enc_string, base64::URL_SAFE_NO_PAD)?;
        Ok(Self(dec_bytes))
    }
    fn to_utf8_string(&self) -> Result<String, Error> {
        let enc_string = base64::encode_config(&self.0, base64::URL_SAFE_NO_PAD);
        let dec_bytes = base64::decode_config(enc_string, base64::URL_SAFE_NO_PAD)?;
        Ok(String::from_utf8(dec_bytes)?)
    }
}

impl Serialize for Base64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&format!("{}", &self))
    }
}

impl<'de> Deserialize<'de> for Base64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Vis;
        impl serde::de::Visitor<'_> for Vis {
            type Value = Base64;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a base64 string")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                base64::decode_config(v, base64::URL_SAFE_NO_PAD)
                    .map(Base64)
                    .map_err(|_| de::Error::custom("failed to decode base64 string"))
            }
        }
        deserializer.deserialize_str(Vis)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct DeepHashItem {
    pub blob: Option<Vec<u8>>,
    pub list: Option<Vec<DeepHashItem>>,
}

pub trait FromItemOrChild {
    fn from_item(item: &[u8]) -> Self;
    fn from_children(children: Vec<DeepHashItem>) -> Self;
}

impl FromItemOrChild for DeepHashItem {
    fn from_item(item: &[u8]) -> DeepHashItem {
        Self {
            blob: Some(item.to_vec()),
            list: None,
        }
    }
    fn from_children(children: Vec<DeepHashItem>) -> DeepHashItem {
        Self {
            blob: None,
            list: Some(children),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::transaction::FromStrs;

    use super::{Base64, ConvertUtf8, DeepHashItem, Error, Tag, ToItems};
    // use serde::{self, de, Deserialize, Deserializer, Serialize, Serializer};
    use serde_json;
    use std::str::FromStr;

    #[test]
    fn test_deserialize_base64() -> Result<(), Error> {
        let base_64 = Base64(vec![44; 7]);
        assert_eq!(base_64.0, vec![44; 7]);
        assert_eq!(format!("{}", base_64), "LCwsLCwsLA");

        let base_64: Base64 = serde_json::from_str("\"LCwsLCwsLA\"")?;
        assert_eq!(base_64.0, vec![44; 7]);
        assert_eq!(format!("{}", base_64), "LCwsLCwsLA");
        Ok(())
    }

    #[test]
    fn test_base64_convert_utf8() -> Result<(), Error> {
        let foo_b64 = Base64::from_utf8_str("foo")?;
        assert_eq!(foo_b64.0, vec![102, 111, 111]);

        let foo_b64 = Base64(vec![102, 111, 111]);
        assert_eq!(foo_b64.to_utf8_string()?, "foo".to_string());
        Ok(())
    }

    #[test]
    fn test_base64_convert_string() -> Result<(), Error> {
        let foo_b64 = Base64::from_str("LCwsLCwsLA")?;
        assert_eq!(foo_b64.0, vec![44; 7]);

        let foo_b64 = Base64(vec![44; 7]);
        assert_eq!(foo_b64.to_string(), "LCwsLCwsLA".to_string());
        Ok(())
    }

    #[test]
    fn test_transaction_slices() -> Result<(), Error> {
        let tags = Vec::<Tag>::new();
        assert_eq!(tags.to_slices()?, Vec::<Vec<&[u8]>>::new());

        let tags = vec![
            Tag::from_utf8_strs("Content-Type", "text/html")?,
            Tag::from_utf8_strs("key2", "value2")?,
        ];

        assert_eq!("Content-Type".to_string(), tags[0].name.to_utf8_string()?);
        assert_eq!("Q29udGVudC1UeXBl".to_string(), tags[0].name.to_string());

        let tag_slices = tags.to_slices()?;
        assert_eq!(tag_slices.len(), 2);
        tag_slices.iter().for_each(|f| assert_eq!(f.len(), 2));
        assert_eq!(
            tag_slices[0][0],
            &[67, 111, 110, 116, 101, 110, 116, 45, 84, 121, 112, 101][..]
        );
        assert_eq!(
            tag_slices[0][1],
            &[116, 101, 120, 116, 47, 104, 116, 109, 108][..]
        );
        assert_eq!(tag_slices[1][0], &[107, 101, 121, 50][..]);
        assert_eq!(tag_slices[1][1], &[118, 97, 108, 117, 101, 50][..]);
        Ok(())
    }

    #[test]
    fn test_tags_deep_hash_item() -> Result<(), Error> {
        let tags = Vec::<Tag>::new();
        assert_eq!(
            tags.to_deep_hash_item()?,
            DeepHashItem {
                blob: Some(Vec::<u8>::new()),
                list: None
            }
        );

        let tags = vec![
            Tag::from_utf8_strs("Content-Type", "text/html")?,
            Tag::from_utf8_strs("key2", "value2")?,
        ];

        assert_eq!("Content-Type".to_string(), tags[0].name.to_utf8_string()?);
        assert_eq!("Q29udGVudC1UeXBl".to_string(), tags[0].name.to_string());

        let deep_hash_item = tags.to_deep_hash_item()?;

        let deep_hash_item_actual = DeepHashItem {
            blob: None,
            list: Some(vec![
                DeepHashItem {
                    blob: None,
                    list: Some(vec![
                        DeepHashItem {
                            blob: Some(vec![
                                67, 111, 110, 116, 101, 110, 116, 45, 84, 121, 112, 101,
                            ]),
                            list: None,
                        },
                        DeepHashItem {
                            blob: Some(vec![116, 101, 120, 116, 47, 104, 116, 109, 108]),
                            list: None,
                        },
                    ]),
                },
                DeepHashItem {
                    blob: None,
                    list: Some(vec![
                        DeepHashItem {
                            blob: Some(vec![107, 101, 121, 50]),
                            list: None,
                        },
                        DeepHashItem {
                            blob: Some(vec![118, 97, 108, 117, 101, 50]),
                            list: None,
                        },
                    ]),
                },
            ]),
        };

        assert_eq!(deep_hash_item, deep_hash_item_actual);
        Ok(())
    }
}
