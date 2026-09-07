use prost::{Message, Name};
use prost_uuid_bytes::{ProstUuid, UUID_LEN, VALUE_TAG};
use uuid::Uuid;

const SAMPLE: &str = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";

fn sample() -> ProstUuid {
    SAMPLE.parse().unwrap()
}

#[test]
fn wire_layout_is_tag_len_16_bytes() {
    let id = sample();
    let wire = id.encode_to_vec();
    assert_eq!(wire.len(), 18);
    assert_eq!(wire[0], (VALUE_TAG << 3 | 2) as u8); // field 1, length-delimited
    assert_eq!(wire[1], UUID_LEN as u8);
    assert_eq!(&wire[2..], id.as_bytes());
    assert_eq!(id.encoded_len(), wire.len());
}

#[test]
fn round_trips() {
    for id in [
        sample(),
        ProstUuid::nil(),
        ProstUuid::max(),
        Uuid::new_v4().into(),
        Uuid::now_v7().into(),
        ProstUuid::from_bytes([0xc0; 16]), // not RFC 9562, still round-trips
    ] {
        let wire = id.encode_to_vec();
        assert_eq!(ProstUuid::decode(&wire[..]).unwrap(), id, "{id}");
        assert_eq!(wire.len(), id.encoded_len());
    }
}

#[test]
fn nil_is_default_and_is_encoded_explicitly() {
    assert_eq!(ProstUuid::default(), ProstUuid::nil());
    assert_eq!(ProstUuid::nil().encoded_len(), 18);
    let mut id = sample();
    id.clear();
    assert_eq!(id, ProstUuid::nil());
}

#[test]
fn decode_rejects_wrong_lengths() {
    for len in [0usize, 1, 15, 17, 32] {
        let mut wire = vec![0x0a, len as u8];
        wire.extend(std::iter::repeat_n(0xab, len));
        let err = ProstUuid::decode(&wire[..]).unwrap_err();
        assert!(
            err.to_string().contains(&format!("got {len}")),
            "len {len}: {err}"
        );
    }
}

#[test]
fn decode_rejects_truncated_buffer() {
    let wire = sample().encode_to_vec();
    assert!(ProstUuid::decode(&wire[..10]).is_err());
}

#[test]
fn decode_rejects_wrong_wire_type() {
    // field 1 as a varint instead of length-delimited
    let wire = [0x08, 0x01];
    let err = ProstUuid::decode(&wire[..]).unwrap_err();
    assert!(err.to_string().contains("wire type"), "{err}");
}

#[test]
fn decode_skips_unknown_fields_and_last_one_wins() {
    let a = sample();
    let b: ProstUuid = Uuid::new_v4().into();
    let mut wire = Vec::new();
    prost::encoding::uint64::encode(7, &42, &mut wire); // unknown field
    a.encode_raw(&mut wire);
    prost::encoding::string::encode(9, &"x".to_owned(), &mut wire); // unknown field
    b.encode_raw(&mut wire);
    assert_eq!(ProstUuid::decode(&wire[..]).unwrap(), b);
}

#[test]
fn works_as_embedded_message_field() {
    #[derive(Clone, PartialEq, Message)]
    struct Wrapper {
        #[prost(message, optional, tag = "3")]
        id: Option<ProstUuid>,
        #[prost(message, repeated, tag = "4")]
        ids: Vec<ProstUuid>,
    }

    let w = Wrapper {
        id: Some(sample()),
        ids: vec![ProstUuid::nil(), Uuid::new_v4().into()],
    };
    let wire = w.encode_to_vec();
    assert_eq!(wire.len(), 3 * (2 + 18));
    assert_eq!(Wrapper::decode(&wire[..]).unwrap(), w);
}

#[test]
fn name_and_type_url() {
    assert_eq!(ProstUuid::full_name(), "uuid.Uuid");
    assert_eq!(ProstUuid::type_url(), "/uuid.Uuid");
}

#[test]
fn conversions_and_display() {
    let raw = Uuid::parse_str(SAMPLE).unwrap();
    let id = ProstUuid::from(raw);
    assert_eq!(Uuid::from(id), raw);
    assert_eq!(id, raw);
    assert_eq!(raw, id);
    assert_eq!(*id, raw);
    assert_eq!(id.to_string(), SAMPLE);
    assert_eq!(format!("{id}"), format!("{raw}"));
    assert_eq!(ProstUuid::from(*raw.as_bytes()), id);
    assert_eq!(<[u8; 16]>::from(id), *raw.as_bytes());
    assert_eq!(ProstUuid::try_from(&raw.as_bytes()[..]).unwrap(), id);
    assert!(ProstUuid::try_from(&raw.as_bytes()[..15]).is_err());
    assert_eq!(id.into_inner(), raw);
    assert_eq!(ProstUuid::new(raw), id);

    // FromStr accepts every form the uuid crate does.
    for s in [
        "6ba7b8109dad11d180b400c04fd430c8",
        "{6ba7b810-9dad-11d1-80b4-00c04fd430c8}",
        "urn:uuid:6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "6BA7B810-9DAD-11D1-80B4-00C04FD430C8",
    ] {
        assert_eq!(s.parse::<ProstUuid>().unwrap(), id, "{s}");
    }
    assert!("6ba7b810-9dad-11d1-80b4".parse::<ProstUuid>().is_err());
}
