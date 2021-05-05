use crate::op::{Cmd, Opt};

use bytes::BytesMut;

#[derive(Debug, Eq, Clone, PartialEq)]
pub enum Event {
    Data(BytesMut),
    Cmd(Cmd),
    Negotiation(Cmd, Opt),
    Subnegotiation(Opt, BytesMut),
}
