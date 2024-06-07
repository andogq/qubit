/// Name of the Qubit NPM package.
const CLIENT_PACKAGE: &str = "@qubit-rs/client";

/// Built-in Qubit types.
pub enum QubitType {
    Query,
    Mutation,
    Subscription,
    StreamHandler,
    StreamUnsubscribe,
}

impl QubitType {
    /// Produce the package and exported type corresponding to a Qubit type.
    pub fn to_ts(&self) -> (String, String) {
        match self {
            Self::Query => (CLIENT_PACKAGE.to_string(), "Query".to_string()),
            Self::Mutation => (CLIENT_PACKAGE.to_string(), "Mutation".to_string()),
            Self::Subscription => (CLIENT_PACKAGE.to_string(), "Subscription".to_string()),
            Self::StreamHandler => (CLIENT_PACKAGE.to_string(), "StreamHandler".to_string()),
            Self::StreamUnsubscribe => {
                (CLIENT_PACKAGE.to_string(), "StreamUnsubscribe".to_string())
            }
        }
    }
}
