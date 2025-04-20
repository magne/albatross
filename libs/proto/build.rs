use std::io::Result;

fn main() -> Result<()> {
    // Directory where .proto files are located
    let proto_dir = "src/proto";

    // Find all .proto files in the specified directory
    let proto_files: Vec<_> = std::fs::read_dir(proto_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() && path.extension()? == "proto" {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    if proto_files.is_empty() {
        println!(
            "cargo:warning=No .proto files found in {}, skipping prost build.",
            proto_dir
        );
        return Ok(());
    }

    println!("cargo:rerun-if-changed={}", proto_dir);
    for proto_file in &proto_files {
        println!("cargo:rerun-if-changed={}", proto_file.display());
    }

    // Configure prost_build
    let mut config = prost_build::Config::new();

    // Add serde::Serialize derive to all generated types
    config.type_attribute(".", "#[derive(serde::Serialize)]");
    // Optionally, add Deserialize if needed later:
    // config.type_attribute(".", "#[derive(serde::Deserialize)]");

    // Compile the protos using the configured builder
    config.compile_protos(&proto_files, &[proto_dir])?;

    Ok(())
}
