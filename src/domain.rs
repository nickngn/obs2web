use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Debug, Serialize)]
pub struct Frontmatter {
    pub title: Option<String>,
    pub date: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Node {
    pub nodes: Vec<Node>,
    pub title: String,
    pub notes: Vec<Note>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Note {
    pub title: String,
    pub path: PathBuf,
}
