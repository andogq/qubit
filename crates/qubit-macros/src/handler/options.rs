use std::fmt::Display;

use syn::{meta::ParseNestedMeta, Ident, LitStr, Result};

/// Handlers can have different variations depending on how they interact with the client.
#[derive(Clone)]
pub enum HandlerKind {
    /// Query handlers support the standard request/response pattern, and are safe to be cached.
    Query,

    /// Mutation handlers also support the standard request/response pattern, however they should
    /// not be cached.
    Mutation,

    /// Subscriptions have an initial request, and returns a stream of responses that the client
    /// will continue to consume.
    Subscription,
}

impl Display for HandlerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                HandlerKind::Query => "Query",
                HandlerKind::Mutation => "Mutation",
                HandlerKind::Subscription => "Subscription",
            }
        )
    }
}

/// Options that may be attached to a handler.
pub struct HandlerOptions {
    /// The kind of handler.
    pub kind: HandlerKind,

    /// Overridden name for the handler.
    pub name: Option<Ident>,
}

impl HandlerOptions {
    /// Build up an instance of [`HandlerOptions`].
    pub fn builder() -> HandlerOptionsBuilder {
        HandlerOptionsBuilder::default()
    }
}

/// Builder for [`HandlerOptions`]. Allows for the kind to be empty until it's provided.
#[derive(Default)]
pub struct HandlerOptionsBuilder {
    /// Kind of the handler.
    pub kind: Option<HandlerKind>,

    /// Overridden name of the handler.
    pub name: Option<Ident>,
}

impl HandlerOptionsBuilder {
    /// Attempt to parse the handler kind from [`ParseNestedMeta`].
    pub fn parse(&mut self, meta: ParseNestedMeta) -> Result<()> {
        if meta.path.is_ident("query") {
            self.kind = Some(HandlerKind::Query);
            Ok(())
        } else if meta.path.is_ident("mutation") {
            self.kind = Some(HandlerKind::Mutation);
            Ok(())
        } else if meta.path.is_ident("subscription") {
            self.kind = Some(HandlerKind::Subscription);
            Ok(())
        } else if meta.path.is_ident("name") {
            // Extract name from the attribute
            let name = meta.value()?.parse::<LitStr>()?.value();

            // Create the ident for the handler name
            let ident = Ident::new(&name, meta.input.span());

            self.name = Some(ident);
            Ok(())
        } else {
            Err(meta.error("unsupported handler property"))
        }
    }

    /// Consume the builder to produce [`HandlerOptions`]. Will return `None` if the builder was
    /// in an invalid state.
    pub fn build(self) -> Option<HandlerOptions> {
        Some(HandlerOptions {
            kind: self.kind?,
            name: self.name,
        })
    }
}
