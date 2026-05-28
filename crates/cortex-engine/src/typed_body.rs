//! Typed body parsing for Fact, Entity, and Relation cells.

use std::collections::BTreeMap;

/// Parsed fields from a Fact cell body.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FactBody {
    pub metric: Option<String>,
    pub value: Option<String>,
    pub currency: Option<String>,
    pub project: Option<String>,
}

/// Parsed fields from an Entity cell body.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EntityBody {
    pub name: Option<String>,
    pub kind: Option<String>,
}

/// Parsed fields from a Relation cell body.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RelationBody {
    pub subject: Option<String>,
    pub predicate: Option<String>,
    pub object: Option<String>,
}

fn parse_key_value_body(body: &[u8]) -> BTreeMap<String, String> {
    let text = String::from_utf8_lossy(body);
    let mut map = BTreeMap::new();
    for line in text.lines() {
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.trim().to_lowercase(), value.trim().to_owned());
        }
    }
    map
}

impl FactBody {
    pub fn parse(body: &[u8]) -> Self {
        let map = parse_key_value_body(body);
        Self {
            metric: map.get("metric").cloned(),
            value: map.get("value").cloned(),
            currency: map.get("currency").cloned(),
            project: map.get("project").cloned(),
        }
    }
}

impl EntityBody {
    pub fn parse(body: &[u8]) -> Self {
        let map = parse_key_value_body(body);
        Self {
            name: map.get("name").cloned(),
            kind: map.get("kind").cloned(),
        }
    }
}

impl RelationBody {
    pub fn parse(body: &[u8]) -> Self {
        let map = parse_key_value_body(body);
        Self {
            subject: map.get("subject").cloned(),
            predicate: map.get("predicate").cloned(),
            object: map.get("object").cloned(),
        }
    }
}
