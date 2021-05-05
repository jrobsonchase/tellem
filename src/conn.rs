use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::codec::Error;
use crate::event::Event;
use crate::handler::{ErasedHandler, Handler};
use crate::parser::Parser;

use pin_project::pin_project;

#[allow(unused_imports)]
use tracing::{debug, info, span, trace, warn, Instrument, Level};

use futures::prelude::*;
use futures::{
    channel::mpsc,
    stream::{SplitSink, SplitStream},
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    runtime::Handle,
    select,
};

use tokio_util::codec::{Decoder, Framed};

trait_alias!(SrvSink = Sink<Event, Error = io::Error> + Unpin + Send + 'static);
trait_alias!(SrvStream = Stream<Item = Result<Event, Error>> + Unpin + Send + 'static);

trait_alias!(ClSink = Sink<Result<Event, Error>, Error = io::Error> + Unpin + Send + 'static);
trait_alias!(ClStream = Stream<Item = Event> + Unpin + Send + 'static);

#[pin_project]
pub struct TnConn<T, R> {
    #[pin]
    sink: T,
    #[pin]
    stream: R,
}

pub type SrvConn = TnConn<Box<dyn SrvSink>, Box<dyn SrvStream>>;
pub type ClConn = TnConn<Box<dyn ClSink>, Box<dyn ClStream>>;

impl<I, T, R> Sink<I> for TnConn<T, R>
where
    T: Sink<I>,
{
    type Error = <T as Sink<I>>::Error;

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().sink.poll_close(cx)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().sink.poll_flush(cx)
    }

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().sink.poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: I) -> Result<(), Self::Error> {
        self.project().sink.start_send(item)
    }
}

impl<T, R> Stream for TnConn<T, R>
where
    R: Stream,
{
    type Item = <R as Stream>::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().stream.poll_next(cx)
    }
}

trait_alias!(AsyncStream = AsyncRead + AsyncWrite + Unpin + Send + 'static);

impl TnConn<(), ()> {
    pub fn start<S>(
        stream: S,
    ) -> TnConn<SplitSink<Framed<S, Parser>, Event>, SplitStream<Framed<S, Parser>>>
    where
        S: AsyncStream,
    {
        let (conn_tx, conn_rx) = Parser::default().framed(stream).split();
        TnConn {
            sink: conn_tx,
            stream: conn_rx,
        }
    }

    pub fn start_erased_stream<S>(
        stream: S,
    ) -> TnConn<
        SplitSink<Framed<Box<dyn AsyncStream>, Parser>, Event>,
        SplitStream<Framed<Box<dyn AsyncStream>, Parser>>,
    >
    where
        S: AsyncStream,
    {
        let (conn_tx, conn_rx) = Parser::default().framed(Box::new(stream) as _).split();
        TnConn {
            sink: conn_tx,
            stream: conn_rx,
        }
    }

    fn pair() -> TnChan<impl SrvSink, impl SrvStream, impl ClSink, impl ClStream> {
        let (cl_tx, srv_rx) = mpsc::unbounded::<Result<Event, Error>>();
        let (srv_tx, cl_rx) = mpsc::unbounded::<Event>();

        let upstream = TnConn {
            sink: srv_tx.sink_map_err(|_| io::ErrorKind::UnexpectedEof.into()),
            stream: srv_rx,
        };

        let downstream = TnConn {
            sink: cl_tx.sink_map_err(|_| io::ErrorKind::UnexpectedEof.into()),
            stream: cl_rx,
        };

        return TnChan {
            upstream,
            downstream,
        };
    }
}

pub type TnConnErased = TnConn<Box<dyn SrvSink>, Box<dyn SrvStream>>;

impl<T, R> TnConn<T, R>
where
    T: SrvSink,
    R: SrvStream,
{
    pub fn split(self) -> (T, R) {
        (self.sink, self.stream)
    }

    pub fn into_erased(self) -> TnConnErased {
        TnConn {
            sink: Box::new(self.sink) as _,
            stream: Box::new(self.stream) as _,
        }
    }

    pub fn with_handler(self, handler: impl Handler<T, R>) -> TnConn<impl SrvSink, impl SrvStream> {
        start_stream(Handle::current(), handler, self)
    }

    pub fn with_handler_erased(
        self,
        handler: impl ErasedHandler,
    ) -> TnConn<impl SrvSink, impl SrvStream> {
        start_stream(Handle::current(), handler, self.into_erased())
    }
}

pub struct TnChan<UTx, URx, DTx, DRx> {
    pub upstream: TnConn<UTx, URx>,
    pub downstream: TnConn<DTx, DRx>,
}

impl<UTx, URx, DTx, DRx> TnChan<UTx, URx, DTx, DRx> {
    fn replace_upstream<NTx, NRx>(
        self,
        new: TnConn<NTx, NRx>,
    ) -> (TnConn<UTx, URx>, TnChan<NTx, NRx, DTx, DRx>) {
        (
            self.upstream,
            TnChan {
                upstream: new,
                downstream: self.downstream,
            },
        )
    }
}

fn start_stream<T, R>(
    handle: Handle,
    mut handler: impl Handler<T, R>,
    upstream: TnConn<T, R>,
) -> TnConn<impl SrvSink, impl SrvStream>
where
    T: SrvSink,
    R: SrvStream,
{
    let (ret, mut chan) = TnConn::pair().replace_upstream(upstream);

    let task = async move {
        loop {
            select! {
                to_send = chan.downstream.next() => {
                    match to_send {
                        Some(event) => {
                            if let Err(error) = handler.send_event(&mut chan, event).await {
                                debug!(?error,"upstream send error")
                            }
                        }
                        None => break,
                    }

                }
                recvd = chan.upstream.try_next() => {
                    match recvd {
                        Ok(Some(event)) => {
                            if let Err(error) = handler.receive_event(&mut chan, event).await  {
                                debug!(?error, "downstream send error");
                            }
                        }
                        Ok(None) => {
                            break;
                        }
                        Err(error) => {
                            debug!(?error, "upstream receive error");
                            if let Err(error) = chan.downstream.send(Err(error)).await {
                                debug!(?error, "downstream send error");
                            }
                            break;
                        }
                    }

                }
            }
        }
    };

    handle.spawn(task);

    return ret;
}
