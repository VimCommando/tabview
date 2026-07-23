use std::fmt;

use serde::de::{DeserializeSeed, Error, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde_json::value::RawValue;

use super::JsonPointer;

#[derive(Debug)]
pub(crate) enum SelectedJsonValue {
    ArrayRows(Vec<Box<RawValue>>),
    Object(Vec<RawObjectEntry>),
    Scalar,
}

#[derive(Debug)]
pub(crate) struct RawObjectEntry {
    pub key: String,
    pub value: Box<RawValue>,
}

impl RawObjectEntry {
    pub fn encoded_len(&self) -> u64 {
        let key_len = serde_json::to_vec(&self.key)
            .map(|key| key.len() as u64)
            .unwrap_or(self.key.len() as u64 + 2);
        key_len
            .saturating_add(1)
            .saturating_add(self.value.get().len() as u64)
    }
}

pub(crate) fn select_json_table(
    bytes: &[u8],
    pointer: Option<&JsonPointer>,
) -> anyhow::Result<SelectedJsonValue> {
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
    type Value = SelectedJsonValue;

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
    type Value = SelectedJsonValue;

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
                let selected = sequence
                    .next_element_seed(SelectSeed {
                        segments: self.remaining,
                    })?
                    .ok_or_else(|| A::Error::custom("JSON starting path was not found"))?;
                while sequence.next_element::<IgnoredAny>()?.is_some() {}
                return Ok(selected);
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
    type Value = SelectedJsonValue;

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
        Ok(SelectedJsonValue::ArrayRows(rows))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut entries = Vec::new();
        while let Some((key, value)) = map.next_entry::<String, Box<RawValue>>()? {
            entries.push(RawObjectEntry { key, value });
        }
        Ok(SelectedJsonValue::Object(entries))
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(SelectedJsonValue::Scalar)
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(SelectedJsonValue::Scalar)
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(SelectedJsonValue::Scalar)
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(SelectedJsonValue::Scalar)
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(SelectedJsonValue::Scalar)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(SelectedJsonValue::Scalar)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(SelectedJsonValue::Scalar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_selected_array_and_skips_surrounding_metadata() {
        let rows = select_json_table(
            br#"{"large_metadata":{"ignored":[1,2,3]},"hits":{"hits":[{"a":1},{"a":2}]},"tail":true}"#,
            Some(&"/hits/hits".parse().unwrap()),
        )
        .expect("rows");
        let SelectedJsonValue::ArrayRows(rows) = rows else {
            panic!("array rows");
        };
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get(), "{\"a\":1}");
        assert_eq!(rows[1].get(), "{\"a\":2}");
    }

    #[test]
    fn resolves_escaped_pointer_segments_with_seeded_visitors() {
        let rows = select_json_table(
            br#"{"a/b":{"~rows":[[1,2]]}}"#,
            Some(&"/a~1b/~0rows".parse().unwrap()),
        )
        .expect("rows");
        let SelectedJsonValue::ArrayRows(rows) = rows else {
            panic!("array rows");
        };
        assert_eq!(rows[0].get(), "[1,2]");
    }

    #[test]
    fn array_pointer_drains_elements_after_the_selected_index() {
        let selected = select_json_table(
            br#"[[{"a":1}],[{"a":2}],[{"a":3}]]"#,
            Some(&"/0".parse().unwrap()),
        )
        .expect("rows");
        let SelectedJsonValue::ArrayRows(rows) = selected else {
            panic!("array rows");
        };

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get(), "{\"a\":1}");
    }

    #[test]
    fn preserves_object_member_order_and_duplicates() {
        let selected = select_json_table(br#"{"a":{"v":1},"a":{"v":2}}"#, None).expect("object");
        let SelectedJsonValue::Object(entries) = selected else {
            panic!("object entries");
        };
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "a");
        assert_eq!(entries[1].value.get(), "{\"v\":2}");
    }

    #[test]
    fn classifies_selected_scalars_without_assigning_table_shape() {
        for input in [br#"true"#.as_slice(), br#"1"#, br#"null"#, br#""text""#] {
            assert!(matches!(
                select_json_table(input, None).expect("scalar"),
                SelectedJsonValue::Scalar
            ));
        }
        assert!(matches!(
            select_json_table(br#"{"value":1}"#, Some(&"/value".parse().unwrap()))
                .expect("selected scalar"),
            SelectedJsonValue::Scalar
        ));
    }
}
