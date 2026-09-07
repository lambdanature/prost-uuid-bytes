//! [`ProstUuid`]: a newtype around [`uuid::Uuid`] that implements
//! [`prost::Message`] as a protobuf message holding the UUID's 16 raw bytes.
//!
//! On the wire this is
//!
//! ```protobuf
//! package uuid;
//! message Uuid {
//!   bytes value = 1;
//! }
//! ```
//!
//! (shipped as [`PROTO_FILE`]), so that a `.proto` field `uuid.Uuid id = 1;`
//! becomes `Option<ProstUuid>` when compiled with
//! `prost_build::Config::extern_path(".uuid.Uuid", "::prost_uuid_bytes::ProstUuid")`.
//!
//! Decoding checks that the payload is exactly 16 bytes; any 128-bit value is
//! accepted. Use [`ProstUuid::validate_rfc9562`] / [`ProstUuid::is_rfc9562`]
//! to additionally require the RFC 9562 variant and version bits.
//!
//! [`Display`](core::fmt::Display) prints the lowercase hyphenated form from
//! RFC 9562 §4 (`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`), and with the `serde`
//! feature (on by default) human-readable formats such as JSON and TOML use
//! the same string, while binary formats carry the 16 bytes.
//!
//! ```
//! use prost::Message;
//! use prost_uuid_bytes::ProstUuid;
//!
//! let id: ProstUuid = "6ba7b810-9dad-11d1-80b4-00c04fd430c8".parse()?;
//! let wire = id.encode_to_vec();
//! assert_eq!(wire.len(), 18); // key + length + 16 bytes
//! assert_eq!(ProstUuid::decode(&wire[..])?, id);
//! assert_eq!(id.to_string(), "6ba7b810-9dad-11d1-80b4-00c04fd430c8");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use core::fmt;

use derive_more::{AsMut, AsRef, Constructor, Deref, DerefMut, Display, From, FromStr, Into};
use prost::{
    bytes::{Buf, BufMut},
    encoding::{
        check_wire_type, decode_varint, encode_key, encode_varint, encoded_len_varint, key_len,
        skip_field, DecodeContext, WireType,
    },
    DecodeError, Message, Name,
};
use uuid::{Uuid, Variant};

/// Field number of `bytes value` inside the `uuid.Uuid` message.
pub const VALUE_TAG: u32 = 1;

/// Size of a UUID in bytes.
pub const UUID_LEN: usize = 16;

/// The `uuid.proto` definition this crate implements, for downstream
/// `build.rs` scripts that want to write it into `OUT_DIR` and compile
/// against it instead of vendoring a copy.
pub const PROTO_FILE: &str = include_str!("../proto/uuid.proto");

/// Newtype around [`uuid::Uuid`] with a [`prost::Message`] implementation
/// that encodes the UUID as a single `bytes` field.
///
/// Dereferences to `Uuid`, converts with `From`/`Into`, parses any textual
/// UUID form accepted by [`Uuid::parse_str`] via `FromStr`, and displays as
/// lowercase hyphenated RFC 9562 text.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    AsRef,
    AsMut,
    Constructor,
    Deref,
    DerefMut,
    Display,
    From,
    FromStr,
    Into,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ProstUuid(Uuid);

impl ProstUuid {
    /// The nil UUID, `00000000-0000-0000-0000-000000000000`. Also the `Default`.
    pub const fn nil() -> Self {
        Self(Uuid::nil())
    }

    /// The max UUID, `ffffffff-ffff-ffff-ffff-ffffffffffff`.
    pub const fn max() -> Self {
        Self(Uuid::max())
    }

    /// Wraps 16 bytes in network byte order (no validation needed).
    pub const fn from_bytes(bytes: [u8; UUID_LEN]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }

    /// Builds a UUID from a slice, which must be exactly 16 bytes long.
    ///
    /// This is the same check the protobuf decoder performs.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        let arr: [u8; UUID_LEN] = bytes
            .try_into()
            .map_err(|_| Error::InvalidLength(bytes.len()))?;
        Ok(Self::from_bytes(arr))
    }

    /// Like [`from_slice`](Self::from_slice), but additionally requires the
    /// RFC 9562 variant and version bits (see [`validate_rfc9562`](Self::validate_rfc9562)).
    pub fn from_slice_strict(bytes: &[u8]) -> Result<Self, Error> {
        let id = Self::from_slice(bytes)?;
        id.validate_rfc9562()?;
        Ok(id)
    }

    /// The 16 bytes in network byte order.
    pub const fn as_bytes(&self) -> &[u8; UUID_LEN] {
        self.0.as_bytes()
    }

    /// Unwraps the inner [`Uuid`].
    pub const fn into_inner(self) -> Uuid {
        self.0
    }

    /// Whether this UUID is well-formed according to RFC 9562: either the
    /// nil or max UUID, or variant `10xx` with a version in `1..=8`.
    pub fn is_rfc9562(&self) -> bool {
        self.validate_rfc9562().is_ok()
    }

    /// Checks the RFC 9562 variant and version bits.
    ///
    /// The nil and max UUIDs are accepted (RFC 9562 §5.9, §5.10). Everything
    /// else must carry the `10xx` variant and a version between 1 and 8.
    /// UUIDs using the NCS, Microsoft, or reserved-future variants fail with
    /// [`Error::InvalidVariant`]; an unassigned version fails with
    /// [`Error::InvalidVersion`].
    ///
    /// ```
    /// use prost_uuid_bytes::{Error, ProstUuid};
    ///
    /// let v1: ProstUuid = "6ba7b810-9dad-11d1-80b4-00c04fd430c8".parse()?;
    /// assert!(v1.is_rfc9562());
    /// assert!(ProstUuid::nil().is_rfc9562());
    ///
    /// let junk = ProstUuid::from_bytes([0x11; 16]); // variant bits 00xx
    /// assert_eq!(junk.validate_rfc9562(), Err(Error::InvalidVariant(uuid::Variant::NCS)));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn validate_rfc9562(&self) -> Result<(), Error> {
        if self.0.is_nil() || self.0.is_max() {
            return Ok(());
        }
        match self.0.get_variant() {
            Variant::RFC4122 => {}
            other => return Err(Error::InvalidVariant(other)),
        }
        match self.0.get_version_num() {
            1..=8 => Ok(()),
            v => Err(Error::InvalidVersion(v)),
        }
    }
}

impl Message for ProstUuid {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        encode_key(VALUE_TAG, WireType::LengthDelimited, buf);
        encode_varint(UUID_LEN as u64, buf);
        buf.put_slice(self.0.as_bytes());
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        if tag != VALUE_TAG {
            return skip_field(wire_type, tag, buf, ctx);
        }
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let len = decode_varint(buf)?;
        if len != UUID_LEN as u64 {
            return Err(decode_error(Error::InvalidLength(len as usize)));
        }
        if buf.remaining() < UUID_LEN {
            return Err(decode_error("buffer underflow"));
        }
        let mut bytes = [0u8; UUID_LEN];
        buf.copy_to_slice(&mut bytes);
        self.0 = Uuid::from_bytes(bytes);
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        key_len(VALUE_TAG) + encoded_len_varint(UUID_LEN as u64) + UUID_LEN
    }

    /// Resets to [`ProstUuid::nil`].
    fn clear(&mut self) {
        self.0 = Uuid::nil();
    }
}

/// `DecodeError::new` is deprecated in prost 0.14, but `DecodeErrorKind` is
/// crate-private, so it is still the only way for a foreign `Message` impl to
/// report a custom decode failure. Confined here so the allow is in one place.
#[allow(deprecated)]
fn decode_error(msg: impl fmt::Display) -> DecodeError {
    DecodeError::new(msg.to_string())
}

impl Name for ProstUuid {
    const NAME: &'static str = "Uuid";
    const PACKAGE: &'static str = "uuid";
}

impl From<[u8; UUID_LEN]> for ProstUuid {
    fn from(bytes: [u8; UUID_LEN]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<ProstUuid> for [u8; UUID_LEN] {
    fn from(id: ProstUuid) -> Self {
        id.0.into_bytes()
    }
}

impl TryFrom<&[u8]> for ProstUuid {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Error> {
        Self::from_slice(bytes)
    }
}

impl PartialEq<Uuid> for ProstUuid {
    fn eq(&self, other: &Uuid) -> bool {
        self.0 == *other
    }
}

impl PartialEq<ProstUuid> for Uuid {
    fn eq(&self, other: &ProstUuid) -> bool {
        *self == other.0
    }
}

/// Validation errors for [`ProstUuid`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// The byte payload was not exactly 16 bytes; carries the actual length.
    InvalidLength(usize),
    /// The variant bits are not the RFC 9562 `10xx` pattern.
    InvalidVariant(Variant),
    /// The version nibble is outside `1..=8`.
    InvalidVersion(usize),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidLength(len) => {
                write!(
                    f,
                    "invalid UUID length: expected {UUID_LEN} bytes, got {len}"
                )
            }
            Error::InvalidVariant(v) => {
                write!(f, "invalid UUID variant: {v:?} (expected RFC 9562)")
            }
            Error::InvalidVersion(v) => {
                write!(f, "invalid UUID version: {v} (expected 1 through 8)")
            }
        }
    }
}

impl std::error::Error for Error {}
