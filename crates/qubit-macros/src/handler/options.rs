use syn::{meta::ParseNestedMeta, Ident, LitStr, Result};

/// Handlers can have different variations depending on how they interact with the client.
pub enum HandlerKind {
    /// Query handlers support the standard request/response pattern.
    Query,

    /// Subscriptions have an initial request, and returns a stream of responses that the client
    /// will continue to consume.
    Subscription,
}

/// Options that may be attached to a handler.
#[derive(Default)]
pub struct HandlerOptions {
    /// The kind of handler.
    pub kind: Option<HandlerKind>,

    /// Overridden name for the handler.
    pub name: Option<Ident>,
}

impl HandlerOptions {
    /// Attempt to parse the handler kind from [`ParseNestedMeta`].
    pub fn parse(&mut self, meta: ParseNestedMeta) -> Result<()> {
        if meta.path.is_ident("query") {
            self.kind = Some(HandlerKind::Query);
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
}
