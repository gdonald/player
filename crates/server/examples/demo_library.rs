use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let directory: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("usage: demo_library <directory>"))?;

    player_server::fixtures::write_demo_library(&directory)?;
    println!("wrote demo library to {}", directory.display());

    Ok(())
}
