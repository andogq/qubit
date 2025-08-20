use std::collections::BTreeMap;

/// Utility structure for a linked-list like structure, where edges are strings.
pub struct Node<V> {
    /// All items present on this node.
    pub items: BTreeMap<String, V>,
    /// All child nodes.
    pub children: BTreeMap<String, Node<V>>,
}

impl<V> Node<V> {
    /// Create a new node with no items or children.
    pub fn new() -> Self {
        Self {
            items: BTreeMap::new(),
            children: BTreeMap::new(),
        }
    }

    /// Insert a new value at the provided path. Intermediary nodes will automatically be created.
    pub fn insert(&mut self, path: &[&str], value: V) {
        assert!(!path.is_empty());

        if path.len() == 1 {
            self.items.insert(path[0].to_string(), value);
            return;
        }

        self.children
            .entry(path[0].to_string())
            .or_default()
            .insert(&path[1..], value);
    }
}

impl<V> Default for Node<V> {
    fn default() -> Self {
        Self::new()
    }
}
