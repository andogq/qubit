use std::{
    fs::{File, OpenOptions},
    path::Path,
};

use ts_rs::TypeVisitor;

use crate::{
    __private::HandlerMeta,
    codegen::{Backend, Codegen, DependentTypes, HandlerCodegen},
    handler::{marker, response::ResponseValue, ts::TsTypeTuple},
    router::{RouterModule, RouterModuleHandler},
};

pub struct CodegenModule(Codegen);

impl CodegenModule {
    pub fn new() -> Self {
        Self(Codegen::new())
    }

    pub fn generate_type(&self, backend: impl Backend<Vec<u8>>) -> std::io::Result<String> {
        let mut generated_type = Vec::new();
        self.0.generate(&mut generated_type, backend)?;
        Ok(String::from_utf8(generated_type).unwrap())
    }

    /// Generate the TypeScript for this router, and write it to the provided path.
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
        (handler.visit_dependent_types)(&mut self.0.dependent_types);
        self.0.tree.insert(path, &handler.handler);
    }
}

pub struct HandlerRegister {
    handler: HandlerCodegen,
    visit_dependent_types: Box<dyn Fn(&mut DependentTypes)>,
}

impl<Ctx> RouterModuleHandler<Ctx> for HandlerRegister {
    fn from_handler<F, MSig, MValue: marker::ResponseMarker, MReturn: marker::HandlerReturnMarker>(
        handler: F,
        meta: &'static HandlerMeta,
    ) -> Self
    where
        F: crate::RegisterableHandler<Ctx, MSig, MValue, MReturn>,
        F::Ctx: crate::FromRequestExtensions<Ctx>,
    {
        Self {
            handler: HandlerCodegen::from_handler(meta, &handler),
            visit_dependent_types: Box::new(move |dependent_types: &mut DependentTypes| {
                <F::Params as TsTypeTuple>::visit_tys(dependent_types);
                dependent_types.visit::<<F::Response as ResponseValue<MValue>>::Value>();
            }),
        }
    }
}
