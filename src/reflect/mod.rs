use std::any::TypeId;

use ts_rs::TS;

/// Type information suitable for reflection.
pub struct Type {
    /// Name of the type, including any generics provided to it.
    name: String,
    /// Declaration of the type.
    declaration: TypeDeclaration,
}

pub enum TypeDeclaration {
    /// Primitive type, no declaration required.
    Primitive,
    /// Custom user type.
    User {
        // Type ID to de-duplicate repeated types.
        type_id: TypeId,
        /// Name of the type.
        name: String,
        /// Generics used within the declaration.
        generics: Vec<String>,
        /// Type definition.
        definition: String,
    },
}

impl Type {
    /// Create from a type that implements [`TS`].
    pub fn from_type<T: TS + 'static + Sized>() -> Self {
        Self {
            name: T::name(),
            declaration: match T::output_path() {
                Some(_) => {
                    // Generate the declaration, which includes `type ... =`, and any generic
                    // parameters.
                    let declaration = T::decl();

                    // Split the declaration into the name and definition.
                    let (name, definition) =
                        declaration.split_once("=").expect("valid TS declaration");

                    // Process the definition.
                    let definition = definition.strip_suffix(';').unwrap().trim().to_string();

                    // Try extract generics, if they're present.
                    let (name, generics) = match name.strip_prefix("type ").unwrap().split_once('<')
                    {
                        Some((name, generics)) => (
                            name,
                            // Extract the generics.
                            generics
                                .rsplit_once('>')
                                .unwrap()
                                .0
                                .split(',')
                                .map(|generic| generic.trim().to_string())
                                .collect(),
                        ),
                        // No generics present in the definition.
                        None => (name, Vec::new()),
                    };

                    TypeDeclaration::User {
                        type_id: TypeId::of::<T::WithoutGenerics>(),
                        name: name.to_string(),
                        generics,
                        definition,
                    }
                }
                None => TypeDeclaration::Primitive,
            },
        }
    }
}
