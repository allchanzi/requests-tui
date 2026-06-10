use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;
use walkdir::{DirEntry, WalkDir};

use super::model::{Auth, Collection, Environment, Node};
use super::{dotenv, parser};

#[derive(Debug, Default, Deserialize)]
struct BrunoJson {
    #[serde(default)]
    name: Option<String>,
}

/// Walk `root` looking for `bruno.json` files and load each one as a [`Collection`].
/// Collections are returned sorted by name. Directories named `node_modules` or `.git`
/// are skipped.
pub fn discover(root: &Path) -> Result<Vec<Collection>> {
    let mut collections = Vec::new();

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            !matches!(
                entry.file_name().to_str(),
                Some("node_modules") | Some(".git")
            ) && !is_linked_worktree(entry)
        })
        .filter_map(Result::ok)
    {
        if entry.file_name() == "bruno.json" {
            if let Some(dir) = entry.path().parent() {
                match load_collection(dir) {
                    Ok(collection) => collections.push(collection),
                    Err(error) => eprintln!("skipping {}: {error:#}", dir.display()),
                }
            }
        }
    }

    collections.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(collections)
}

/// Whether `entry` is the root of a linked git worktree, so its subtree (a duplicate
/// checkout of collections that also live in the main working tree) can be skipped.
/// A linked worktree has `.git` as a *file* pointing at the main repo, whereas the main
/// checkout has it as a directory. The scan root itself (depth 0) is never skipped, so
/// running directly inside a worktree still works.
fn is_linked_worktree(entry: &DirEntry) -> bool {
    entry.file_type().is_dir() && entry.depth() > 0 && entry.path().join(".git").is_file()
}

/// Load a single collection from a directory containing `bruno.json`.
pub fn load_collection(dir: &Path) -> Result<Collection> {
    let manifest_path = dir.join("bruno.json");
    let manifest_raw = fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let manifest: BrunoJson = serde_json::from_str(&manifest_raw)
        .with_context(|| format!("parsing {}", manifest_path.display()))?;

    let (auth, headers, vars) = match fs::read_to_string(dir.join("collection.bru")) {
        Ok(raw) => parser::parse_collection_config(&raw),
        Err(_) => (Auth::None, Vec::new(), Vec::new()),
    };

    let name = manifest.name.filter(|n| !n.is_empty()).unwrap_or_else(|| {
        dir.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "collection".to_string())
    });

    let environments = load_environments(dir);
    let process_env = dotenv::load(dir);
    let nodes = load_nodes(dir, dir);

    Ok(Collection {
        name,
        path: dir.to_path_buf(),
        auth,
        headers,
        vars,
        nodes,
        environments,
        process_env,
    })
}

fn load_environments(dir: &Path) -> Vec<Environment> {
    let env_dir = dir.join("environments");
    let mut environments = Vec::new();

    if let Ok(read) = fs::read_dir(&env_dir) {
        for entry in read.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("bru") {
                continue;
            }
            let name = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .unwrap_or_default();
            if let Ok(raw) = fs::read_to_string(&path) {
                environments.push(parser::parse_environment(&raw, &name));
            }
        }
    }

    environments.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    environments
}

/// Build the request tree for `dir`. Subdirectories become folders (except
/// `environments`), and `*.bru` files (except `collection.bru` / `folder.bru`) become
/// requests. Requests are ordered by `seq` then name; folders by name.
fn load_nodes(collection_root: &Path, dir: &Path) -> Vec<Node> {
    let mut folders: Vec<Node> = Vec::new();
    let mut requests: Vec<(i64, String, Node)> = Vec::new();

    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };

    for entry in read.filter_map(Result::ok) {
        let path = entry.path();
        let file_type = entry.file_type();

        if file_type.map(|ft| ft.is_dir()).unwrap_or(false) {
            if path.file_name().and_then(|n| n.to_str()) == Some("environments") {
                continue;
            }
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let children = load_nodes(collection_root, &path);
            if !children.is_empty() {
                folders.push(Node::Folder { name, children });
            }
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some("bru") {
            continue;
        }
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if matches!(file_name, "collection.bru" | "folder.bru") {
            continue;
        }

        if let Ok(raw) = fs::read_to_string(&path) {
            if let Some(request) = parser::parse_request(&raw, &path) {
                requests.push((request.seq, request.name.to_lowercase(), Node::Request(request)));
            }
        }
    }

    folders.sort_by(|a, b| folder_name(a).cmp(&folder_name(b)));
    requests.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    let mut nodes = folders;
    nodes.extend(requests.into_iter().map(|(_, _, node)| node));
    nodes
}

fn folder_name(node: &Node) -> String {
    match node {
        Node::Folder { name, .. } => name.to_lowercase(),
        Node::Request(request) => request.name.to_lowercase(),
    }
}

/// Resolve a usable starting root: the given path if it exists, else an error.
pub fn resolve_root(root: PathBuf) -> Result<PathBuf> {
    if root.exists() {
        Ok(root)
    } else {
        anyhow::bail!("root path does not exist: {}", root.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create `<dir>/bruno.json` so the directory is discovered as a collection.
    fn write_collection(dir: &Path, name: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("bruno.json"), format!("{{\"name\": \"{name}\"}}")).unwrap();
    }

    #[test]
    fn skips_linked_worktrees_but_keeps_main_checkout() {
        let root = std::env::temp_dir().join(format!("requests-tui-wt-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);

        // Main checkout (`.git` as a directory) plus a linked worktree (`.git` as a file),
        // both holding a same-named collection.
        let main = root.join("main");
        write_collection(&main, "Example Dashboard");
        fs::create_dir_all(main.join(".git")).unwrap();

        let worktree = root.join("worktrees").join("feature-x");
        write_collection(&worktree, "Example Dashboard");
        fs::write(worktree.join(".git"), "gitdir: /repo/.git/worktrees/feature-x").unwrap();

        let collections = discover(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();

        // Only the main checkout's collection survives; the worktree duplicate is skipped.
        assert_eq!(collections.len(), 1);
        assert!(collections[0].path.ends_with("main"));
    }
}
