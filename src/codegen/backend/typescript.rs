use std::io::Write;

use crate::{
    codegen::{Backend, BackendStage, HandlerBackend, HandlerCodegen, QUBIT_HEADER, TypeBackend},
    reflection::{handler::HandlerKind, ty::CodegenType},
};

pub struct TypeScript {
    include_preamble: bool,
    router_name: String,
}

impl TypeScript {
    pub fn new() -> Self {
        Self {
            include_preamble: true,
            router_name: "QubitServer".to_string(),
        }
    }

    pub fn with_router_name(mut self, router_name: impl ToString) -> Self {
        self.router_name = router_name.to_string();
        self
    }

    pub fn without_preamble(mut self) -> Self {
        self.include_preamble = false;
        self
    }
}

impl Default for TypeScript {
    fn default() -> Self {
        Self::new()
    }
}

impl<W: Write> Backend<W> for TypeScript {
    type HandlerBackend = Self;
    type TypeBackend = Self;

    const STAGES: &[BackendStage] = &[BackendStage::Type, BackendStage::Handler];

    fn get_handler_backend(&self) -> &Self::HandlerBackend {
        self
    }

    fn get_type_backend(&self) -> &Self::TypeBackend {
        self
    }

    fn begin(&self, writer: &mut W) -> Result<(), std::io::Error> {
        if self.include_preamble {
            writeln!(writer, "/* eslint-disable */")?;
            writeln!(writer, "// @ts-nocheck")?;

            writeln!(writer, "/*")?;
            writeln!(writer, "{QUBIT_HEADER}")?;
            writeln!(writer, "*/")?;

            writeln!(
                writer,
                r#"import {{ Query, Mutation, Subscription }} from "@qubit-rs/client";"#
            )?;
        }

        Ok(())
    }
}

impl<W: Write> HandlerBackend<W> for TypeScript {
    fn begin(&self, writer: &mut W) -> std::io::Result<()> {
        write!(writer, "export type {} = ", self.router_name)
    }

    fn end(&self, writer: &mut W) -> std::io::Result<()> {
        writeln!(writer, ";")
    }

    fn write_key(&self, key: &str, writer: &mut W) -> std::io::Result<()> {
        write!(writer, "{key}: ")
    }

    fn write_handler(&self, handler: &HandlerCodegen, writer: &mut W) -> std::io::Result<()> {
        let kind = match handler.kind {
            HandlerKind::Query => "Query",
            HandlerKind::Mutation => "Mutation",
            HandlerKind::Subscription => "Subscription",
        };

        let params = handler
            .params
            .iter()
            .map(|(name, ty)| format!("{name}: {ty}"))
            .collect::<Vec<_>>()
            .join(", ");

        let return_ty = &handler.return_ty;

        write!(writer, "{kind}<[{params}], {return_ty}>, ")
    }

    fn begin_nested(&self, _root: bool, writer: &mut W) -> std::io::Result<()> {
        write!(writer, "{{ ")
    }

    fn end_nested(&self, root: bool, writer: &mut W) -> std::io::Result<()> {
        write!(writer, "}}")?;

        if !root {
            write!(writer, ", ")?;
        }

        Ok(())
    }
}

impl<W: Write> TypeBackend<W> for TypeScript {
    fn write_type(
        &self,
        name: &CodegenType,
        definition: &str,
        writer: &mut W,
    ) -> std::io::Result<()> {
        write!(writer, "export type {name} = {definition}")
    }
}
