use std::convert::TryFrom;

use crate::event::Event;
use crate::op::{Cmd, Opt};
use crate::util::Escape;

use tracing::*;

use either::Either;

use thiserror::Error;

use bytes::{Buf, BytesMut};

#[derive(Default, Debug)]
pub struct Parser {
    sb_opt: Option<Opt>,
    sb_params: Option<BytesMut>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unknown telnet command: {0}")]
    UnknownCommand(u8),
    #[error("Malformed subnegotation")]
    MalformedSub,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Parser {
    #[instrument(level = "trace", skip(self, buf))]
    pub fn parse(&mut self, buf: &mut BytesMut) -> Result<Either<bool, Event>> {
        debug!(?buf, "parsing telnet event");
        let mut off = None;
        let mut iter = buf.iter().enumerate().peekable();
        while let Some((i, b)) = iter.next() {
            if *b == Cmd::IAC as u8 {
                let next = iter.peek().map(|(_, n)| **n);
                if next == Some(Cmd::IAC as u8) {
                    continue;
                } else {
                    off = Some(i);
                    break;
                }
            }
        }

        let data_len = match off {
            Some(0) => {
                return self.parse_iac(buf);
            }
            Some(n) => n,
            None => buf.len(),
        };

        if data_len == 0 {
            return Ok(Either::Left(false));
        }

        trace!("splitting the first {} bytes", data_len);
        let mut data = buf.split_to(data_len);

        match self.sb_params.as_mut() {
            Some(params) => {
                debug!(?params, "adding subnegotiation data");
                data.unescape_to(params);
                return Ok(Either::Left(true));
            }
            None => {
                data.unescape_inplace();
                Ok(Either::Right(Event::Data(data)))
            }
        }
    }

    #[instrument(skip(self, buf))]
    fn parse_iac(&mut self, buf: &mut BytesMut) -> Result<Either<bool, Event>> {
        debug!("parsing iac");
        if buf.len() < 2 {
            return Ok(Either::Left(false));
        }

        let cmd_op = buf[1];
        let cmd = Cmd::try_from(cmd_op).map_err(|_| Error::UnknownCommand(cmd_op))?;

        debug!(?cmd, "found command");

        match cmd {
            Cmd::SB => {
                let opt_op = match buf.get(2) {
                    Some(op) => *op,
                    None => return Ok(Either::Left(false)),
                };

                trace!("consuming 3 bytes");
                buf.advance(3);

                let opt = Opt::from(opt_op);

                if self.sb_opt.is_some() || self.sb_params.is_some() {
                    return Err(Error::MalformedSub);
                }

                self.sb_opt = Some(opt);
                self.sb_params = Some(BytesMut::new());
                return Ok(Either::Left(true));
            }
            Cmd::SE => {
                trace!("consuming 2 bytes");
                buf.advance(2);
                let opt = self.sb_opt.take();
                let params = self.sb_params.take();

                let opt = opt.ok_or(Error::MalformedSub)?;
                let params = params.ok_or(Error::MalformedSub)?;

                return Ok(Either::Right(Event::Subnegotiation(opt, params)));
            }
            Cmd::WILL | Cmd::WONT | Cmd::DO | Cmd::DONT => {
                let opt_op = match buf.get(2) {
                    Some(op) => *op,
                    None => return Ok(Either::Left(false)),
                };

                trace!("consuming 3 bytes");
                buf.advance(3);

                return Ok(Either::Right(Event::Negotiation(cmd, Opt::from(opt_op))));
            }
            other => {
                trace!("consuming 2 bytes");
                buf.advance(2);
                return Ok(Either::Right(Event::Cmd(other)));
            }
        }
    }
}
