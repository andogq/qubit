//! Implementation for values that may be returned as a response from
//! [`QubitHandler`](super::QubitHandler). See [`ResponseValue`].

use serde::Serialize;
use ts_rs::TS;

use super::marker::*;

/// Any Rust value that can be returned from a handler. It may require a transform function
/// to turn it into a serialisable value.
pub trait ResponseValue<MValue: ResponseMarker> {
    /// Serialisable value that will be produced.
    type Value: 'static + TS + Clone + Serialize;

    /// Transform into a serialisable value.
    fn transform(self) -> Self::Value;

    fn debug() -> String;
}

/// As a [`ResponseValue`], these values can be directly returned without any
/// transformation.
impl<T> ResponseValue<MTs> for T
where
    T: 'static + TS + Clone + Serialize,
{
    type Value = Self;

    fn transform(self) -> Self::Value {
        self
    }

    fn debug() -> String {
        "TS".to_string()
    }
}

/// As a [`ResponseValue`], the iterator will be collected into a `Vec` before being
/// returned.
///
/// The `MValue` generic is a marker for the value contained within the iterator.
impl<T, MItem> ResponseValue<MIter<MItem>> for T
where
    T: Iterator,
    T::Item: ResponseValue<MItem>,
    MItem: ResponseMarker,
{
    type Value = Vec<<T::Item as ResponseValue<MItem>>::Value>;

    fn transform(self) -> Self::Value {
        self.map(|value| value.transform()).collect()
    }

    fn debug() -> String {
        format!("Iter<{}>", T::Item::debug())
    }
}
