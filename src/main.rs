use clap::Parser;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use comrak::{markdown_to_html, ComrakOptions, ComrakRenderOptions};
use std::fs;
use serde::{Deserialize, Serialize};
use gray_matter::Matter;
use gray_matter::engine::YAML;
use tera::{Tera, Context};
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the Obsidian vault
    #[arg(short, long)]
    vault_path: PathBuf,

    /// Path to the output directory
    #[arg(short, long)]
    output_dir: PathBuf,
}

#[derive(Deserialize, Debug, Serialize)]
struct Frontmatter {
    title: Option<String>,
    date: Option<String>,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Clone)]
struct Note {
    title: String,
    path: PathBuf,
}

fn rewrite_links(content: &str) -> String {
    let mut new_content = String::new();
    let mut last_index = 0;
    let mut in_link = false;
    let mut in_asset = false;
    let mut link_text = String::new();

    for (i, c) in content.char_indices() {
        if c == '[' && content.chars().nth(i + 1) == Some('[') {
            if !in_link && !in_asset {
                in_link = true;
                new_content.push_str(&content[last_index..i]);
                last_index = i;
            }
        } else if c == '!' && content.chars().nth(i + 1) == Some('[') && content.chars().nth(i + 2) == Some('[') {
            if !in_link && !in_asset {
                in_asset = true;
                new_content.push_str(&content[last_index..i]);
                last_index = i;
            }
        } else if c == ']' && content.chars().nth(i + 1) == Some(']') {
            if in_link {
                in_link = false;
                let link_slug = link_text.to_lowercase().replace(" ", "-");
                let html_link = format!("<a href=\"{}.html\">{}</a>", link_slug, link_text);
                new_content.push_str(&html_link);
                link_text.clear();
                last_index = i + 2;
            } else if in_asset {
                in_asset = false;
                let html_link = format!("<img src=\"{}\">", link_text);
                new_content.push_str(&html_link);
                link_text.clear();
                last_index = i + 2;
            }
        } else if in_link || in_asset {
            if c != '[' && c != '!' {
                link_text.push(c);
            }
        } else {
            // new_content.push(c);
        }
    }
    new_content.push_str(&content[last_index..]);
    new_content
}

// New helpers
fn init_tera() -> std::io::Result<Tera> {
    Tera::new("templates/**/*.html").map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to initialize templates: {e}"),
        )
    })
}

fn prepare_output_dir(output_dir: &Path) -> std::io::Result<()> {
    // Remove old output and recreate
    if output_dir.exists() {
        println!("Cleaning output directory: {}", output_dir.display());
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(&output_dir)?;
    Ok(())
}

fn make_comrak_options() -> ComrakOptions {
    let mut comrak_options = ComrakOptions::default();
    let mut render_options = ComrakRenderOptions::default();
    render_options.unsafe_ = true;
    comrak_options.render = render_options;
    comrak_options
}

fn process_markdown_file(
    path: &Path,
    output_dir: &Path,
    tera: &Tera,
    comrak_options: &ComrakOptions,
    notes: &mut Vec<Note>,
    tags: &mut HashMap<String, Vec<Note>>,
) -> std::io::Result<()> {
    // Compute output path next to output_dir using the vault-relative location
    // The caller guarantees parent dirs exist.
    println!("Converting markdown: {}", path.display());

    let markdown_content = fs::read_to_string(path)?;
    let matter = Matter::<YAML>::new();
    let result = matter.parse(&markdown_content);

    let (frontmatter, content) = match result.data {
        Some(data) => {
            let fm = data.deserialize::<Frontmatter>().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Frontmatter deserialize error in {}: {e}", path.display()),
                )
            })?;
            (Some(fm), result.content)
        }
        None => (None, result.content),
    };

    let content_with_links = rewrite_links(&content);
    let html_content = markdown_to_html(&content_with_links, comrak_options);

    let mut context = Context::new();
    let fallback_title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string();
    let title = if let Some(fm) = &frontmatter {
        fm.title.clone().unwrap_or_else(|| fallback_title.clone())
    } else {
        fallback_title.clone()
    };

    // Compute output html path
    // We need to mirror the directory structure from the vault into output_dir.
    // So we take the file path relative to the vault root; the caller provides output path base.
    // For this helper, we rebuild relative to the vault by scanning for the first component after the vault path is handled by caller.
    let mut output_path = output_dir.join(path.file_name().unwrap_or_default());
    // Try to reconstruct relative path using canonicalization when possible
    // If the parent folder exists under output_dir, keep same structure:
    if let Some(parent) = path.parent() {
        let rel = parent; // caller ensures directories
        let parent_rel_name = rel.file_name();
        if let Some(name) = parent_rel_name {
            output_path = output_dir.join(name).join(path.file_name().unwrap_or_default());
            // Ensure parent exists
            if let Some(parent_out) = output_path.parent() {
                fs::create_dir_all(parent_out)?;
            }
        }
    }

    let mut html_path = output_path.clone();
    html_path.set_extension("html");

    let note = Note {
        title: title.clone(),
        path: html_path.strip_prefix(output_dir).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Failed to compute output relative path: couldn't strip '{}' from '{}': {e}",
                    output_dir.display(),
                    html_path.display()
                ),
            )
        })?.to_path_buf(),
    };

    if let Some(fm) = frontmatter {
        context.insert("title", &title);
        context.insert("date", &fm.date);
        context.insert("tags", &fm.tags);
        if let Some(tag_list) = fm.tags {
            for tag in tag_list {
                tags.entry(tag).or_default().push(note.clone());
            }
        }
    } else {
        context.insert("title", &title);
    }
    context.insert("relative_path", html_path.strip_prefix(output_dir).unwrap());
    context.insert("content", &html_content);

    let rendered_html = tera.render("base.html", &context).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Template rendering failed for base.html: {e}"),
        )
    })?;

    fs::write(&html_path, rendered_html)?;
    println!("Wrote HTML: {}", html_path.display());

    notes.push(note);
    Ok(())
}

fn process_asset(path: &Path, output_path: &Path) -> std::io::Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    println!("Copying asset: {} -> {}", path.display(), output_path.display());
    fs::copy(path, output_path)?;
    Ok(())
}

fn render_index(tera: &Tera, output_dir: &Path, notes: &[Note]) -> std::io::Result<()> {
    let mut context = Context::new();
    context.insert("notes", &notes);
    let index_html = tera.render("index.html", &context).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Template rendering failed for index.html: {e}"),
        )
    })?;
    let index_path = output_dir.join("index.html");
    fs::write(index_path, index_html)?;
    Ok(())
}

fn render_tag_pages(
    tera: &Tera,
    output_dir: &Path,
    tags: HashMap<String, Vec<Note>>,
) -> std::io::Result<()> {
    let tags_dir = output_dir.join("tags");
    fs::create_dir_all(&tags_dir)?;
    for (tag, notes) in tags {
        let mut context = Context::new();
        context.insert("tag", &tag);
        context.insert("notes", &notes);
        let tag_html = tera.render("tag.html", &context).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Template rendering failed for tag.html (tag='{}'): {e}", tag),
            )
        })?;
        let tag_path = tags_dir.join(format!("{}.html", tag));
        fs::write(tag_path, tag_html)?;
    }
    Ok(())
}

fn build_site(vault_path: &Path, output_dir: &Path) -> std::io::Result<()> {
    println!("Building site...");

    let tera = init_tera()?;
    prepare_output_dir(output_dir)?;
    let comrak_options = make_comrak_options();

    let mut notes: Vec<Note> = Vec::new();
    let mut tags: HashMap<String, Vec<Note>> = HashMap::new();

    for entry in WalkDir::new(&vault_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }

        // Preserve relative structure under output_dir
        let relative_path = path.strip_prefix(&vault_path).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Failed to compute relative path: couldn't strip '{}' from '{}': {e}",
                    vault_path.display(),
                    path.display()
                ),
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

    fs::copy("templates/style.css", output_dir.join("style.css")).unwrap();
    render_index(&tera, output_dir, &notes)?;
    // render_tag_pages(&tera, output_dir, tags)?;

    println!("Site built successfully.");
    Ok(())
}
// ... existing code ...
fn main() -> std::io::Result<()> {
    let args = Args::parse();

    build_site(&args.vault_path, &args.output_dir)?;

    Ok(())
}