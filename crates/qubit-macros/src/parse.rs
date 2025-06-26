use proc_macro2::{Span, TokenStream};
use syn::{Error, Ident, ItemFn, LitStr, meta::ParseNestedMeta, spanned::Spanned};

/// Parse the provided token streams into an AST.
pub fn parse(tokens_attrs: TokenStream, tokens_item: TokenStream) -> Result<Ast, Error> {
    // Parse the attributes.
    let attrs = Attributes::parse(tokens_attrs)?;

    // Parse the handler.
    let handler = syn::parse2(tokens_item)?;

    Ok(Ast::new(attrs, handler))
}

/// Simple representation of a handler, suitable for further processing by a macro.
#[derive(Clone, Debug)]
pub struct Ast {
    /// Provided attributes.
    pub attrs: Attributes,

    /// Handler implementation.
    pub handler: ItemFn,
}

impl Ast {
    /// Create a new AST instance.
    pub fn new(attrs: Attributes, handler: ItemFn) -> Self {
        Self { attrs, handler }
    }
}

/// Attributes possible to be provided to the macro.
#[derive(Clone, Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct Attributes {
    /// Overriden name for the handler.
    pub name: Option<Ident>,

    /// Kind of the handler.
    pub kind: HandlerKind,
}

impl Attributes {
    /// Create a new builder instance.
    fn builder() -> AttributesBuilder {
        AttributesBuilder::default()
    }

    pub fn parse(tokens: TokenStream) -> Result<Self, Error> {
        let mut attrs = Self::builder();

        let attrs_span = tokens.span();

        let attrs_parser = syn::meta::parser(|meta| Ok(attrs.parse(meta)?));
        syn::parse::Parser::parse2(attrs_parser, tokens)?;

        attrs
            .build()
            .map_err(|e| Error::new(attrs_span, e.to_string()))
    }
}

#[cfg(test)]
impl Attributes {
    pub(crate) fn query() -> Self {
        Self {
            kind: HandlerKind::Query,
            name: None,
        }
    }

    pub(crate) fn mutation() -> Self {
        Self {
            kind: HandlerKind::Mutation,
            name: None,
        }
    }

    pub(crate) fn subscription() -> Self {
        Self {
            kind: HandlerKind::Subscription,
            name: None,
        }
    }

    pub(crate) fn with_name(mut self, name: impl AsRef<str>) -> Self {
        self.name = Some(Ident::new(name.as_ref(), proc_macro2::Span::call_site()));
        self
    }
}

#[derive(Clone, Debug, Default)]
struct AttributesBuilder {
    name: Option<Ident>,
    kind: Option<HandlerKind>,
}

impl AttributesBuilder {
    fn build(self) -> Result<Attributes, AttributesBuilderError> {
        Ok(Attributes {
            name: self.name,
            kind: self.kind.ok_or(AttributesBuilderError::KindRequired)?,
        })
    }

    fn parse(&mut self, meta: ParseNestedMeta) -> Result<(), AttributesParseError> {
        if let Some(ident) = meta.path.get_ident() {
            // Try match the ident against a handler kind.
            if let Ok(kind) = HandlerKind::try_from(ident.to_string().as_str()) {
                // Prevent redefining handler kind if it's already been passed.
                if self.kind.is_some() {
                    return Err(AttributesParseError::KindProvided(ident.span()));
                }

                self.kind = Some(kind);
                return Ok(());
            }
        }

        if meta.path.is_ident("name") {
            let path_span = meta.path.span();

            // Fetch whatever is after the `=` (throwing an error if there isn't one).
            let value = meta.value()?;

            // Parse as a string (surrounded in quotes).
            let lit = value.parse::<LitStr>()?;

            // Parse the contents of the string as an ident.
            let name = lit.parse()?;

            // Prevent redefining handler name if it's already been passed.
            if self.name.is_some() {
                return Err(AttributesParseError::NameProvided(path_span));
            }

            self.name = Some(name);
            return Ok(());
        }

        Err(AttributesParseError::UnsupportedProperty(meta.input.span()))
    }
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum AttributesBuilderError {
    #[error("`kind` attribute is required")]
    KindRequired,
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum AttributesParseError {
    #[error("handler kind has already been provided")]
    KindProvided(Span),
    #[error("handler name has already been provided")]
    NameProvided(Span),
    #[error("unsupported handler property")]
    UnsupportedProperty(Span),
    #[error(transparent)]
    ParseError(#[from] Error),
}

impl From<AttributesParseError> for Error {
    fn from(err: AttributesParseError) -> Self {
        Error::new(
            match err {
                AttributesParseError::KindProvided(span) => span,
                AttributesParseError::NameProvided(span) => span,
                AttributesParseError::UnsupportedProperty(span) => span,
                AttributesParseError::ParseError(error) => return error,
            },
            err.to_string(),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandlerKind {
    Query,
    Mutation,
    Subscription,
}

impl TryFrom<&str> for HandlerKind {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Ok(match s {
            "query" => Self::Query,
            "mutation" => Self::Mutation,
            "subscription" => Self::Subscription,
            _ => return Err(()),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use quote::quote;
    use rstest::*;

    #[rstest]
    #[case::query(quote!(query), Attributes::query())]
    #[case::mutation(quote!(mutation), Attributes::mutation())]
    #[case::subscription(quote!(subscription), Attributes::subscription())]
    #[case::kind_name(quote!(query, name = "other_name"), Attributes::query().with_name("other_name"))]
    #[case::name_kind(quote!(name = "other_name", mutation), Attributes::mutation().with_name("other_name"))]
    fn parse_attributes(#[case] tokens: TokenStream, #[case] expected: Attributes) {
        let attrs = Attributes::parse(tokens).unwrap();
        assert_eq!(attrs, expected);
    }

    // TODO: These tests should somehow verify the which error is returned, and what the span
    // points to.
    #[rstest]
    #[case::multiple_kind(quote!(query, mutation))]
    #[case::no_kind(quote!(name = "other_name"))]
    #[case::multiple_name(quote!(query, name = "name_1", name = "name_2"))]
    fn parse_attributes_fail(#[case] tokens: TokenStream) {
        assert!(Attributes::parse(tokens).is_err());
    }
}
