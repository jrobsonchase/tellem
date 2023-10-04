use bytes::BytesMut;

use crate::op::{
    Cmd,
    Opt,
};

/// A parsed telnet event
#[derive(Debug, Eq, Clone, PartialEq)]
pub enum Event {
    /// In-band data. Not necessarily a full line.
    Data(BytesMut),
    /// A telnet command.
    Cmd(Cmd),
    /// A telnet negotiation.
    Negotiation(Cmd, Opt),
    /// A telnet subnegotiation.
    Subnegotiation(Opt, BytesMut),
}
