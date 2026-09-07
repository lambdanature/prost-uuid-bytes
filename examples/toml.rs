//! TOML interop via the `toml` crate: a config file holding UUIDs as
//! RFC 9562 strings round-trips through `ProstUuid`.
//!
//! Run with `cargo run --example toml`.

use prost_uuid_bytes::ProstUuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Config {
    node_id: ProstUuid,
    cluster: Cluster,
    peers: Vec<Peer>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Cluster {
    id: ProstUuid,
    name: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Peer {
    id: ProstUuid,
    addr: String,
}

const INPUT: &str = r#"
node_id = "018f2a6e-9c4b-7d3e-8a1f-0b2c3d4e5f60"

[cluster]
id = "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
name = "prod"

[[peers]]
id = "6BA7B811-9DAD-11D1-80B4-00C04FD430C8"
addr = "10.0.0.2:7000"

[[peers]]
id = "6ba7b8129dad11d180b400c04fd430c8"
addr = "10.0.0.3:7000"
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config: Config = toml::from_str(INPUT)?;
    println!("parsed: {config:#?}\n");

    // Output is always normalized to lowercase hyphenated form.
    let out = toml::to_string(&config)?;
    println!("serialized:\n{out}");

    let back: Config = toml::from_str(&out)?;
    assert_eq!(back, config);

    for peer in &config.peers {
        assert!(peer.id.is_rfc9562());
    }
    assert_eq!(config.node_id.get_version_num(), 7);

    // Malformed UUID text is rejected with a located error.
    let err = toml::from_str::<Config>(
        &INPUT.replace("018f2a6e-9c4b-7d3e-8a1f-0b2c3d4e5f60", "not-a-uuid"),
    )
    .map(|_| ())
    .unwrap_err();
    println!("invalid input -> error:\n{err}");

    Ok(())
}
