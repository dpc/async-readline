// TODO: http://rachid.koucha.free.fr/tech_corner/pty_pdip.html

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
use futures::{Async, AsyncSink};

use futures::sync::BiLock;

pub struct Line {
    pub line : Vec<u8>,
    pub text_last_nl : bool,
}

struct ReadlineInner {
    stdin : raw::PollFd,
    stdout : raw::PollFd,

    line : Vec<u8>,

    lines_ready : Vec<Vec<u8>>,

    text_last_nl : bool,


    wr_pending: Vec<u8>,
}

pub struct Lines {
    inner : BiLock<ReadlineInner>,
}

pub struct Writer {
    inner : BiLock<ReadlineInner>,
}

impl ReadlineInner {
    fn clear_line(&mut self) -> io::Result<()> {
        write!(self.wr_pending, "\x1b[2K")?;
        write!(self.wr_pending, "\x1b[1000D")?;
        Ok(())
    }

    fn redraw_line(&mut self) -> io::Result<()> {
        write!(self.wr_pending, "\x1b[2K")?;
        write!(self.wr_pending, "\x1b[1000D")?;
        write!(self.wr_pending, "> {}", String::from_utf8_lossy(&self.line))?;
        Ok(())
    }

    fn leave_prompt(&mut self) -> io::Result<()> {
        self.clear_line()?;
        self.restore_original()?;
        if !self.text_last_nl {
            write!(self.wr_pending, "\x1b[1B")?;
            write!(self.wr_pending, "\x1b[1A")?;
        }
        Ok(())
    }

    fn enter_prompt(&mut self) -> io::Result<()> {
        self.save_original()?;
        if !self.text_last_nl {
            //write!(self.wr_pending, "\x1b[1E")?;
            write!(self.wr_pending, "\n")?;
            self.clear_line()?;
        }
        Ok(())
    }

    fn save_original(&mut self) -> io::Result<()> {
        write!(self.wr_pending, "\x1b[s")?;
        Ok(())
    }

    fn restore_original(&mut self) -> io::Result<()> {
        write!(self.wr_pending, "\x1b[u")?;
        Ok(())
    }

    fn wr_pending_flush(&mut self) -> io::Result<()> {
        let n = self.stdout.write(&self.wr_pending)?;
        self.wr_pending.drain(..n);
        self.stdout.flush()?;
        Ok(())
    }

    fn handle_char(&mut self, ch : u8) {
        match ch {
            13 => self.lines_ready.push(std::mem::replace(&mut self.line, vec![])),
            127 => {
                let _ = self.line.pop();
            },
            _ => self.line.push(ch),
        }
    }

    fn poll_command(&mut self) -> futures::Poll<Option<Line>, io::Error> {
        let mut tmp_buf = [0u8; 16];

        loop {
            let _ = self.wr_pending_flush();

            if let Some(line) = self.lines_ready.pop() {
                self.clear_line()?;
                let _ = self.wr_pending_flush();
                return Ok(
                    Async::Ready(Some(
                            Line {
                                line: line,
                                text_last_nl: self.text_last_nl
                            }
                            ))
                    )
            }

            // FIXME: 0 means EOF?
            let bytes_read = try_nb!(self.stdin.read(&mut tmp_buf));

            for ch in &tmp_buf[..bytes_read] {
                self.handle_char(*ch)
            }

            self.redraw_line()?;
        }
    }

    fn start_write(&mut self, mut item: Vec<u8>) -> futures::StartSend<Vec<u8>, io::Error> {
        if item.len() > 0 {
            self.leave_prompt()?;
            self.text_last_nl = item[item.len() - 1] == 10;
            self.wr_pending.append(&mut item);
            self.enter_prompt()?;
            self.redraw_line()?;
        }
        Ok(AsyncSink::Ready)
    }

    fn poll_write_complete(&mut self) -> futures::Poll<(), io::Error> {
        loop {
            try_nb!(self.wr_pending_flush());

            if self.wr_pending.len() == 0 {
                return Ok(Async::Ready(()));
            }

        }
    }
}

impl futures::Stream for Lines {
    type Item = Line;
    type Error = io::Error;

    fn poll(&mut self) -> futures::Poll<Option<Self::Item>, Self::Error> {
        if let Async::Ready(mut guard) = self.inner.poll_lock() {
            guard.poll_command()
        } else {
            Ok(Async::NotReady)
        }

    }
}

impl futures::Sink for Writer {
    type SinkItem = Vec<u8>;
    type SinkError = io::Error;

    fn start_send(&mut self, item: Self::SinkItem) -> futures::StartSend<Self::SinkItem, io::Error> {
        if let Async::Ready(mut guard) = self.inner.poll_lock() {
            guard.start_write(item)
        } else {
            Ok(AsyncSink::NotReady(item))
        }
    }

    fn poll_complete(&mut self) -> futures::Poll<(), io::Error> {
        if let Async::Ready(mut guard) = self.inner.poll_lock() {
            guard.poll_write_complete()
        } else {
            Ok(Async::NotReady)
        }
    }
}

pub fn init(stdin : PollFd, stdout : PollFd) -> (Lines, Writer) {
    let mut inner = ReadlineInner {
        stdin: stdin,
        stdout : stdout,
        line: vec![],
        text_last_nl: true,
        wr_pending : vec!(),
        lines_ready : vec![],
    };

    let _ = inner.enter_prompt();

    let (l1, l2) = BiLock::new(inner);



    let writer = Writer { inner: l1 };
    let lines = Lines { inner: l2 };
    (lines, writer)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
