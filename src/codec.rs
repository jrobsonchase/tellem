use std::io;

use bytes::{
    BufMut,
    BytesMut,
};
use either::Either;
use thiserror::Error;
use tokio_util::codec::{
    Decoder,
    Encoder,
};

use crate::{
    event::Event,
    op::Cmd,
    parser::{
        self,
        Parser,
    },
    util::Escape,
};

/// Errors arising from the telnet codec
#[derive(Debug, Error)]
pub enum Error {
    /// An error occurred while parsing
    #[error("parse error: {0}")]
    Parse(#[from] parser::Error),
    /// An error occurred in the underlying IO stream
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

impl Decoder for Parser {
    type Error = Error;
    type Item = Event;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(loop {
            if buf.is_empty() {
                break None;
            }
            match self.parse(buf)? {
                Either::Left(true) => continue,
                Either::Left(false) => break None,
                Either::Right(item) => break item.into(),
            }
        })
    }
}

impl Encoder<Event> for Parser {
    type Error = io::Error;

    fn encode(&mut self, item: Event, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            Event::Data(bytes) => {
                bytes.escape_to(dst);
            }
            Event::Cmd(cmd) => {
                dst.put_u8(Cmd::IAC.into());
                dst.put_u8(cmd.into());
            }
            Event::Negotiation(cmd, opt) => {
                dst.put_u8(Cmd::IAC.into());
                dst.put_u8(cmd.into());
                dst.put_u8(opt.into());
            }
            Event::Subnegotiation(opt, params) => {
                dst.put_u8(Cmd::IAC.into());
                dst.put_u8(Cmd::SB.into());
                dst.put_u8(opt.into());
                params.escape_to(dst);
                dst.put_u8(Cmd::IAC.into());
                dst.put_u8(Cmd::SE.into());
            }
        }

        Ok(())
    }
}
