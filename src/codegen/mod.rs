//! Code generation from [`crate::reflection`] primitives.

mod backend;
mod reflection;

use std::io::Write;

pub use self::backend::*;
pub(crate) use self::reflection::*;

use crate::{
    RegisterableHandler,
    handler::{marker, response::ResponseValue},
    reflection::{
        handler::{HandlerKind, HandlerMeta},
        ty::CodegenType,
    },
    util::Node,
};

/// Header string to include at the top of every generated file.
const QUBIT_HEADER: &str = include_str!("header.txt");

/// Code generation for Qubit. Collects all required information to pass to a [`Backend`]
/// implementation.
pub struct Codegen {
    /// All custom types that must be declared in the resulting type.
    pub(crate) dependent_types: DependentTypes,
    /// Tree of handlers registered to this codegen instance.
    pub(crate) tree: Node<HandlerCodegen>,
}

impl Codegen {
    /// Create a new instance.
    pub(crate) fn new() -> Self {
        Self {
            dependent_types: DependentTypes::new(),
            tree: Node::new(),
        }
    }

    /// With the provided [`Backend`] generate the type, and write it to [`Write`].
    pub fn generate<W: Write, B: Backend<W>>(
        &self,
        writer: &mut W,
        backend: B,
    ) -> std::io::Result<()> {
        backend.begin(writer)?;

        for stage in B::STAGES {
            match stage {
                BackendStage::Handler => {
                    let handler_backend = backend.get_handler_backend();
                    handler_backend.begin(writer)?;

                    fn write_node<W: Write, B: Backend<W>>(
                        node: &Node<HandlerCodegen>,
                        root: bool,
                        writer: &mut W,
                        handler_backend: &<B as Backend<W>>::HandlerBackend,
                    ) -> std::io::Result<()> {
                        handler_backend.begin_nested(root, writer)?;

                        // Write out all the handlers.
                        for (key, handler) in &node.items {
                            handler_backend.write_key(key, writer)?;
                            handler_backend.write_handler(handler, writer)?;
                        }

                        // Recurse and write nested nodes.
                        for (key, node) in &node.children {
                            handler_backend.write_key(key, writer)?;
                            write_node::<W, B>(node, false, writer, handler_backend)?;
                        }

                        handler_backend.end_nested(root, writer)?;

                        Ok(())
                    }

                    // Walk tree with recursion.
                    write_node::<W, B>(&self.tree, true, writer, handler_backend)?;

                    handler_backend.end(writer)?;
                }
                BackendStage::Type => {
                    let type_backend = backend.get_type_backend();
                    type_backend.begin(writer)?;

                    for (name, definition) in self.dependent_types.definitions.values() {
                        type_backend.write_type(name, definition, writer)?;
                    }

                    type_backend.end(writer)?;
                }
            }
        }

        backend.end(writer)?;

        Ok(())
    }
}

impl Default for Codegen {
    fn default() -> Self {
        Self::new()
    }
}

/// Representation of a handler function, for the purpose of code generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandlerCodegen {
    /// The kind of handler.
    kind: HandlerKind,
    /// Parameter name and types that the handler accepts.
    params: Vec<(&'static str, CodegenType)>,
    /// Return type of the handler.
    return_ty: CodegenType,
}

impl HandlerCodegen {
    /// Derive code generation information from a handler.
    pub fn from_handler<F, Ctx, MSig, MValue, MReturn>(meta: &HandlerMeta, _handler: &F) -> Self
    where
        F: RegisterableHandler<Ctx, MSig, MValue, MReturn>,
        MValue: marker::ResponseMarker,
        MReturn: marker::HandlerReturnMarker,
    {
        HandlerCodegen {
            kind: meta.kind,
            params: ParamVisitor::visit::<F::Params>(meta.param_names).unwrap(),
            return_ty: CodegenType::from_type::<<F::Response as ResponseValue<MValue>>::Value>(),
        }
    }
}
