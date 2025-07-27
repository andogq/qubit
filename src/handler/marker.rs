//! TODO: Explain the utility of markers

use std::marker::PhantomData;

/// Marker trait for any markers that can be used as a [`ResponseValue`](response_value).
///
/// [response_value]: super::response::ResponseValue
pub trait ResponseMarker {}

/// Marker for anything that implements [`TS`].
pub struct MTs;
impl ResponseMarker for MTs {}

/// Marker for anything that implements [`Iterator`]. [`Iterator::Item`] (represented with the
/// `MItem` generic) can be any type that implements [`ResponseMarker`].
pub struct MIter<MItem: ResponseMarker>(PhantomData<MItem>);
impl<MItem> ResponseMarker for MIter<MItem> where MItem: ResponseMarker {}

/// Marker trait for any markers that can be used as a return value from [`QubitHandler`].
pub trait HandlerReturnMarker {}

/// Marker for any [`ResponseMarker`] which is directly returned.
pub struct MResponse<MValue: ResponseMarker>(PhantomData<MValue>);
impl<MValue> HandlerReturnMarker for MResponse<MValue> where MValue: ResponseMarker {}

/// Marker for any returned [`Futures`][Future]. The result of the future [`MReturn`] may be any
/// other [`HandlerReturnMarker`].
pub struct MFuture<MReturn: HandlerReturnMarker>(PhantomData<MReturn>);
impl<MReturn> HandlerReturnMarker for MFuture<MReturn> where MReturn: HandlerReturnMarker {}

/// Marker for a [`Stream`](futures::stream::Stream), consisting of [`ResponseMarker`].
pub struct MStream<MValue: ResponseMarker>(PhantomData<MValue>);
impl<MValue> HandlerReturnMarker for MStream<MValue> where MValue: ResponseMarker {}
