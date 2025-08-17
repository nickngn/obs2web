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

    /// Serve the output directory
    #[arg(short, long)]
    serve: bool,
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

fn build_site(vault_path: &Path, output_dir: &Path) -> std::io::Result<()> {
    println!("Building site...");

    // Initialize Tera
    let tera = Tera::new("templates/**/*.html").unwrap();

    // Create the output directory if it doesn't exist
    fs::create_dir_all(&output_dir)?;

    let mut comrak_options = ComrakOptions::default();
    let mut render_options = ComrakRenderOptions::default();
    render_options.unsafe_ = true;
    comrak_options.render = render_options;

    let mut notes: Vec<Note> = Vec::new();
    let mut tags: HashMap<String, Vec<Note>> = HashMap::new();

    for entry in WalkDir::new(&vault_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }

        let relative_path = path.strip_prefix(&vault_path).unwrap();
        let output_path = output_dir.join(relative_path);

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let markdown_content = fs::read_to_string(path)?;
            let matter = Matter::<YAML>::new();
            let result = matter.parse(&markdown_content);

            let (frontmatter, content) = match result.data {
                Some(data) => (Some(data.deserialize::<Frontmatter>().unwrap()), result.content),
                None => (None, result.content),
            };

            let content_with_links = rewrite_links(&content);
            let html_content = markdown_to_html(&content_with_links, &comrak_options);

            let mut context = Context::new();
            let title = if let Some(fm) = &frontmatter {
                fm.title.clone().unwrap_or_else(|| path.file_stem().unwrap().to_string_lossy().to_string())
            } else {
                path.file_stem().unwrap().to_string_lossy().to_string()
            };

            let note = Note {
                title: title.clone(),
                path: output_path.strip_prefix(&output_dir).unwrap().with_extension("html").to_path_buf(),
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
            context.insert("content", &html_content);

            let mut html_path = output_path;
            html_path.set_extension("html");

            notes.push(note);

            let rendered_html = tera.render("base.html", &context).unwrap();

            fs::write(&html_path, rendered_html)?;
        } else {
            fs::copy(path, &output_path)?;
        }
    }

    // Render index page
    let mut context = Context::new();
    context.insert("notes", &notes);
    let index_html = tera.render("index.html", &context).unwrap();
    let index_path = output_dir.join("index.html");
    fs::write(index_path, index_html)?;

    // Render tag pages
    let tags_dir = output_dir.join("tags");
    fs::create_dir_all(&tags_dir)?;
    for (tag, notes) in tags {
        let mut context = Context::new();
        context.insert("tag", &tag);
        context.insert("notes", &notes);
        let tag_html = tera.render("tag.html", &context).unwrap();
        let tag_path = tags_dir.join(format!("{}.html", tag));
        fs::write(tag_path, tag_html)?;
    }

    println!("Site built successfully.");

    Ok(())
}


fn main() -> std::io::Result<()> {
    let args = Args::parse();

    build_site(&args.vault_path, &args.output_dir)?;

    if args.serve {
        println!("Starting server...");
        // Server logic will go here
    }

    Ok(())
}
