use bytes::{
    BufMut,
    BytesMut,
};

use crate::op::Cmd;

pub trait Escape {
    fn escape_to(&self, dst: &mut BytesMut);
    fn unescape_to(&self, dst: &mut BytesMut);

    fn unescape_inplace(&mut self);
}

impl Escape for BytesMut {
    fn escape_to(&self, dst: &mut BytesMut) {
        dst.reserve(self.len());
        for byte in self {
            if *byte == Cmd::IAC {
                dst.put_u8(*byte);
            }

            dst.put_u8(*byte)
        }
    }

    fn unescape_to(&self, dst: &mut BytesMut) {
        dst.reserve(self.len());
        let mut iter = self.iter();
        while let Some(b) = iter.next() {
            if *b == Cmd::IAC {
                iter.next();
            }
            dst.put_u8(*b);
        }
    }

    fn unescape_inplace(&mut self) {
        let mut removed = 0;
        let mut j = 0;
        let mut skip = false;
        for i in 0..self.len() {
            if skip {
                removed += 1;
                continue;
            }
            if self[i] == Cmd::IAC as u8 {
                skip = true
            }
            if i != j {
                self[j] = self[i];
            }
            j += 1;
        }

        self.truncate(self.len() - removed)
    }
}
