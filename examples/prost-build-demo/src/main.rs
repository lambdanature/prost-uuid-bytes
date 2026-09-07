//! Proves that a `.proto` compiled with
//! `extern_path(".uuid.Uuid", "::prost_uuid_bytes::ProstUuid")` is wire
//! compatible with the same `.proto` compiled without it.
//!
//! Run with `cargo run -p prost-build-demo` (needs `protoc`).

use prost::Message;
use prost_uuid_bytes::ProstUuid;

/// Generated with `uuid.Uuid` mapped to `ProstUuid`.
mod with_extern {
    include!(concat!(env!("OUT_DIR"), "/with_extern/demo.rs"));
}

/// Generated vanilla: `uuid.Uuid` is a struct with a `value: Vec<u8>` field.
mod plain {
    pub mod uuid {
        include!(concat!(env!("OUT_DIR"), "/plain/uuid.rs"));
    }
    pub mod demo {
        include!(concat!(env!("OUT_DIR"), "/plain/demo.rs"));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id: ProstUuid = uuid::Uuid::now_v7().into();
    let g1: ProstUuid = "6ba7b810-9dad-11d1-80b4-00c04fd430c8".parse()?;
    let g2 = ProstUuid::nil();

    let typed = with_extern::User {
        id: Some(id),
        name: "Ada".into(),
        group_ids: vec![g1, g2],
    };
    let raw = plain::demo::User {
        id: Some(plain::uuid::Uuid {
            value: id.as_bytes().to_vec(),
        }),
        name: "Ada".into(),
        group_ids: vec![
            plain::uuid::Uuid {
                value: g1.as_bytes().to_vec(),
            },
            plain::uuid::Uuid {
                value: g2.as_bytes().to_vec(),
            },
        ],
    };

    let typed_wire = typed.encode_to_vec();
    let raw_wire = raw.encode_to_vec();
    assert_eq!(
        typed_wire, raw_wire,
        "extern_path output must be byte-identical"
    );
    println!(
        "typed and plain encodings are identical ({} bytes)",
        typed_wire.len()
    );

    // Cross-decode in both directions.
    let typed_from_raw = with_extern::User::decode(&raw_wire[..])?;
    assert_eq!(typed_from_raw, typed);
    println!("decoded from plain: id = {}", typed_from_raw.id.unwrap());

    let raw_from_typed = plain::demo::User::decode(&typed_wire[..])?;
    assert_eq!(raw_from_typed, raw);

    // A plain encoder can emit any length; the typed decoder rejects it.
    let bad = plain::demo::User {
        id: Some(plain::uuid::Uuid {
            value: vec![1, 2, 3],
        }),
        ..Default::default()
    };
    let err = with_extern::User::decode(&bad.encode_to_vec()[..]).unwrap_err();
    println!("3-byte uuid from plain encoder -> {err}");

    Ok(())
}
