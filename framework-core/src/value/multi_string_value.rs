use serde::ser::{SerializeMap, SerializeStruct};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MultiStringValue {
    pub(crate) lower: bool,
    pub(crate) map: HashMap<String, Vec<String>>,
}

impl MultiStringValue {
    pub fn create(lower: bool, map: HashMap<String, Vec<String>>) -> Self {
        Self { lower, map }
    }

    fn new(lower: bool) -> Self {
        Self {
            lower,
            map: Default::default(),
        }
    }

    pub fn move_from(&mut self, source: HashMap<String, Vec<String>>) {
        for (nk, nvs) in source {
            let k = if self.lower {
                nk.to_ascii_lowercase()
            } else {
                nk
            };
            self.map.insert(k, nvs);
        }
    }

    pub fn keys(&self) -> Vec<&String> {
        self.map.keys().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&Vec<String>> {
        let key = if self.lower {
            &name.to_ascii_lowercase()
        } else {
            name
        };
        self.map
            .iter()
            .find(|(ek, _)| ek.eq(&key))
            .map(|(_, ev)| ev)
    }

    pub fn get_first(&self, name: &str) -> Option<&String> {
        self.get(name)?.first()
    }

    pub fn content_type(&self) -> Option<&String> {
        self.get("content-type")?.first()
    }

    pub fn mime_type(&self) -> Option<String> {
        self.content_type().map(|v| {
            v.split_once(";")
                .map_or(v.as_str(), |(mime, _)| mime)
                .to_string()
        })
    }

    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(&String, &Vec<String>),
    {
        self.map.iter().for_each(|(k, vs)| f(k, vs))
    }

    pub fn set(&mut self, name: impl ToString, value: impl ToString) {
        self.map.insert(name.to_string(), vec![value.to_string()]);
    }

    pub fn remove(&mut self, name: &str) {
        let key = if self.lower {
            name.to_ascii_lowercase()
        } else {
            name.to_string()
        };
        self.map.remove(&key);
    }

    pub fn set_content_type(&mut self, v: impl ToString) {
        self.set("content-type", v)
    }

    pub fn set_content_length(&mut self, v: impl ToString) {
        self.set("content-length", v)
    }
}

impl Serialize for MultiStringValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("MultiStringValue", 2)?;
        state.serialize_field("lower", &self.lower)?;
        state.serialize_field("map", &SortedMultiStringMap(&self.map))?;
        state.end()
    }
}

struct SortedMultiStringMap<'a>(&'a HashMap<String, Vec<String>>);

impl Serialize for SortedMultiStringMap<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut keys = self.0.keys().collect::<Vec<_>>();
        keys.sort();

        let mut map = serializer.serialize_map(Some(keys.len()))?;
        for key in keys {
            if let Some(value) = self.0.get(key) {
                map.serialize_entry(key, value)?;
            }
        }
        map.end()
    }
}
