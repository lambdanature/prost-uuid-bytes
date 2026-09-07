# prost-uuid-bytes

`ProstUuid` is a newtype around [`uuid::Uuid`](https://docs.rs/uuid) that
implements [`prost::Message`](https://docs.rs/prost), so a `uuid.Uuid`
message in your `.proto` files becomes a real `Uuid` in generated Rust code.

On the wire the UUID is carried as its **16 raw bytes** in a `bytes` field;
in text (Display, JSON, TOML, …) it is the lowercase hyphenated form from
[RFC 9562](https://www.rfc-editor.org/rfc/rfc9562):
`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`.

| crate | proto shape | inner message size |
|---|---|---|
| [prost-uuid](https://gitlab.com/oss47/prost-uuid) | `{ string uuid_str = 1; }` | 38 B |
| [prost-uuid-doubleint](https://github.com/evilbluebeaver/prost_uuid_doubleint) | `{ uint64 high = 1; uint64 low = 2; }` | ≤ 22 B |
| **prost-uuid-bytes** | `{ bytes value = 1; }` | 18 B |

## Usage

### Proto files

Reference `uuid.Uuid` from your messages (the definition is in
[`proto/uuid.proto`](proto/uuid.proto) and exported as
`prost_uuid_bytes::PROTO_FILE`):

```protobuf
syntax = "proto3";
package uuid;

message Uuid {
  bytes value = 1;
}
```

```protobuf
syntax = "proto3";
package user;

import "uuid.proto";

message User {
  uuid.Uuid id = 1;
  string name = 2;
}
```

### Cargo.toml

```toml
[dependencies]
prost = "0.14"
prost-uuid-bytes = "0.1"

[build-dependencies]
prost-build = "0.14"
prost-uuid-bytes = "0.1"   # only if you use PROTO_FILE in build.rs
```

### build.rs

```rust
fn main() -> std::io::Result<()> {
    // Optional: materialize uuid.proto from the crate instead of vendoring it.
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    std::fs::write(out.join("uuid.proto"), prost_uuid_bytes::PROTO_FILE)?;

    prost_build::Config::new()
        .extern_path(".uuid.Uuid", "::prost_uuid_bytes::ProstUuid")
        .compile_protos(&["proto/user.proto"], &["proto".into(), out])
}
```

The generated `User` then has `id: Option<ProstUuid>`. `ProstUuid`
dereferences to `Uuid`, converts with `From`/`Into`, and parses with
`str::parse` (all forms the `uuid` crate accepts: hyphenated, simple,
braced, URN).

```rust
use prost::Message;
use prost_uuid_bytes::ProstUuid;

let id: ProstUuid = "6ba7b810-9dad-11d1-80b4-00c04fd430c8".parse()?;
let wire = id.encode_to_vec();           // 0a 10 <16 bytes>
assert_eq!(ProstUuid::decode(&wire[..])?, id);
println!("{id}");                        // 6ba7b810-9dad-11d1-80b4-00c04fd430c8
```

## Validation

Decoding requires the `bytes` payload to be **exactly 16 bytes**; any other
length is a `prost::DecodeError`. Any 128-bit value is otherwise accepted, so
legacy GUIDs and opaque identifiers survive a round-trip.

If you need the RFC 9562 variant/version bits enforced, opt in:

```rust
use prost_uuid_bytes::{Error, ProstUuid};

let id = ProstUuid::from_bytes([0xc0; 16]);
assert!(!id.is_rfc9562());
assert_eq!(id.validate_rfc9562(), Err(Error::InvalidVariant(uuid::Variant::Microsoft)));

// Length + RFC 9562 check in one go:
let strict = ProstUuid::from_slice_strict(&[0xc0; 16]);
assert!(strict.is_err());
```

The nil and max UUIDs pass the strict check; everything else must have the
`10xx` variant and a version in `1..=8`.

## serde (JSON, TOML, …)

The `serde` feature (enabled by default) derives `Serialize`/`Deserialize`
transparently over `uuid::Uuid`: human-readable formats use the hyphenated
string, binary formats use the 16 bytes.

```rust
#[derive(serde::Serialize, serde::Deserialize)]
struct User { id: ProstUuid, name: String }

let json = serde_json::to_string(&user)?;      // {"id":"6ba7b810-…","name":"Ada"}
let cfg: User = toml::from_str(r#"
    id = "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    name = "Ada"
"#)?;
```

Disable with `default-features = false`.

## Examples

| command | shows |
|---|---|
| `cargo run --example json` | serde_json round-trip, lenient input, error on invalid text |
| `cargo run --example toml` | TOML config round-trip with nested tables and arrays |
| `cargo run --example prost` | annotated wire bytes, decode error on bad length, strict check |
| `cargo run -p prost-build-demo` | `prost-build` + `extern_path` output is byte-identical to the plain `bytes` field (needs `protoc`) |

## Thanks to

- https://gitlab.com/oss47/prost-uuid
- https://github.com/evilbluebeaver/prost_uuid_doubleint

## License

MIT
