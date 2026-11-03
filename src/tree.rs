use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::state::{load_dir, Node, NodeKind};

const MAX_DEPTH: usize = 32;
const MAX_ROWS: usize = 2_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub node: Node,
    pub depth: usize,
    pub expanded: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tree {
    pub rows: Vec<Row>,
    pub root: PathBuf,
    roots: Vec<Node>,
    children: HashMap<PathBuf, Vec<Node>>,
    expanded: HashSet<PathBuf>,
}

impl Tree {
    /// Uses the supplied root listing without reading the filesystem.
    /// An oversized initial listing is retained; toggles enforce the row limit.
    pub fn new(root: PathBuf, nodes: &[Node]) -> Self {
        let mut roots = Vec::with_capacity(nodes.len());
        if let Some(parent) = nodes.iter().find(|node| node.kind == NodeKind::Parent) {
            roots.push(parent.clone());
        }
        roots.extend(
            nodes
                .iter()
                .filter(|node| node.kind != NodeKind::Parent)
                .cloned(),
        );
        let rows = roots
            .iter()
            .map(|node| Row {
                node: node.clone(),
                depth: 0,
                expanded: false,
            })
            .collect();
        Self {
            rows,
            root,
            roots,
            children: HashMap::new(),
            expanded: HashSet::new(),
        }
    }

    /// Changes a folder's expansion atomically, retaining cached listings and
    /// descendant expansion choices across collapse. Other rows are not togglable.
    pub fn toggle(&mut self, index: usize) -> Result<usize, String> {
        let row = self.rows.get(index).ok_or("Tree row is out of range")?;
        if row.node.kind != NodeKind::Dir {
            return Err("Only directory rows can be expanded".into());
        }
        let path = row.node.path.clone();
        let mut next = self.clone();
        let opening = !next.expanded.remove(&path);
        if opening {
            next.expanded.insert(path.clone());
            if !next.children.contains_key(&path) {
                let children = load_dir(&path)
                    .map_err(|error| format!("Cannot read {}: {error}", path.display()))?
                    .into_iter()
                    .filter(|node| node.kind != NodeKind::Parent)
                    .collect();
                next.children.insert(path.clone(), children);
            }
        }
        next.rows = next.project(opening)?;
        let selected = next
            .rows
            .iter()
            .position(|row| row.node.path == path && row.node.kind == NodeKind::Dir)
            .ok_or("Toggled directory is missing from the tree")?;
        *self = next;
        Ok(selected)
    }

    fn project(&self, check_cycles: bool) -> Result<Vec<Row>, String> {
        // Collapsing never needs filesystem access, even if a cached folder vanished.
        let mut ancestors = if check_cycles {
            Some(vec![self.root.canonicalize().map_err(|error| {
                format!("Cannot resolve {}: {error}", self.root.display())
            })?])
        } else {
            None
        };
        let mut rows = Vec::new();
        self.append(&self.roots, 0, &mut ancestors, &mut rows)?;
        Ok(rows)
    }

    fn append(
        &self,
        nodes: &[Node],
        depth: usize,
        ancestors: &mut Option<Vec<PathBuf>>,
        rows: &mut Vec<Row>,
    ) -> Result<(), String> {
        for node in nodes {
            if depth > MAX_DEPTH {
                return Err(format!("Tree depth exceeds {MAX_DEPTH}"));
            }
            if rows.len() == MAX_ROWS {
                return Err(format!("Tree row count exceeds {MAX_ROWS}"));
            }
            let expanded = node.kind == NodeKind::Dir && self.expanded.contains(&node.path);
            rows.push(Row {
                node: node.clone(),
                depth,
                expanded,
            });
            if !expanded {
                continue;
            }
            if let Some(chain) = ancestors.as_mut() {
                let canonical = node
                    .path
                    .canonicalize()
                    .map_err(|error| format!("Cannot resolve {}: {error}", node.path.display()))?;
                if chain.contains(&canonical) {
                    return Err(format!(
                        "Directory ancestor cycle at {}",
                        node.path.display()
                    ));
                }
                chain.push(canonical);
            }
            if let Some(children) = self.children.get(&node.path) {
                self.append(children, depth + 1, ancestors, rows)?;
            }
            if let Some(chain) = ancestors.as_mut() {
                chain.pop();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "mobux-tree-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn tree(&self) -> Tree {
            Tree::new(self.0.clone(), &load_dir(&self.0).unwrap())
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn index(tree: &Tree, path: &Path) -> usize {
        tree.rows
            .iter()
            .position(|row| row.node.path == path)
            .unwrap()
    }

    fn toggle(tree: &mut Tree, path: &Path) {
        let index = index(tree, path);
        assert_eq!(tree.toggle(index).unwrap(), index);
    }

    #[test]
    fn nested_expansion_collapse_and_cache() {
        let fixture = Fixture::new();
        let a = fixture.0.join("a");
        let b = a.join("b");
        fs::create_dir_all(&b).unwrap();
        fs::write(a.join("z-file"), "").unwrap();
        fs::write(b.join("leaf"), "").unwrap();
        let mut tree = fixture.tree();
        assert!(tree.children.is_empty());
        toggle(&mut tree, &a);
        assert_eq!(tree.rows[index(&tree, &b)].depth, 1);
        assert!(index(&tree, &b) < index(&tree, &a.join("z-file")));
        toggle(&mut tree, &b);
        assert_eq!(tree.rows[index(&tree, &b.join("leaf"))].depth, 2);
        assert_eq!(
            tree.rows
                .iter()
                .filter(|row| row.node.kind == NodeKind::Parent)
                .count(),
            1
        );
        let expanded = tree.rows.clone();
        toggle(&mut tree, &a);
        assert_eq!(tree.rows.len(), 2);
        assert_eq!(tree.children.len(), 2);
        fs::remove_file(b.join("leaf")).unwrap();
        toggle(&mut tree, &a);
        assert_eq!(tree.rows, expanded);
        // Collapsing works without reading or resolving missing directories.
        fs::remove_dir_all(&a).unwrap();
        toggle(&mut tree, &a);
        assert_eq!(tree.rows.len(), 2);
    }

    #[test]
    fn failures_preserve_all_state_and_failed_reads_are_not_cached() {
        let fixture = Fixture::new();
        let dir = fixture.0.join("dir");
        fs::create_dir(&dir).unwrap();
        let mut tree = fixture.tree();
        let before = tree.clone();
        assert!(tree.toggle(usize::MAX).is_err());
        assert!(tree.toggle(0).is_err());
        fs::remove_dir(&dir).unwrap();
        assert!(tree.toggle(index(&tree, &dir)).is_err());
        assert_eq!(tree, before);
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("new"), "").unwrap();
        toggle(&mut tree, &dir);
        assert!(tree.rows.iter().any(|row| row.node.path == dir.join("new")));
    }

    #[cfg(unix)]
    #[test]
    fn ordinary_links_work_but_ancestor_cycles_are_transactional() {
        use std::os::unix::fs::symlink;

        let fixture = Fixture::new();
        let target = fixture.0.join("target");
        let link = fixture.0.join("link");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("leaf"), "").unwrap();
        symlink(&target, &link).unwrap();
        symlink(&fixture.0, target.join("root-cycle")).unwrap();
        symlink(&target, target.join("self-cycle")).unwrap();
        let mut tree = fixture.tree();
        toggle(&mut tree, &target);
        toggle(&mut tree, &link);
        assert!(tree
            .rows
            .iter()
            .any(|row| row.node.path == link.join("leaf")));
        for cycle in [link.join("root-cycle"), link.join("self-cycle")] {
            let before = tree.clone();
            assert!(tree
                .toggle(index(&tree, &cycle))
                .unwrap_err()
                .contains("cycle"));
            assert_eq!(tree, before);
        }
    }

    #[test]
    fn row_limit_is_exact_and_failed_projection_does_not_cache() {
        let fixture = Fixture::new();
        let dir = fixture.0.join("dir");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("leaf"), "").unwrap();
        let mut nodes = load_dir(&fixture.0).unwrap();
        nodes.extend((nodes.len()..MAX_ROWS).map(|i| Node {
            name: format!("file-{i}"),
            path: fixture.0.join(format!("file-{i}")),
            kind: NodeKind::File,
        }));
        let mut tree = Tree::new(fixture.0.clone(), &nodes);
        assert_eq!(tree.rows.len(), MAX_ROWS);
        let before = tree.clone();
        assert!(tree
            .toggle(index(&tree, &dir))
            .unwrap_err()
            .contains("2000"));
        assert_eq!(tree, before);
        fs::remove_file(dir.join("leaf")).unwrap();
        toggle(&mut tree, &dir);
        assert_eq!(tree.rows.len(), MAX_ROWS);
    }

    #[test]
    fn depth_limit_is_exact_and_transactional() {
        let fixture = Fixture::new();
        let mut dirs = Vec::new();
        let mut path = fixture.0.clone();
        for _ in 0..MAX_DEPTH + 2 {
            path = path.join("d");
            fs::create_dir(&path).unwrap();
            dirs.push(path.clone());
        }
        let mut tree = fixture.tree();
        for dir in &dirs[..MAX_DEPTH] {
            toggle(&mut tree, dir);
        }
        let index = index(&tree, &dirs[MAX_DEPTH]);
        assert_eq!(tree.rows[index].depth, MAX_DEPTH);
        let before = tree.clone();
        assert!(tree.toggle(index).unwrap_err().contains("32"));
        assert_eq!(tree, before);
    }
}
