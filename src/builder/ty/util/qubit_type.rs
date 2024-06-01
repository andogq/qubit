/// Name of the Qubit NPM package.
const CLIENT_PACKAGE: &str = "@qubit-rs/client";

/// Built-in Qubit types.
pub enum QubitType {
    /// Corresponds to `Stream` from ``@qubit-rs/client`.
    Stream,
}

impl QubitType {
    /// Produce the package and exported type corresponding to a Qubit type.
    pub fn to_ts(&self) -> (String, String) {
        match self {
            Self::Stream => (CLIENT_PACKAGE.to_string(), "Stream".to_string()),
        }
    }
}
