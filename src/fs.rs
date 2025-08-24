use std::fs;
use std::path::Path;

pub fn prepare_output_dir(output_dir: &Path) -> std::io::Result<()> {
    // Remove old output and recreate
    if output_dir.exists() {
        println!("Cleaning output directory: {}", output_dir.display());
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(&output_dir)?;
    Ok(())
}

pub fn process_asset(path: &Path, output_path: &Path) -> std::io::Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    println!("Copying asset: {} -> {}", path.display(), output_path.display());
    fs::copy(path, output_path)?;
    Ok(())
}
