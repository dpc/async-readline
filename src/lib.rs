extern crate mio;
#[macro_use]
extern crate tokio_core;
extern crate libc;
extern crate nix;
extern crate termios;
extern crate futures;

mod raw;

pub use raw::*;

use std::io::{self, Read, Write};
use futures::Async;


pub struct Commands {
    stdin : raw::PollFd,

    line_len : usize,
    line_buf : Vec<u8>,
}

impl Commands {
    /// TODO: This should be asynchronous, non blocking, handled inside poll()
    fn clear_line(&self) -> io::Result<()> {
        let mut stdout = std::io::stdout();
        write!(stdout, "\x1b[1000D")?;
        stdout.flush()?;
        Ok(())
    }

    /// TODO: This should be asynchronous, non blocking, handled inside poll()
    fn redraw_line(&self) -> io::Result<()> {
        let mut stdout = std::io::stdout();
        write!(stdout, "\x1b[1000D")?;
        write!(stdout, "{}", String::from_utf8_lossy(&self.line_buf[..self.line_len]))?;
        stdout.flush()?;
        Ok(())
    }
}

impl futures::Stream for Commands {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn poll(&mut self) -> futures::Poll<Option<Self::Item>, Self::Error> {
        loop {
            if self.line_len == self.line_buf.len() {
                self.line_buf.resize(self.line_len * 2, 0u8);
            }

            let prev_line_len = self.line_len;
            // FIXME: 0 means EOF?
            let bytes_read = try_nb!(self.stdin.read(&mut self.line_buf[self.line_len..]));
            self.line_len += bytes_read;

            let has_newline = self.line_buf[prev_line_len..self.line_len].iter().find(|&&b| b == 13).is_some();
            if has_newline {
                self.clear_line()?;
                self.line_buf.truncate(self.line_len);
                self.line_len = 0;
                let ret = std::mem::replace(&mut self.line_buf, vec![0u8; 8]);
                return Ok(Async::Ready(Some(ret)))
            } else {
                self.redraw_line()?;
            }
        }
    }
}

pub fn init(stdin : PollFd) -> Commands {
    Commands {
        stdin: stdin,
        line_buf: vec![0u8; 8],
        line_len: 0,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
