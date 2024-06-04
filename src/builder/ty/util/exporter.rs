use std::path::{Path, PathBuf};

use ts_rs::{
    typelist::{TypeList, TypeVisitor},
    Dependency, ExportError, TS,
};

/// Visitor for [`ts_rs::TypeList`], which will export each type and any dependent types. It will
/// also save a list of top level [`Dependency`]s that must be imported in order for the original
/// [`ts_rs::TypeList`] to be used.
struct TypeListExporter {
    out_dir: PathBuf,
    dependencies: Vec<Dependency>,
}

impl TypeListExporter {
    /// Create a new empty instance.
    pub fn new(out_dir: impl AsRef<Path>) -> Self {
        Self {
            out_dir: out_dir.as_ref().to_owned(),
            dependencies: Vec::new(),
        }
    }

    /// Consume the exporter to produce the dependency list.
    pub fn into_inner(self) -> Vec<Dependency> {
        self.dependencies
    }

    /// Export the provided type list, and return the dependency list.
    pub fn export(type_list: impl TypeList, out_dir: impl AsRef<Path>) -> Vec<Dependency> {
        let mut visitor = Self::new(out_dir);

        type_list.for_each(&mut visitor);

        visitor.into_inner()
    }
}

impl TypeVisitor for TypeListExporter {
    fn visit<T: TS + 'static + ?Sized>(&mut self) {
        let Some(dep) = Dependency::from_ty::<T>() else {
            // Type must be a primitive, so recurse to ensure all generics are properly exported.
            T::dependency_types().for_each(self);
            T::generics().for_each(self);

            return;
        };

        // Don't duplicate dependencies
        if self.dependencies.contains(&dep) {
            return;
        }

        // Save the top level dependency
        self.dependencies.push(dep);

        // Export all required types to files
        T::export_all_to(&self.out_dir)
            .expect("type is not a primitive, so can initiate an export");
    }
}

/// Export the type definitions to the privided directory. Will return a list of top level
/// dependencies that must be imported in order to use this type.
pub fn export_with_dependencies<T: 'static + TS>(
    out_dir: impl AsRef<Path>,
) -> Result<Vec<Dependency>, ExportError> {
    // Ensure any generics used in the type are exported
    let mut dependencies =
        TypeListExporter::export(T::dependency_types().extend(T::generics()), &out_dir);

    // Make sure the top-level type isn't a primitive, so it can be exported.
    if T::output_path().is_some() {
        // Can directly export the type (and dependencies)!
        T::export_all_to(&out_dir)?;

        // Only the top level type is required to be imported
        dependencies.push(Dependency::from_ty::<T>().expect("type is non-primitive"));
    }

    Ok(dependencies)
}
