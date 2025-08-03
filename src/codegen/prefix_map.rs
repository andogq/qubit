use std::collections::{
    BTreeMap, BTreeSet,
    btree_map::{self, Entry},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct KeyId(usize);

struct KeyInfo<V> {
    parent: Option<KeyId>,
    name: String,
    content: KeyContent<V>,
}

enum KeyContent<V> {
    Value(V),
    Nested(BTreeSet<KeyId>),
}

pub struct PrefixMap<V> {
    next_key_id: KeyId,

    top_keys: BTreeMap<String, KeyId>,
    keys: BTreeMap<KeyId, KeyInfo<V>>,
}

impl<V> PrefixMap<V> {
    pub fn new() -> Self {
        Self {
            next_key_id: KeyId(0),
            top_keys: BTreeMap::new(),
            keys: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, key_name: impl ToString, value: V) {
        let key_name = key_name.to_string();
        let key = self.insert_key(key_name.clone());

        self.keys.insert(
            key,
            KeyInfo {
                parent: None,
                name: key_name,
                content: KeyContent::Value(value),
            },
        );
    }

    fn insert_key(&mut self, key_name: impl ToString) -> KeyId {
        match self.top_keys.entry(key_name.to_string()) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let key = self.next_key_id;
                self.next_key_id = KeyId(key.0 + 1);

                entry.insert(key);

                key
            }
        }
    }

    pub fn nest(&mut self, key_name: impl ToString, other: Self) {
        let key_name = key_name.to_string();
        let key = self.insert_key(key_name.clone());

        // Generate new IDs for all of the keys.
        let key_map = BTreeMap::from_iter(other.keys.keys().map(|key| {
            (*key, {
                let new_key = self.next_key_id;
                self.next_key_id = KeyId(new_key.0 + 1);
                new_key
            })
        }));

        // Create the key
        self.keys.insert(
            key,
            KeyInfo {
                parent: None,
                name: key_name,
                content: KeyContent::Nested(
                    other
                        .top_keys
                        .values()
                        .map(|key| *key_map.get(key).expect("child key mappings created"))
                        .collect(),
                ),
            },
        );

        // Copy all of the keys, mapping them to new IDs.
        for (child_key, child_key_info) in other.keys {
            let new_key = *key_map.get(&child_key).expect("child key mappings created");

            self.keys.insert(
                new_key,
                KeyInfo {
                    // Update the child's parent to the new ID.
                    parent: child_key_info
                        .parent
                        .map(|parent| *key_map.get(&parent).expect("child key mappings created")),
                    name: child_key_info.name,
                    content: match child_key_info.content {
                        KeyContent::Value(value) => KeyContent::Value(value),
                        KeyContent::Nested(children) => KeyContent::Nested(
                            // Map the children IDs.
                            children
                                .into_iter()
                                .map(|child| {
                                    *key_map.get(&child).expect("child key mappings created")
                                })
                                .collect(),
                        ),
                    },
                },
            );
        }

        // Nest all of the top-level keys.
        for (_, child_key) in other.top_keys {
            let child_key_info = self
                .keys
                .get_mut(&child_key)
                .expect("child keys transfered");
            child_key_info.parent = Some(key);
        }
    }

    pub fn iter(&self) -> PrefixMapWalker<'_, V> {
        let mut top_keys = self.top_keys.iter();
        let key = top_keys.next().map(|(_, key)| *key);

        PrefixMapWalker {
            prefix_map: self,
            key,
            top_keys,
        }
    }
}

pub enum MapStep<V> {
    Value(V),
    BeginNested,
    EndNested,
}

struct PrefixMapWalker<'p, V> {
    prefix_map: &'p PrefixMap<V>,
    top_keys: btree_map::Iter<'p, String, KeyId>,
    key: Option<KeyId>,
}

impl<'p, V> PrefixMapWalker<'p, V> {
    fn advance_key(&mut self) {
        let Some(key) = self.key else {
            return;
        };

        let key_info = self.prefix_map.keys.get(&key).expect("key info");
        if let Some(parent_id) = key_info.parent {
            // Find the next sibling key.
            let parent = self
                .prefix_map
                .keys
                .get(&parent_id)
                .expect("parent key info");

            let KeyContent::Nested(children) = &parent.content else {
                panic!("parent key doesn't have nested children content");
            };

            let next_sibling = children
                .iter()
                .cloned()
                .skip_while(|&sibling| sibling != key)
                .next();

            if let Some(next_sibling) = next_sibling {
                // Continue and visit the next sibling.
                self.key = Some(next_sibling);
            } else {
                // Exhausted all siblings, so the parent is finished. Recurse to find the next key.
                self.key = Some(parent_id);
                self.advance_key();
            }
        } else {
            // Get the next top level key.
            self.key = self.top_keys.next().map(|(_, key)| *key);
        }
    }
}

impl<'p, V> Iterator for PrefixMapWalker<'p, V> {
    type Item = MapStep<&'p V>;

    fn next(&mut self) -> Option<Self::Item> {
        let key_id = self.key?;

        let key = self.prefix_map.keys.get(&key_id).expect("valid key id");

        // match key.content {
        //     KeyContent::Value(value) => {
        //         Some(MapStep::Value(value));
        //         self.advance_key();
        //     }
        //     KeyContent::Nested(children) => {
        //         self.key = children.iter().next();
        //         Some(MapStep::BeginNested)
        //     }
        // }

        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;
}
