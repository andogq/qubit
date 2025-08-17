use std::fmt::Write;

use crate::{
    __private::HandlerKind,
    codegen::{
        Backend, BackendStage, CodegenType, HandlerBackend, HandlerCodegen, QUBIT_HEADER,
        TypeBackend,
    },
};

pub struct Ts;

impl<W: Write> Backend<W> for Ts {
    type HandlerBackend = Self;

    type TypeBackend = Self;

    const STAGES: &[BackendStage] = &[BackendStage::Type, BackendStage::Handler];

    fn begin(writer: &mut W) -> Result<(), std::fmt::Error> {
        writeln!(writer, "/* eslint-disable */")?;
        writeln!(writer, "// @ts-nocheck")?;

        writeln!(writer, "/*")?;
        writeln!(writer, "{QUBIT_HEADER}")?;
        writeln!(writer, "*/")?;

        writeln!(
            writer,
            r#"import {{ Query, Mutation, Subscription }} from "@qubit-rs/client";"#
        )?;

        Ok(())
    }
}

impl<W: Write> HandlerBackend<W> for Ts {
    fn begin(writer: &mut W) -> Result<(), std::fmt::Error> {
        write!(writer, "export type QubitServer = ")
    }

    fn write_key(key: &str, writer: &mut W) -> Result<(), std::fmt::Error> {
        write!(writer, "{key}: ")
    }

    fn write_handler(handler: &HandlerCodegen, writer: &mut W) -> Result<(), std::fmt::Error> {
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

    fn begin_nested(_root: bool, writer: &mut W) -> Result<(), std::fmt::Error> {
        write!(writer, "{{ ")
    }

    fn end_nested(root: bool, writer: &mut W) -> Result<(), std::fmt::Error> {
        write!(writer, " }}")?;

        if !root {
            write!(writer, ", ")?;
        }

        Ok(())
    }
}

impl<W: Write> TypeBackend<W> for Ts {
    fn write_type(
        name: &CodegenType,
        definition: &str,
        writer: &mut W,
    ) -> Result<(), std::fmt::Error> {
        write!(writer, "export type {name} = {definition}")
    }
}
