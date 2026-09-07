#![cfg(feature = "serde")]

use prost_uuid_bytes::ProstUuid;
use serde::{Deserialize, Serialize};

const SAMPLE: &str = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Doc {
    id: ProstUuid,
    ids: Vec<ProstUuid>,
    opt: Option<ProstUuid>,
}

fn doc() -> Doc {
    Doc {
        id: SAMPLE.parse().unwrap(),
        ids: vec![ProstUuid::nil(), ProstUuid::max()],
        opt: None,
    }
}

#[test]
fn json_is_hyphenated_string() {
    let json = serde_json::to_string(&doc()).unwrap();
    assert_eq!(
        json,
        format!(
            r#"{{"id":"{SAMPLE}","ids":["00000000-0000-0000-0000-000000000000","ffffffff-ffff-ffff-ffff-ffffffffffff"],"opt":null}}"#
        )
    );
    assert_eq!(serde_json::from_str::<Doc>(&json).unwrap(), doc());
    assert_eq!(
        serde_json::to_value(ProstUuid::nil()).unwrap(),
        serde_json::Value::String("00000000-0000-0000-0000-000000000000".into())
    );
}

#[test]
fn json_accepts_all_uuid_text_forms() {
    for s in [
        "6ba7b8109dad11d180b400c04fd430c8",
        "{6ba7b810-9dad-11d1-80b4-00c04fd430c8}",
        "urn:uuid:6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "6BA7B810-9DAD-11D1-80B4-00C04FD430C8",
    ] {
        let id: ProstUuid = serde_json::from_str(&format!("\"{s}\"")).unwrap();
        assert_eq!(id.to_string(), SAMPLE, "{s}");
    }
    assert!(serde_json::from_str::<ProstUuid>("\"nope\"").is_err());
    assert!(serde_json::from_str::<ProstUuid>("42").is_err());
}

#[test]
fn toml_round_trip() {
    let text = toml::to_string(&doc()).unwrap();
    assert!(text.contains(&format!("id = \"{SAMPLE}\"")), "{text}");
    assert_eq!(toml::from_str::<Doc>(&text).unwrap(), doc());

    let parsed: Doc = toml::from_str(
        r#"
id = "{6BA7B810-9DAD-11D1-80B4-00C04FD430C8}"
ids = ["6ba7b8109dad11d180b400c04fd430c8"]
opt = "urn:uuid:6ba7b810-9dad-11d1-80b4-00c04fd430c8"
"#,
    )
    .unwrap();
    assert_eq!(parsed.id.to_string(), SAMPLE);
    assert_eq!(parsed.ids, vec![parsed.id]);
    assert_eq!(parsed.opt, Some(parsed.id));

    assert!(toml::from_str::<Doc>("id = \"x\"\nids = []").is_err());
    assert!(toml::from_str::<Doc>("id = 1\nids = []").is_err());
}
