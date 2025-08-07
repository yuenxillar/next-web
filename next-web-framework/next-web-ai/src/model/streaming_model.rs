use futures_core::Stream;


pub trait StreamingModel<TReq, TResChunk>: Send + Sync {
    fn stream<S>(request: TReq) -> S
    where
        S: Stream<Item = TResChunk> + Send + 'static;
}
