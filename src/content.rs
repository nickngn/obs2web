use comrak::{ComrakOptions, ComrakRenderOptions, ListStyleType};
use gray_matter::engine::YAML;
use gray_matter::Matter;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tera::{Context, Tera};
use crate::domain::{Frontmatter, Note};

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

pub fn make_comrak_options() -> ComrakOptions {
    let mut comrak_options = ComrakOptions::default();
    comrak_options.extension.table = true;
    comrak_options.extension.autolink = true;
    comrak_options.extension.tagfilter = true;
    comrak_options.extension.strikethrough = true;
    comrak_options.extension.tasklist = true;
    comrak_options.parse.smart = true;
    let mut render_options = ComrakRenderOptions::default();
    render_options.unsafe_ = true;
    render_options.list_style=ListStyleType::Plus;
    comrak_options.render = render_options;
    comrak_options
}

pub fn process_markdown_file(
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
    let html_content = comrak::markdown_to_html(&content_with_links, comrak_options);

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
        if let Some(_name) = parent_rel_name {
            let file_name = path.file_name().unwrap_or_default().to_str().unwrap()
                .replace("?", "");
            output_path = output_dir.join(file_name);
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
        path: html_path.to_path_buf(),
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
    context.insert("relative_path", &href_to_root_style_css(&output_dir));
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

fn href_to_root_style_css<P: AsRef<Path>>(file_path: P) -> String {
    let path = file_path.as_ref();
    let depth = path.parent().map(|p| p.components().count()).unwrap_or(0);

    if depth == 0 {
        // For files in the root (e.g., "123.md"), base.html will do "./style.css"
        ".".to_string()
    } else {
        // Build "../" repeated `depth` times, but without the trailing slash at the end
        // because base.html adds "/style.css".
        let mut s = String::with_capacity(3 * depth - 1);
        for i in 0..depth {
            s.push_str("..");
            if i + 1 != depth {
                s.push_str("/");
            }
        }
        s
    }
}
