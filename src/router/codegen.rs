use std::{
    fs::{File, OpenOptions},
    path::Path,
};

use ts_rs::TypeVisitor;

use crate::{
    FromRequestExtensions, RegisterableHandler,
    codegen::{Backend, Codegen, DependentTypes, HandlerCodegen},
    handler::{marker, response::ResponseValue, ts::TsTypeTuple},
    reflection::handler::HandlerMeta,
    router::{RouterModule, RouterModuleHandler},
};

/// Wrapper around [`Codegen`] for use with a [`Router`].
///
/// [`Router`]: crate::Router
pub struct CodegenModule(Codegen);

impl CodegenModule {
    /// Create a new instance.
    pub(crate) fn new() -> Self {
        Self(Codegen::new())
    }

    /// Generate a type with the provided backend into a string.
    pub fn generate_type(&self, backend: impl Backend<Vec<u8>>) -> std::io::Result<String> {
        let mut generated_type = Vec::new();
        self.0.generate(&mut generated_type, backend)?;
        Ok(String::from_utf8(generated_type).unwrap())
    }

    /// Generate the TypeScript for this router, and write it to the provided path.
    ///
    /// If a file at the path doesn't exist it will be created. If it does exist, it will be
    /// overwritten. If the directory doesn't exist, an error will be returned.
    pub fn write_type(
        &self,
        output_path: impl AsRef<Path>,
        backend: impl Backend<File>,
    ) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(output_path)?;
        self.0.generate(&mut file, backend)
    }
}

impl Default for CodegenModule {
    fn default() -> Self {
        Self::new()
    }
}

impl<Ctx> RouterModule<Ctx> for CodegenModule {
    type Handler = HandlerRegister;

    fn visit_handler(&mut self, path: &[&str], handler: &Self::Handler) {
        // Save all dependent types.
        (handler.visit_dependent_types)(&mut self.0.dependent_types);
        // Insert the handler into the graph.
        self.0.tree.insert(path, handler.handler.clone());
    }
}

/// All information required to generate the handler type at runtime.
pub struct HandlerRegister {
    /// Reflected information about the handler.
    handler: HandlerCodegen,
    /// Callback to register dependent types for this handler into the provided [`DependentTypes`]
    /// instance.
    visit_dependent_types: Box<dyn Fn(&mut DependentTypes)>,
}

impl<Ctx> RouterModuleHandler<Ctx> for HandlerRegister {
    fn from_handler<F, MSig, MValue: marker::ResponseMarker, MReturn: marker::HandlerReturnMarker>(
        handler: F,
        meta: &'static HandlerMeta,
    ) -> Self
    where
        F: RegisterableHandler<Ctx, MSig, MValue, MReturn>,
        F::Ctx: FromRequestExtensions<Ctx>,
    {
        Self {
            handler: HandlerCodegen::from_handler(meta, &handler),
            visit_dependent_types: Box::new(move |dependent_types: &mut DependentTypes| {
                // Add all parameter types into the dependent types.
                <F::Params as TsTypeTuple>::visit_tys(dependent_types);
                // Add the return type into the dependent types.
                dependent_types.visit::<<F::Response as ResponseValue<MValue>>::Value>();
            }),
        }
    }
}
