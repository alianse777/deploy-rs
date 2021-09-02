use anyhow::Result;
use std::path::Path;

pub fn build_frontend_if_exists<P: AsRef<Path>>(path: P) -> Result<bool> {
    let path = path.as_ref();
    if !path.is_dir() {
        log::warn!("No npm package found in {:?}, skipping npm build...", path);
        return Ok(false);
    }
    log::info!("Building npm package: {:?}", path);
    cmd_lib::run_cmd!(cd "$path"; npm run build)?;
    Ok(true)
}

pub fn build_backend(features: Option<Vec<String>>, musl: bool) -> Result<()> {
    let path = Path::new(".");
    if !path.join("Cargo.toml").is_file() {
        return Err(anyhow::anyhow!("No Cargo.toml found"));
    }
    let features_str = match features {
        Some(f) => f.join(","),
        None => "".to_owned(),
    };
    let target = if musl {
        "x86_64-unknown-linux-musl"
    } else {
        "x86_64-unknown-linux-gnu"
    };
    log::info!("Building backend");
    cmd_lib::run_cmd!(cargo build --release --target="$target" --no-default-features --features "$features_str")?;
    Ok(())
}
