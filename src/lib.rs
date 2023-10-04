#![warn(missing_docs)]

//! A simple(-ish) telnet protocol implementation.

#[macro_use]
mod macros;

mod op;
mod util;
#[doc(inline)]
pub use op::*;
mod parser;
#[doc(inline)]
pub use parser::Parser;
mod event;
#[doc(inline)]
pub use event::*;

#[cfg(feature = "codec")]
mod codec;
#[cfg(feature = "codec")]
#[doc(inline)]
pub use codec::*;
