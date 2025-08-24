use std::collections::HashMap;
use std::path::Path;
use tera::{Context, Tera};
use crate::domain::{Note, Node};
use std::collections::VecDeque;
use std::fs;

pub fn init_tera() -> std::io::Result<Tera> {
    Tera::new("templates/**/*.html").map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to initialize templates: {e}"),
        )
    })
}

pub fn render_index(tera: &Tera, output_dir: &Path, notes: &[Note]) -> std::io::Result<()> {
    let mut context = Context::new();

    let notes_tree = initiate_nodes_tree(notes.to_vec(), output_dir);

    context.insert("nodes", &notes_tree);
    let index_html = tera.render("index.html", &context).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Template rendering failed for index.html: {e:?}"),
        )
    })?;
    let index_path = output_dir.join("index.html");
    fs::write(index_path, index_html)?;
    Ok(())
}

pub fn render_tag_pages(
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
                format!("Template rendering failed for tag.html (tag=\"{}\"): {e}", tag),
            )
        })?;
        let tag_path = tags_dir.join(format!("{}.html", tag));
        fs::write(tag_path, tag_html)?;
    }
    Ok(())
}

fn initiate_nodes_tree(notes: Vec<Note>, output_dir: &Path) -> Node {
    let mut root_node = Node {
        nodes: Vec::new(),
        title: output_dir.to_str().unwrap().to_string(),
        notes: Vec::new(),
    };
    notes.iter().for_each(|n| {
        let mut parts = n.path.to_str().unwrap().split("/").collect::<VecDeque<&str>>();
        parts.pop_back();
        parts.pop_front();
        let node_ref = find_or_create_node(parts, &mut root_node);
        let mut note = n.clone();
        note.path = note.path.strip_prefix(output_dir).unwrap().to_path_buf();
        node_ref.notes.push(note);
    });
    root_node
}

fn find_or_create_node<'a>(mut path_parts: VecDeque<&str>, node: &'a mut Node) -> &'a mut Node {
    if path_parts.is_empty() {
        return node;
    }
    let cur_folder = path_parts.pop_front().unwrap();
    // Find index first to avoid overlapping mutable borrows
    let idx = match node.nodes.iter().position(|n| n.title == cur_folder) {
        Some(i) => i,
        None => {
            node.nodes.push(Node {
                nodes: Vec::new(),
                title: cur_folder.to_string(),
                notes: Vec::new(),
            });
            node.nodes.len() - 1
        }
    };

    let child = &mut node.nodes[idx];
    find_or_create_node(path_parts, child)
}
