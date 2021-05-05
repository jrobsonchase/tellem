use std::marker::PhantomData;

use tracing::{debug, Instrument, Span};

use crate::codec::Error;
use crate::conn::{ClSink, ClStream, SrvSink, SrvStream, TnChan};
use crate::event::Event;
use crate::{Cmd, Opt};

use async_trait::async_trait;
use bytes::BytesMut;
use futures::prelude::*;

#[async_trait]
pub trait Handler<T, R>: Send + 'static
where
    T: SrvSink,
    R: SrvStream,
{
    async fn receive_event(
        &mut self,
        chan: &mut TnChan<T, R, impl ClSink, impl ClStream>,
        event: Event,
    ) -> Result<(), Error> {
        match event {
            Event::Cmd(cmd) => self.receive_cmd(chan, cmd),
            Event::Data(data) => self.receive_data(chan, data),
            Event::Negotiation(cmd, opt) => self.receive_negotiation(chan, cmd, opt),
            Event::Subnegotiation(opt, params) => self.receive_subnegotiation(chan, opt, params),
        }
        .await
    }

    async fn send_event(
        &mut self,
        chan: &mut TnChan<T, R, impl ClSink, impl ClStream>,
        event: Event,
    ) -> Result<(), Error> {
        match event {
            Event::Cmd(cmd) => self.send_cmd(chan, cmd),
            Event::Data(data) => self.send_data(chan, data),
            Event::Negotiation(cmd, opt) => self.send_negotiation(chan, cmd, opt),
            Event::Subnegotiation(opt, params) => self.send_subnegotiation(chan, opt, params),
        }
        .await
    }

    async fn send_data(
        &mut self,
        chan: &mut TnChan<T, R, impl ClSink, impl ClStream>,
        data: BytesMut,
    ) -> Result<(), Error> {
        Ok(chan.upstream.send(Event::Data(data)).await?)
    }
    async fn send_cmd(
        &mut self,
        chan: &mut TnChan<T, R, impl ClSink, impl ClStream>,
        cmd: Cmd,
    ) -> Result<(), Error> {
        Ok(chan.upstream.send(Event::Cmd(cmd)).await?)
    }
    async fn send_negotiation(
        &mut self,
        chan: &mut TnChan<T, R, impl ClSink, impl ClStream>,
        cmd: Cmd,
        opt: Opt,
    ) -> Result<(), Error> {
        Ok(chan.upstream.send(Event::Negotiation(cmd, opt)).await?)
    }
    async fn send_subnegotiation(
        &mut self,
        chan: &mut TnChan<T, R, impl ClSink, impl ClStream>,
        opt: Opt,
        params: BytesMut,
    ) -> Result<(), Error> {
        Ok(chan
            .upstream
            .send(Event::Subnegotiation(opt, params))
            .await?)
    }

    async fn receive_data(
        &mut self,
        chan: &mut TnChan<T, R, impl ClSink, impl ClStream>,
        data: BytesMut,
    ) -> Result<(), Error> {
        Ok(chan.downstream.send(Ok(Event::Data(data))).await?)
    }
    async fn receive_cmd(
        &mut self,
        chan: &mut TnChan<T, R, impl ClSink, impl ClStream>,
        cmd: Cmd,
    ) -> Result<(), Error> {
        Ok(chan.downstream.send(Ok(Event::Cmd(cmd))).await?)
    }
    async fn receive_negotiation(
        &mut self,
        chan: &mut TnChan<T, R, impl ClSink, impl ClStream>,
        cmd: Cmd,
        opt: Opt,
    ) -> Result<(), Error> {
        Ok(chan
            .downstream
            .send(Ok(Event::Negotiation(cmd, opt)))
            .await?)
    }
    async fn receive_subnegotiation(
        &mut self,
        chan: &mut TnChan<T, R, impl ClSink, impl ClStream>,
        opt: Opt,
        params: BytesMut,
    ) -> Result<(), Error> {
        Ok(chan
            .downstream
            .send(Ok(Event::Subnegotiation(opt, params)))
            .await?)
    }
}

trait_alias!(ErasedHandler = Handler<Box<dyn SrvSink>, Box<dyn SrvStream>>);

pub struct NopHandler;

impl<T, R> Handler<T, R> for NopHandler
where
    T: SrvSink,
    R: SrvStream,
{
}

pub struct DebugHandler;

#[async_trait]
impl<T, R> Handler<T, R> for DebugHandler
where
    T: SrvSink,
    R: SrvStream,
{
    async fn send_event(
        &mut self,
        chan: &mut TnChan<T, R, impl ClSink, impl ClStream>,
        event: Event,
    ) -> Result<(), Error> {
        debug!(?event, "sending event");
        Ok(chan.upstream.send(event).await?)
    }
    async fn receive_event(
        &mut self,
        chan: &mut TnChan<T, R, impl ClSink, impl ClStream>,
        event: Event,
    ) -> Result<(), Error> {
        debug!(?event, "received event");
        Ok(chan.downstream.send(Ok(event)).await?)
    }
}

pub trait HandlerExt<T, R>: Handler<T, R> + Sized
where
    T: SrvSink,
    R: SrvStream,
{
    fn instrumented(self, span: Span) -> InstrumentedHandler<Self, T, R> {
        InstrumentedHandler { span, inner: self, _ph: PhantomData }
    }
}

impl<E, T, R> HandlerExt<T, R> for E
where
    E: Handler<T, R>,
    T: SrvSink,
    R: SrvStream,
{
}

pub struct InstrumentedHandler<H, T, R> {
    span: Span,
    inner: H,
    _ph: PhantomData<fn() -> (T, R)>,
}

#[async_trait]
impl<H, T, R> Handler<T, R> for InstrumentedHandler<H, T, R>
where
    H: Handler<T, R>,
    T: SrvSink,
    R: SrvStream,
{
    async fn send_event(
        &mut self,
        chan: &mut TnChan<T, R, impl ClSink, impl ClStream>,
        event: Event,
    ) -> Result<(), Error> {
        Ok(self
            .inner
            .send_event(chan, event)
            .instrument(self.span.clone())
            .await?)
    }
    async fn receive_event(
        &mut self,
        chan: &mut TnChan<T, R, impl ClSink, impl ClStream>,
        event: Event,
    ) -> Result<(), Error> {
        Ok(self
            .inner
            .receive_event(chan, event)
            .instrument(self.span.clone())
            .await?)
    }
}
