use std::collections::HashMap;
use std::path::{Path, PathBuf};
use clap::Parser;
use walkdir::WalkDir;
use crate::content::{make_comrak_options, process_markdown_file};
use crate::domain::Note;
use crate::fs::{prepare_output_dir, process_asset};
use crate::template::{init_tera, render_index};

pub mod domain;
pub mod template;
pub mod content;
pub mod fs;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the Obsidian vault
    #[arg(short, long)]
    pub vault_path: PathBuf,

    /// Path to the output directory
    #[arg(short, long)]
    pub output_dir: PathBuf,
}

pub fn build_site(vault_path: &Path, output_dir: &Path) -> std::io::Result<()> {
    println!("Building site...");

    let tera = init_tera()?;
    prepare_output_dir(output_dir)?;
    let comrak_options = make_comrak_options();

    let mut notes: Vec<Note> = Vec::new();
    let mut tags: HashMap<String, Vec<Note>> = HashMap::new();

    for entry in WalkDir::new(vault_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }

        // Preserve relative structure under output_dir
        let relative_path = path.strip_prefix(vault_path).map_err(|_e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to compute relative path",
            )
        })?;
        let output_path = output_dir.join(relative_path);

        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            process_markdown_file(
                path,
                &output_dir.join(relative_path.parent().unwrap_or_else(|| Path::new(""))),
                &tera,
                &comrak_options,
                &mut notes,
                &mut tags,
            )?;
        } else {
            process_asset(path, &output_path)?;
        }
    }

    std::fs::copy("templates/style.css", output_dir.join("style.css")).unwrap();
    render_index(&tera, output_dir, &notes)?;
    // render_tag_pages(&tera, output_dir, tags)?;

    println!("Site built successfully.");
    Ok(())
}