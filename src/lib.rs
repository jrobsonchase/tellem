#[macro_use]
mod macros;

pub mod op;
mod util;
pub use op::*;
pub mod parser;
pub use parser::Parser;
pub mod event;
pub use event::*;

#[cfg(feature = "codec")]
pub mod codec;
#[cfg(feature = "codec")]
pub use codec::*;

#[cfg(feature = "conn")]
pub mod conn;
#[cfg(feature = "conn")]
pub use conn::*;

#[cfg(feature = "codec")]
pub mod handler;
#[cfg(feature = "codec")]
pub use handler::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
