//! JSON interop via `serde_json`: a `ProstUuid` serializes as its
//! RFC 9562 hyphenated string and deserializes from any textual UUID form.
//!
//! Run with `cargo run --example json`.

use prost_uuid_bytes::ProstUuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct User {
    id: ProstUuid,
    name: String,
    groups: Vec<ProstUuid>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user = User {
        id: uuid::Uuid::new_v4().into(),
        name: "Ada".to_owned(),
        groups: vec![
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8".parse()?,
            ProstUuid::nil(),
        ],
    };

    let json = serde_json::to_string_pretty(&user)?;
    println!("serialized:\n{json}\n");

    let back: User = serde_json::from_str(&json)?;
    assert_eq!(back, user);
    println!("round-trip ok: {}", back.id);

    // Input may use any form the `uuid` crate parses: simple, braced, URN...
    let lenient = r#"{
        "id": "{6BA7B810-9DAD-11D1-80B4-00C04FD430C8}",
        "name": "Bob",
        "groups": ["6ba7b8109dad11d180b400c04fd430c8", "urn:uuid:6ba7b810-9dad-11d1-80b4-00c04fd430c8"]
    }"#;
    let bob: User = serde_json::from_str(lenient)?;
    println!("lenient input -> {} / {:?}", bob.id, bob.groups);
    assert!(bob.groups.iter().all(|g| *g == bob.id));

    // ...but it is still validated.
    let bad = r#"{"id": "not-a-uuid", "name": "x", "groups": []}"#;
    let err = serde_json::from_str::<User>(bad).unwrap_err();
    println!("invalid input -> error: {err}");

    Ok(())
}
