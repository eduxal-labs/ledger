use std::error::Error;
use tonic_prost_build::configure;

fn main() -> Result<(), Box<dyn Error>> {
    env();
    configure()
        .build_client(false)
        .compile_well_known_types(true)
        .extern_path(
            ".google.protobuf.Timestamp",
            "crate::proto::time::Timestamp",
        )
        .extern_path(".google.protobuf.Empty", "()")
        .compile_protos(
            &[
                "./protos/services/authentication.proto",
                "./protos/services/sync.proto",
                "./protos/services/ai_marking.proto",
                "./protos/services/question_bank.proto",
                "./protos/types/role.proto",
                "./protos/types/member.proto",
            ],
            &["./protos/"],
        )?;
    Ok(())
}

fn env() {
    if let Ok(content) = std::fs::read_to_string(".env") {
        for line in content.lines() {
            // Parse KEY=VALUE pairs
            if let Some((key, value)) = line.split_once('=') {
                // Set as cargo environment variable for compile-time access
                println!("cargo:rustc-env={}={}", key.trim(), value.trim());
            }
        }
    }
}
