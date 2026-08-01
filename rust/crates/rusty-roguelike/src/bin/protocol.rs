use std::{env, fs, path::PathBuf};

use anyhow::{bail, Context, Result};

fn main() -> Result<()> {
    let mut check = false;
    let mut destination: Option<PathBuf> = None;
    for argument in env::args().skip(1) {
        if argument == "--check" {
            check = true;
        } else if destination.replace(PathBuf::from(argument)).is_some() {
            bail!("expected one protocol destination");
        }
    }
    let destination = destination.context("expected a protocol destination")?;
    let generated = rusty_roguelike::generated_typescript();

    if check {
        let current = fs::read_to_string(&destination)
            .with_context(|| format!("could not read {}", destination.display()))?;
        if current != generated {
            bail!(
                "{} does not match the Rust protocol owner; run protocol:generate",
                destination.display()
            );
        }
    } else {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&destination, generated)
            .with_context(|| format!("could not write {}", destination.display()))?;
    }
    Ok(())
}
