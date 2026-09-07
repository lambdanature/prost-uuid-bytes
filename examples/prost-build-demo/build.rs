//! Compiles `user.proto` twice: once mapping `uuid.Uuid` to `ProstUuid`
//! (the way a real project would), and once vanilla so `main.rs` can prove
//! that both produce byte-identical output.

use std::{env, fs, path::PathBuf};

fn main() -> std::io::Result<()> {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    // `uuid.proto` ships inside the prost-uuid-bytes crate as a string, so
    // downstream users can materialize it instead of vendoring a copy.
    let include = out.join("include");
    fs::create_dir_all(&include)?;
    fs::write(include.join("uuid.proto"), prost_uuid_bytes::PROTO_FILE)?;

    let protos = ["proto/user.proto"];
    let includes = ["proto".into(), include];

    let with_extern = out.join("with_extern");
    fs::create_dir_all(&with_extern)?;
    prost_build::Config::new()
        .out_dir(&with_extern)
        .extern_path(".uuid.Uuid", "::prost_uuid_bytes::ProstUuid")
        .compile_protos(&protos, &includes)?;

    let plain = out.join("plain");
    fs::create_dir_all(&plain)?;
    prost_build::Config::new()
        .out_dir(&plain)
        .compile_protos(&protos, &includes)?;

    println!("cargo:rerun-if-changed=proto/user.proto");
    Ok(())
}
