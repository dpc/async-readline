extern crate mio;
extern crate tokio_core;
extern crate libc;
extern crate nix;
extern crate termios;
extern crate futures;

use mio::unix::EventedFd;
use tokio_core::reactor::PollEvented;
use std::io;
use futures::Async;

static STDIN_FILENO: libc::c_int = libc::STDIN_FILENO;

pub type Mode = termios::Termios;

pub struct RawAsyncStdio {
    stdin : PollEvented<EventedFd<'static>>,
    stdin_isatty : bool,
}


impl RawAsyncStdio {

    pub fn new(handle : &tokio_core::reactor::Handle) -> io::Result<Self> {
        let evented_fd = EventedFd(&STDIN_FILENO);
        let poll_evented = PollEvented::new(evented_fd, handle)?;
        let raw_stdio = RawAsyncStdio {
            stdin : poll_evented,
            stdin_isatty : true,
        };
        raw_stdio.enable_raw_mode()?;
        Ok(raw_stdio)
    }

    fn enable_raw_mode(&self) -> io::Result<termios::Termios> {
        let mut orig_term = termios::Termios::from_fd(STDIN_FILENO)?;

        use nix::errno::Errno::ENOTTY;
        use termios::{BRKINT, CS8, ECHO, ICANON, ICRNL, IEXTEN, INPCK, ISIG, ISTRIP,
        IXON, /* OPOST, */ VMIN, VTIME};
        if !self.stdin_isatty {
            try!(Err(nix::Error::from_errno(ENOTTY)));
        }
        termios::tcgetattr(STDIN_FILENO, &mut orig_term)?;
        let mut raw = orig_term;
        // disable BREAK interrupt, CR to NL conversion on input,
        // input parity check, strip high bit (bit 8), output flow control
        raw.c_iflag &= !(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
        // we don't want raw output, it turns newlines into straight linefeeds
        // raw.c_oflag = raw.c_oflag & !(OPOST); // disable all output processing
        raw.c_cflag |=  CS8; // character-size mark (8 bits)
        // disable echoing, canonical mode, extended input processing and signals
        raw.c_lflag &= !(ECHO | ICANON | IEXTEN | ISIG);
        raw.c_cc[VMIN] = 1; // One character-at-a-time input
        raw.c_cc[VTIME] = 0; // with blocking read
        try!(termios::tcsetattr(STDIN_FILENO, termios::TCSADRAIN, &raw));
        Ok(orig_term)
    }

    pub fn stdin<'a, 'b>(&'a self, buf : &'b mut [u8]) -> RawAsyncStdin<'a, 'b> {
        RawAsyncStdin {
            stdio: self,
            buf: buf
        }
    }
}

pub struct RawAsyncStdin<'a, 'b> {
    stdio : &'a RawAsyncStdio,
    buf : &'b mut [u8],
}

impl<'a, 'b> futures::Stream for RawAsyncStdin<'a, 'b> {
    type Item = usize;
    type Error = io::Error;
    fn poll(&mut self) -> futures::Poll<Option<Self::Item>, Self::Error> {
        if let Async::NotReady = self.stdio.stdin.poll_read() {
            return Ok(Async::NotReady)
        }
        loop {
            match nix::unistd::read(*self.stdio.stdin.get_ref().0, self.buf) {

                Err(nix::Error::Sys(nix::errno::EWOULDBLOCK)) => {
                    self.stdio.stdin.need_read();
                    return Ok(Async::NotReady)
                },
                Err(nix::Error::Sys(nix::errno::EINTR)) => {
                    // continue
                },
                Err(nix::Error::Sys(e)) => return Err(e.into()),
                Err(nix::Error::InvalidPath) => return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid path")),
                Ok(0) =>  return Ok(Async::Ready(None)),
                Ok(count) => return Ok(futures::Async::Ready(Some(count))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
