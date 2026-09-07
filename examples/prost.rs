//! Protobuf interop with a hand-derived `prost::Message`: shows the wire
//! bytes, decoding, the length check, and the strict RFC 9562 check.
//!
//! Run with `cargo run --example prost`.

use prost::Message;
use prost_uuid_bytes::{Error, ProstUuid};

/// Equivalent to
///
/// ```protobuf
/// message User {
///   uuid.Uuid id = 1;
///   string name = 2;
/// }
/// ```
///
/// compiled with `extern_path(".uuid.Uuid", "::prost_uuid_bytes::ProstUuid")`.
#[derive(Clone, PartialEq, Message)]
struct User {
    #[prost(message, optional, tag = "1")]
    id: Option<ProstUuid>,
    #[prost(string, tag = "2")]
    name: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id: ProstUuid = "6ba7b810-9dad-11d1-80b4-00c04fd430c8".parse()?;
    let user = User {
        id: Some(id),
        name: "Ada".to_owned(),
    };

    let wire = user.encode_to_vec();
    println!(
        "User {{ id: {id}, name: \"Ada\" }} encodes to {} bytes:",
        wire.len()
    );
    println!("  {}   field 1 (uuid.Uuid), length 18", hex(&wire[..2]));
    println!("  {}   field 1 (bytes value), length 16", hex(&wire[2..4]));
    println!("  {}", hex(&wire[4..20]));
    println!("  {}   field 2 (string name) = \"Ada\"\n", hex(&wire[20..]));
    assert_eq!(&wire[4..20], id.as_bytes());

    let decoded = User::decode(&wire[..])?;
    assert_eq!(decoded, user);
    println!("decoded id: {}", decoded.id.unwrap());

    // Bare ProstUuid is itself a message (18 bytes).
    assert_eq!(id.encoded_len(), 18);
    assert_eq!(ProstUuid::decode(&id.encode_to_vec()[..])?, id);

    // A payload of the wrong length is rejected at decode time.
    let mut corrupted = wire.clone();
    corrupted[1] -= 1; // outer length 18 -> 17
    corrupted[3] -= 1; // bytes length 16 -> 15
    corrupted.remove(19);
    let err = User::decode(&corrupted[..]).unwrap_err();
    println!("truncated payload -> DecodeError: {err}\n");

    // Length is all the decoder checks. Stricter RFC 9562 validation is opt-in.
    let odd = ProstUuid::from_bytes([0xc0; 16]); // variant 110x (Microsoft), version 0xc
    let decoded_odd = ProstUuid::decode(&odd.encode_to_vec()[..])?;
    println!(
        "{decoded_odd} decodes fine, is_rfc9562 = {}",
        decoded_odd.is_rfc9562()
    );
    assert_eq!(
        decoded_odd.validate_rfc9562(),
        Err(Error::InvalidVariant(uuid::Variant::Microsoft))
    );
    println!("{id} is_rfc9562 = {}", id.is_rfc9562());
    println!(
        "from_slice_strict on 15 bytes -> {:?}",
        ProstUuid::from_slice_strict(&id.as_bytes()[..15])
    );

    Ok(())
}
