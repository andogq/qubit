use std::path::Path;

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

    pub fn generate_type<B: Backend<String>>(&self, backend: B) -> Result<String, std::fmt::Error> {
        let mut generated_type = String::new();
        self.0.generate(&mut generated_type, backend).unwrap();
        Ok(generated_type)
    }

    /// Generate the TypeScript for this router, and write it to the provided path.
    pub fn write_type<B: Backend<String>>(
        &self,
        output_path: impl AsRef<Path>,
        backend: B,
    ) -> Result<(), std::fmt::Error> {
        let generated_type = self.generate_type(backend)?;
        std::fs::write(output_path, generated_type).unwrap();
        Ok(())
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
