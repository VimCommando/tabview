use std::fmt;

use serde::de::{DeserializeSeed, Error, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde_json::value::RawValue;

use super::JsonPointer;

pub(crate) fn select_json_rows(
    bytes: &[u8],
    pointer: Option<&JsonPointer>,
) -> anyhow::Result<Vec<Box<RawValue>>> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let segments = pointer.map(JsonPointer::segments).unwrap_or_default();
    let rows = SelectSeed { segments }.deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(rows)
}

struct SelectSeed<'a> {
    segments: &'a [String],
}

impl<'de> DeserializeSeed<'de> for SelectSeed<'_> {
    type Value = Vec<Box<RawValue>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if self.segments.is_empty() {
            deserializer.deserialize_any(TableVisitor)
        } else {
            deserializer.deserialize_any(PathVisitor {
                segment: &self.segments[0],
                remaining: &self.segments[1..],
            })
        }
    }
}

struct PathVisitor<'a> {
    segment: &'a str,
    remaining: &'a [String],
}

impl<'de> Visitor<'de> for PathVisitor<'_> {
    type Value = Vec<Box<RawValue>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object or array containing the JSON Pointer segment")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut selected = None;
        while let Some(key) = map.next_key::<String>()? {
            if key == self.segment {
                selected = Some(map.next_value_seed(SelectSeed {
                    segments: self.remaining,
                })?);
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        selected.ok_or_else(|| A::Error::custom("JSON starting path was not found"))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let target = self
            .segment
            .parse::<usize>()
            .map_err(|_| A::Error::custom("JSON Pointer array segment is not an index"))?;
        for index in 0..=target {
            if index == target {
                return sequence
                    .next_element_seed(SelectSeed {
                        segments: self.remaining,
                    })?
                    .ok_or_else(|| A::Error::custom("JSON starting path was not found"));
            }
            if sequence.next_element::<IgnoredAny>()?.is_none() {
                return Err(A::Error::custom("JSON starting path was not found"));
            }
        }
        unreachable!()
    }
}

struct TableVisitor;

impl<'de> Visitor<'de> for TableVisitor {
    type Value = Vec<Box<RawValue>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON object or array")
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut rows = Vec::new();
        while let Some(row) = sequence.next_element::<Box<RawValue>>()? {
            rows.push(row);
        }
        Ok(rows)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut object = serde_json::Map::new();
        while let Some((key, value)) = map.next_entry::<String, serde_json::Value>()? {
            object.insert(key, value);
        }
        let raw = RawValue::from_string(
            serde_json::to_string(&serde_json::Value::Object(object)).map_err(A::Error::custom)?,
        )
        .map_err(A::Error::custom)?;
        Ok(vec![raw])
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Err(E::custom(
            "JSON starting path does not identify an object or array",
        ))
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Err(E::custom(
            "JSON starting path does not identify an object or array",
        ))
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Err(E::custom(
            "JSON starting path does not identify an object or array",
        ))
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Err(E::custom(
            "JSON starting path does not identify an object or array",
        ))
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Err(E::custom(
            "JSON starting path does not identify an object or array",
        ))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Err(E::custom(
            "JSON starting path does not identify an object or array",
        ))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Err(E::custom(
            "JSON starting path does not identify an object or array",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_selected_array_and_skips_surrounding_metadata() {
        let rows = select_json_rows(
            br#"{"large_metadata":{"ignored":[1,2,3]},"hits":{"hits":[{"a":1},{"a":2}]},"tail":true}"#,
            Some(&"/hits/hits".parse().unwrap()),
        )
        .expect("rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get(), "{\"a\":1}");
        assert_eq!(rows[1].get(), "{\"a\":2}");
    }

    #[test]
    fn resolves_escaped_pointer_segments_with_seeded_visitors() {
        let rows = select_json_rows(
            br#"{"a/b":{"~rows":[[1,2]]}}"#,
            Some(&"/a~1b/~0rows".parse().unwrap()),
        )
        .expect("rows");
        assert_eq!(rows[0].get(), "[1,2]");
    }
}
