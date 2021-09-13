use anyhow::Result;
use cargo_toml::{Manifest, Package};
use std::{env, path::Path};

pub struct CrateInfo {
    pub package: Package,
}

impl CrateInfo {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let manifest = Manifest::from_path(path.as_ref().join("Cargo.toml"))?;
        Ok(Self {
            package: manifest
                .package
                .ok_or_else(|| anyhow::anyhow!("Not a binary package"))?,
        })
    }

    pub fn crate_name(&self) -> &str {
        &self.package.name
    }

    pub fn cargo_target(&self) -> String {
        // TODO: better detection
        if let Ok(t) = env::var("CARGO_TARGET_DIR") {
            return t;
        }
        if Path::new("/opt/cargo/target").is_dir() {
            return "/opt/cargo/target".to_owned();
        }
        return "./target".to_owned();
    }
}
