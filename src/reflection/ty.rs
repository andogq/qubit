//! Anything relating to runtime reflection of type information.

use std::fmt::Display;

use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodegenType {
    name: String,
    generics: Vec<String>,
}

impl CodegenType {
    pub fn from_type_with_definition<T: TS + 'static + ?Sized>() -> (Self, String) {
        // Generate the declaration, which includes `type ... =`, and any generic
        // parameters.
        let declaration = T::decl();

        // Split the declaration into the name and definition.
        let (name, definition) = declaration.split_once("=").expect("valid TS declaration");

        // Process the definition.
        let definition = definition.strip_suffix(';').unwrap().trim().to_string();

        let name = name.strip_prefix("type").unwrap().trim().to_string();

        (Self::from_name_and_generics(name), definition)
    }

    pub fn from_type<T: TS + 'static + ?Sized>() -> Self {
        Self::from_name_and_generics(T::name())
    }

    fn from_name_and_generics(s: impl AsRef<str>) -> Self {
        let (name, generics) = match s.as_ref().split_once('<') {
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
            None => (s.as_ref(), Vec::new()),
        };

        Self {
            name: name.to_string(),
            generics,
        }
    }
}

impl Display for CodegenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;

        if !self.generics.is_empty() {
            write!(f, "<{}>", self.generics.join(", "))?;
        }

        Ok(())
    }
}
