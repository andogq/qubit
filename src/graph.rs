use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PrefixId(usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ItemId(usize);

struct Prefix<P> {
    data: P,
    parent: Option<PrefixId>,
    items: Vec<ItemId>,
}

struct Item<I> {
    data: I,
    prefix: PrefixId,
}

pub struct Graph<P, I> {
    prefix: Vec<Prefix<P>>,
    items: Vec<Item<I>>,
}

impl<P, I> Graph<P, I> {
    pub fn new() -> Self {
        Self {
            prefix: Vec::new(),
            items: Vec::new(),
        }
    }

    pub fn insert_prefix(&mut self, parent: Option<PrefixId>, data: P) -> PrefixId {
        let prefix_id = PrefixId(self.prefix.len());
        self.prefix.push(Prefix {
            data,
            parent,
            items: Vec::new(),
        });
        prefix_id
    }

    pub fn insert_item(&mut self, prefix: PrefixId, data: I) -> ItemId {
        let item_id = ItemId(self.items.len());
        self.items.push(Item { prefix, data });
        self.prefix[prefix.0].items.push(item_id);
        item_id
    }

    pub fn merge(&mut self, other: Self) -> HashMap<PrefixId, PrefixId> {
        // Insert all the prefixes.
        let prefix_map = other
            .prefix
            .into_iter()
            .enumerate()
            .map(|(id, prefix)| (PrefixId(id), prefix))
            .fold(HashMap::new(), |mut map, (prefix_id, prefix)| {
                // Look up the parent's new ID.
                let parent = prefix.parent.map(|parent| {
                    assert!(parent.0 < prefix_id.0, "parent must exist before child");
                    map[&parent]
                });

                // Create the new prefix, and save a mapping to it's old ID.
                let new_prefix_id = self.insert_prefix(parent, prefix.data);
                map.insert(prefix_id, new_prefix_id);

                map
            });

        // Insert all the items.
        other
            .items
            .into_iter()
            .enumerate()
            .map(|(id, item)| (ItemId(id), item))
            .fold(HashMap::new(), |mut map, (item_id, item)| {
                // Create the new item, and save a mapping to it's old ID.
                let new_item_id = self.insert_item(prefix_map[&item.prefix], item.data);
                map.insert(item_id, new_item_id);

                map
            });

        prefix_map
    }

    pub fn nest(&mut self, prefix: PrefixId, other: Self) {
        // Select all prefixes without a parent.
        let root_prefixes = other
            .prefix
            .iter()
            .enumerate()
            .map(|(id, prefix)| (PrefixId(id), prefix))
            .filter(|(_, prefix)| prefix.parent.is_none())
            .map(|(id, _)| id)
            .collect::<Vec<_>>();

        // Merge the graphs.
        let prefix_map = self.merge(other);

        // Update the old root prefixes to their new parent prefix.
        root_prefixes
            .into_iter()
            .map(|prefix| prefix_map[&prefix])
            .for_each(|child_prefix| {
                self.prefix[child_prefix.0].parent = Some(prefix);
            });
    }

    pub fn iter(&self) -> impl Iterator<Item = (Vec<&P>, &I)> {
        self.items.iter().map(|item| {
            (
                {
                    let mut prefix = vec![item.prefix];

                    while let Some(parent) = self.prefix[prefix.last().unwrap().0].parent {
                        prefix.push(parent);
                    }

                    prefix
                        .into_iter()
                        .map(|prefix| &self.prefix[prefix.0].data)
                        .rev()
                        .collect()
                },
                &item.data,
            )
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn merge() {
        let mut graph = {
            let mut graph = Graph::new();

            let prefix_a = graph.insert_prefix(None, "a");
            let prefix_b = graph.insert_prefix(None, "b");

            graph.insert_item(prefix_a, "A");
            graph.insert_item(prefix_b, "B");

            graph
        };

        graph.merge({
            let mut graph = Graph::new();

            let prefix_c = graph.insert_prefix(None, "c");
            let prefix_d = graph.insert_prefix(None, "d");

            graph.insert_item(prefix_c, "C");
            graph.insert_item(prefix_d, "D");

            graph
        });

        assert_eq!(
            graph.iter().collect::<Vec<_>>(),
            [
                (vec![&"a"], &"A"),
                (vec![&"b"], &"B"),
                (vec![&"c"], &"C"),
                (vec![&"d"], &"D")
            ]
        );
    }
}
