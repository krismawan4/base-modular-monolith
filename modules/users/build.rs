use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../../proto/users.proto");

    if let Err(err) = tonic_build::compile_protos("../../proto/users.proto") {
        eprintln!("Warning: tonic_build failed ({err}). Falling back to pre-generated proto file...");
        let out_dir = env::var("OUT_DIR")?;
        let dest = Path::new(&out_dir).join("users.rs");
        let pregenerated = Path::new("src/internal/proto_gen/users.rs");
        if pregenerated.exists() {
            fs::copy(pregenerated, dest)?;
        } else {
            return Err(err.into());
        }
    }
    Ok(())
}
