mod typescript;

use std::io::Write;

use crate::{HandlerCodegen, reflection::ty::CodegenType};

pub use self::typescript::TypeScript;

/// Code generation backend implementation.
pub trait Backend<W: Write> {
    /// Code generation implementation for handlers.
    type HandlerBackend: HandlerBackend<W>;
    /// Code generation implementation for types.
    type TypeBackend: TypeBackend<W>;

    /// Code generation stages. Depending on the ordering of the stages, the handler or type
    /// implementations may be called first. This is useful if types must be defined before
    /// handlers.
    const STAGES: &[BackendStage];

    /// Produce the handler backend.
    fn get_handler_backend(&self) -> &Self::HandlerBackend;
    /// Produce the type backend.
    fn get_type_backend(&self) -> &Self::TypeBackend;

    /// Optional hook which will be called at the beginning of code generation.
    #[allow(unused)]
    fn begin(&self, writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }

    /// Optional hook which will be called at the end of code generation.
    #[allow(unused)]
    fn end(&self, writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }
}

/// Backend implementation for handlers.
pub trait HandlerBackend<W: Write> {
    /// Optional hook which will be called at the start of this stage.
    #[allow(unused)]
    fn begin(&self, writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }

    /// Optional hook which will be called at the end of this stage.
    #[allow(unused)]
    fn end(&self, writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }

    /// Write the provided key.
    fn write_key(&self, key: &str, writer: &mut W) -> std::io::Result<()>;
    /// Write the provided handler.
    fn write_handler(&self, handler: &HandlerCodegen, writer: &mut W) -> std::io::Result<()>;
    /// Begin a nested handler.
    ///
    /// If `root` is `true`, then this is the first nesting.
    fn begin_nested(&self, root: bool, writer: &mut W) -> std::io::Result<()>;
    /// End a nested handler.
    ///
    /// If `root` is `true`, then this is the first nesting.
    fn end_nested(&self, root: bool, writer: &mut W) -> std::io::Result<()>;
}

/// Backend implementation for handlers.
pub trait TypeBackend<W: Write> {
    /// Optional hook which will be called at the start of this stage.
    #[allow(unused)]
    fn begin(&self, writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }

    /// Optional hook which will be called at the end of this stage.
    #[allow(unused)]
    fn end(&self, writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }

    /// Write the provided type.
    fn write_type(
        &self,
        name: &CodegenType,
        definition: &str,
        writer: &mut W,
    ) -> std::io::Result<()>;
}

/// Available codegen backend stages.
pub enum BackendStage {
    /// Corresponds to [`HandlerBackend`].
    Handler,
    /// Corresponds to [`TypeBackend`].
    Type,
}
