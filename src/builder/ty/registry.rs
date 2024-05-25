use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};

/// Types built into qubit that are accessible from the client.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum InbuiltType {
    /// Maps between the `@qubit-rs/client` `Stream` type, and [`futures::Stream`] that handlers
    /// can use.
    Stream,
}

impl Display for InbuiltType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InbuiltType::Stream => {
                write!(f, r#"import type {{ Stream }} from "@qubit-rs/client";"#)
            }
        }
    }
}

/// Registry of all types required to be exported.
#[derive(Default)]
pub struct TypeRegistry {
    /// User-generated types.
    user: BTreeMap<String, String>,

    /// Inbuilt types.
    inbuilt: BTreeSet<InbuiltType>,
}

impl TypeRegistry {
    /// Register a user-defined type. Returns `true` if this type has already been registered.
    pub fn register(&mut self, name: impl ToString, definition: impl ToString) -> bool {
        let name = name.to_string();

        // Ensure that the type doesn't already exist
        if self.user.contains_key(&name) {
            return true;
        }

        // BUG: Will overwrite existing type if names collide
        self.user.insert(name, definition.to_string());

        false
    }

    /// Register an inbulit type to ensure that it's exported.
    pub fn inbuilt(&mut self, inbuilt: InbuiltType) -> &mut Self {
        self.inbuilt.insert(inbuilt);

        self
    }
}

impl Display for TypeRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Write out all inbuilt types first
        for inbuilt in &self.inbuilt {
            write!(f, "{inbuilt}\n")?;
        }

        for (name, ty) in &self.user {
            write!(f, "export type {name} = {ty};\n")?;
        }

        Ok(())
    }
}
