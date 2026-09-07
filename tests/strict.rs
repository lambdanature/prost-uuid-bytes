use prost_uuid_bytes::{Error, ProstUuid};
use uuid::{Uuid, Variant};

#[test]
fn well_formed_uuids_pass() {
    let ok = [
        ProstUuid::nil(),
        ProstUuid::max(),
        Uuid::new_v4().into(),
        Uuid::now_v7().into(),
        "6ba7b810-9dad-11d1-80b4-00c04fd430c8".parse().unwrap(), // v1
        Uuid::new_v3(&Uuid::NAMESPACE_DNS, b"example.com").into(),
        Uuid::new_v5(&Uuid::NAMESPACE_DNS, b"example.com").into(),
        Uuid::new_v8([0x42; 16]).into(),
    ];
    for id in ok {
        assert_eq!(id.validate_rfc9562(), Ok(()), "{id}");
        assert!(id.is_rfc9562());
    }
}

fn with_bits(variant_byte: u8, version_nibble: u8) -> ProstUuid {
    let mut b = [0x5a; 16];
    b[6] = (version_nibble << 4) | (b[6] & 0x0f);
    b[8] = variant_byte;
    ProstUuid::from_bytes(b)
}

#[test]
fn bad_variants_fail() {
    let cases = [
        (0x00, Variant::NCS),
        (0x7f, Variant::NCS),
        (0xc0, Variant::Microsoft),
        (0xdf, Variant::Microsoft),
        (0xe0, Variant::Future),
        (0xff, Variant::Future),
    ];
    for (byte, variant) in cases {
        let id = with_bits(byte, 4);
        assert_eq!(
            id.validate_rfc9562(),
            Err(Error::InvalidVariant(variant)),
            "{id}"
        );
        assert!(!id.is_rfc9562());
    }
}

#[test]
fn bad_versions_fail() {
    for v in [0u8, 9, 10, 15] {
        let id = with_bits(0x80, v);
        assert_eq!(
            id.validate_rfc9562(),
            Err(Error::InvalidVersion(v as usize)),
            "{id}"
        );
    }
    for v in 1..=8u8 {
        assert!(with_bits(0xbf, v).is_rfc9562(), "v{v}");
    }
}

#[test]
fn from_slice_variants() {
    let good = Uuid::new_v4();
    assert_eq!(ProstUuid::from_slice(good.as_bytes()).unwrap(), good);
    assert_eq!(ProstUuid::from_slice_strict(good.as_bytes()).unwrap(), good);
    assert_eq!(ProstUuid::from_slice(&[0; 3]), Err(Error::InvalidLength(3)));
    assert_eq!(
        ProstUuid::from_slice_strict(&[0; 17]),
        Err(Error::InvalidLength(17))
    );
    let ncs = [0u8; 16].map(|_| 0x11);
    assert!(ProstUuid::from_slice(&ncs).is_ok());
    assert_eq!(
        ProstUuid::from_slice_strict(&ncs),
        Err(Error::InvalidVariant(Variant::NCS))
    );
}

#[test]
fn error_messages() {
    assert_eq!(
        Error::InvalidLength(3).to_string(),
        "invalid UUID length: expected 16 bytes, got 3"
    );
    assert_eq!(
        Error::InvalidVariant(Variant::NCS).to_string(),
        "invalid UUID variant: NCS (expected RFC 9562)"
    );
    assert_eq!(
        Error::InvalidVersion(9).to_string(),
        "invalid UUID version: 9 (expected 1 through 8)"
    );
    let _: &dyn std::error::Error = &Error::InvalidVersion(0);
}
