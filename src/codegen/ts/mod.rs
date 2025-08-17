use std::fmt::Write;

use ts_rs::TS;

use crate::__private::HandlerKind;

use super::Backend;

/// Lines to prepend to the beginning of the generated file.
const HEADER_LINES: &[&str] = &["/* eslint-disable */", "// @ts-nocheck"];
const QUBIT_PACKAGE: &str = "@qubit-rs/client";

struct TsBackend {
    handlers: Vec<TsHandler>,
    types: Vec<TsType>,
}

impl TsBackend {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
            types: Vec::new(),
        }
    }

    /// Write all user types registered to this backend into the provided writer. Each type will be
    /// exported.
    fn write_types(&self, mut writer: impl Write) -> Result<(), std::fmt::Error> {
        self.types
            .iter()
            .try_for_each(|TsType { name, definition }| {
                writeln!(writer, "export type {name} = {definition};")
            })
    }

    fn write_router_type(&self, mut writer: impl Write) -> Result<(), std::fmt::Error> {
        Ok(())
    }

    /// Write all required Qubit imports.
    fn write_qubit_imports(&self, mut writer: impl Write) -> Result<(), std::fmt::Error> {
        let mut query = false;
        let mut mutation = false;
        let mut subscription = false;

        for handler in &self.handlers {
            match handler.kind {
                HandlerKind::Query => query = true,
                HandlerKind::Mutation => mutation = true,
                HandlerKind::Subscription => subscription = true,
            }

            if query && mutation && subscription {
                // No need to continue searching if all of them are required.
                break;
            }
        }

        write!(writer, "import {{ ")?;
        [
            query.then_some("Query"),
            mutation.then_some("Mutation"),
            subscription.then_some("Subscription"),
        ]
        .into_iter()
        .flatten()
        .try_for_each(|name| write!(writer, "{name}, "))?;
        writeln!(writer, r#"}} from "{QUBIT_PACKAGE}";"#)?;

        Ok(())
    }
}

// impl Backend for TsBackend {
//     type HandlerBuilder = TsHandlerBuilder;

//     const FILE_EXTENSION: &'static str = "ts";

//     fn register_user_type<T: TS + 'static + ?Sized>(&mut self) {
//         self.types.push(TsType::from::<T>());
//     }

//     fn register_handler(&mut self, handler: <Self::HandlerBuilder as HandlerBuilder>::Output) {
//         self.handlers.push(handler);
//     }

//     fn codegen(
//         &self,
//         header: &'static str,
//         mut writer: impl std::fmt::Write,
//     ) -> Result<(), std::fmt::Error> {
//         // Add comments to disable common linting tools.
//         HEADER_LINES
//             .iter()
//             .try_for_each(|line| writeln!(writer, "{line}"))?;

//         // Write the header in a multi-line comment block.
//         write_comment_block(&mut writer, header)?;

//         // Write generated user types
//         writeln!(writer)?;
//         self.write_types(&mut writer)?;

//         // Write router type.
//         writeln!(writer)?;
//         self.write_router_type(&mut writer)?;

//         Ok(())
//     }
// }

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
struct TsType {
    name: String,
    definition: String,
}

impl TsType {
    fn from<T: TS + 'static + ?Sized>() -> Self {
        // Parse type information from the declaration, since it's the only way to retain generics.
        let declaration = T::decl();

        let (name, definition) = declaration.split_once("=").expect("valid TS declaration");

        let name = name.strip_prefix("type").unwrap().trim();
        let definition = definition.strip_suffix(';').unwrap().trim();

        Self {
            name: name.to_string(),
            definition: definition.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
struct TsHandler {
    kind: HandlerKind,
    name: &'static str,
    params: Vec<(&'static str, String)>,
    return_ty: String,
}

struct TsHandlerBuilder {
    kind: HandlerKind,
    name: &'static str,
    params: Vec<(&'static str, String)>,
}

// impl HandlerBuilder for TsHandlerBuilder {
//     type Output = TsHandler;
//
//     fn new(name: &'static str, kind: HandlerKind) -> Self {
//         Self {
//             kind,
//             name,
//             params: Vec::new(),
//         }
//     }
//
//     fn push_param<T: TS + 'static + ?Sized>(&mut self, param_name: &'static str) {
//         self.params.push((param_name, T::name()));
//     }
//
//     fn returning<T: TS + 'static + ?Sized>(self) -> Self::Output {
//         TsHandler {
//             kind: self.kind,
//             name: self.name,
//             params: self.params,
//             return_ty: T::name(),
//         }
//     }
// }

/// Write `content` to the provided writer, wrapping it in a formatted comment block.
fn write_comment_block(mut writer: impl Write, content: &str) -> Result<(), std::fmt::Error> {
    writeln!(writer, "/*")?;
    content
        .lines()
        .try_for_each(|line| write!(writer, " * {line}"))?;
    writeln!(writer, " */")?;

    Ok(())
}

// #[cfg(test)]
// mod test {
//     use super::*;

//     use crate::{__private::HandlerMeta, codegen::Codegen};

//     mod with_codegen {
//         use serde::{Deserialize, Serialize};

//         use super::*;

//         #[test]
//         fn only_inbuilt_types() {
//             let mut codegen = Codegen::new(TsBackend::new());

//             codegen.register_handler(
//                 &HandlerMeta {
//                     kind: HandlerKind::Query,
//                     name: "handler_a",
//                     param_names: &["param_a", "param_b", "param_c"],
//                 },
//                 #[allow(unused)]
//                 &|ctx: (), param_a: u32, param_b: String, param_c: bool| -> Vec<i32> { todo!() },
//             );

//             assert!(codegen.backend.types.is_empty());

//             assert_eq!(
//                 codegen.backend.handlers,
//                 vec![TsHandler {
//                     kind: HandlerKind::Query,
//                     name: "handler_a",
//                     params: vec![
//                         ("param_a", "number".to_string()),
//                         ("param_b", "string".to_string()),
//                         ("param_c", "boolean".to_string())
//                     ],
//                     return_ty: "Array<number>".to_string()
//                 }]
//             )
//         }

//         #[test]
//         fn with_user_types() {
//             #[derive(TS, Deserialize)]
//             struct TypeA;
//             #[derive(TS, Clone, Deserialize, Serialize)]
//             struct TypeB<T>(T);
//             #[derive(TS, Deserialize)]
//             struct TypeC;

//             let mut codegen = Codegen::new(TsBackend::new());

//             codegen.register_handler(
//                 &HandlerMeta {
//                     kind: HandlerKind::Query,
//                     name: "handler_a",
//                     param_names: &["param_a", "param_b", "param_c"],
//                 },
//                 #[allow(unused)]
//                 &|ctx: (),
//                   param_a: TypeA,
//                   param_b: TypeB<u32>,
//                   param_c: Vec<TypeC>|
//                  -> TypeB<bool> { todo!() },
//             );

//             assert_eq!(
//                 codegen.backend.types,
//                 vec![
//                     TsType {
//                         name: "TypeA".to_string(),
//                         definition: "null".to_string()
//                     },
//                     TsType {
//                         name: "TypeB<T>".to_string(),
//                         definition: "T".to_string()
//                     },
//                     TsType {
//                         name: "TypeC".to_string(),
//                         definition: "null".to_string()
//                     },
//                 ]
//             );

//             assert_eq!(
//                 codegen.backend.handlers,
//                 vec![TsHandler {
//                     kind: HandlerKind::Query,
//                     name: "handler_a",
//                     params: vec![
//                         ("param_a", "TypeA".to_string()),
//                         ("param_b", "TypeB<number>".to_string()),
//                         ("param_c", "Array<TypeC>".to_string())
//                     ],
//                     return_ty: "TypeB<boolean>".to_string()
//                 }]
//             );
//         }
//     }
// }
