use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use subxt::utils::AccountId32;

pub(crate) fn serialize_map<S>(
    value: &BTreeMap<AccountId32, u128>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let string_map: BTreeMap<_, _> = value.iter().map(|(k, v)| (k, v.to_string())).collect();
    string_map.serialize(serializer)
}

pub(crate) fn deserialize_map<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<AccountId32, u128>, D::Error>
where
    D: Deserializer<'de>,
{
    let string_map: BTreeMap<AccountId32, String> = BTreeMap::deserialize(deserializer)?;
    string_map
        .into_iter()
        .map(|(k, v)| Ok((k, v.parse::<u128>().map_err(serde::de::Error::custom)?)))
        .collect()
}

pub(crate) fn ser_u128_as_string<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = value.to_string();
    serializer.serialize_str(&s)
}

pub(crate) fn de_u128_from_string<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<u128>().map_err(serde::de::Error::custom)
}
