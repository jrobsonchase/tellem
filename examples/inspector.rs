// use std::fmt::Debug;

use bytes::BytesMut;
use futures::prelude::*;
#[allow(unused_imports)]
use tracing::{debug, info, span, trace, warn, Instrument, Level, Span};
use tracing_subscriber::EnvFilter;

use tellem::*;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .pretty()
        .init();
    let dst = std::env::args().nth(1).expect("target server is required");

    let socket = tokio::net::TcpListener::bind("0.0.0.0:2323").await?;

    while let Ok((client_stream, client_addr)) = socket.accept().await {
        info!(%client_addr, "accepted connection");
        let conn = tokio::net::TcpStream::connect(&dst).await?;
        info!(%dst, "connected to remote");

        let (client_tx, client_rx) = span!(Level::INFO, "client_stream")
            .in_scope(move || TnConn::start(client_stream).split());
        let (server_tx, server_rx) = span!(Level::INFO, "server_stream").in_scope(|| {
            TnConn::start(conn)
                .with_handler(DebugHandler.instrumented(Span::current()))
                .with_handler_erased(ForceMSSP)
                .split()
        });

        forward_stream(span!(Level::INFO, "fwd_client"), client_rx, server_tx);
        forward_stream(span!(Level::INFO, "fwd_server"), server_rx, client_tx);
    }

    Ok(())
}

fn forward_stream<T, U, N, E, F>(span: tracing::Span, from: T, to: U)
where
    T: Stream<Item = Result<N, E>> + Unpin + Send + 'static,
    U: Sink<N, Error = F> + Unpin + Send + 'static,
    E: std::fmt::Debug + From<F> + Send + 'static,
    N: std::fmt::Debug + Send + 'static,
{
    tokio::spawn(
        async move {
            let result = from.forward(to.sink_map_err(From::from)).await;
            info!(?result, "stream ended")
        }
        .instrument(span),
    );
}

struct ForceMSSP;

#[async_trait::async_trait]
impl<T, R> Handler<T, R> for ForceMSSP
where
    T: SrvSink,
    R: SrvStream,
{
    async fn send_negotiation(
        &mut self,
        chan: &mut TnChan<T, R, impl ClSink, impl ClStream>,
        cmd: Cmd,
        opt: Opt,
    ) -> Result<(), Error> {
        Ok(match opt {
            Opt::Known(KnownOpt::MSSP) => {}
            _ => chan.upstream.send(Event::Negotiation(cmd, opt)).await?,
        })
    }
    async fn receive_negotiation(
        &mut self,
        chan: &mut TnChan<T, R, impl ClSink, impl ClStream>,
        cmd: Cmd,
        opt: Opt,
    ) -> Result<(), Error> {
        Ok(match (cmd, opt) {
            (Cmd::WILL, Opt::Known(KnownOpt::MSSP)) => {
                chan.upstream.send(Event::Negotiation(Cmd::DO, opt)).await?
            }
            _ => {
                chan.downstream
                    .send(Ok(Event::Negotiation(cmd, opt)))
                    .await?
            }
        })
    }
    async fn receive_subnegotiation(
        &mut self,
        chan: &mut TnChan<T, R, impl ClSink, impl ClStream>,
        opt: Opt,
        params: BytesMut,
    ) -> Result<(), Error> {
        Ok(match opt {
            Opt::Known(KnownOpt::MSSP) => {
                info!(?params, "MSSP Data");
            }
            _ => {
                chan.downstream
                    .send(Ok(Event::Subnegotiation(opt, params)))
                    .await?
            }
        })
    }
}
