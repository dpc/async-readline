//! This should be moved to separate crate and API cleaned up

use libc;
use tokio_core;
use nix;
use mio;
use termios;

use mio::unix::EventedFd;
use tokio_core::reactor::PollEvented;
use std::io;

static STDIN_FILENO: libc::c_int = libc::STDIN_FILENO;
static STDOUT_FILENO: libc::c_int = libc::STDOUT_FILENO;
static STDERR_FILENO: libc::c_int = libc::STDERR_FILENO;

pub type Mode = termios::Termios;


pub struct StdioFd(mio::unix::EventedFd<'static>);

impl StdioFd {
    fn stdin() -> Self {
        StdioFd(EventedFd(&STDIN_FILENO)).set_nonblocking()
    }
    fn stdout() -> Self {
        StdioFd(EventedFd(&STDOUT_FILENO)).set_nonblocking()
    }
    fn stderr() -> Self {
        StdioFd(EventedFd(&STDERR_FILENO)).set_nonblocking()
    }

    fn set_nonblocking(self) -> Self {
        use nix::fcntl::{fcntl, FcntlArg, O_NONBLOCK};
        fcntl(*(self.0).0, FcntlArg::F_SETFL(O_NONBLOCK)).expect("fcntl");
        self
    }
}

impl mio::Evented for StdioFd {
    fn register(&self, poll: &mio::Poll, token: mio::Token, interest: mio::Ready, opts: mio::PollOpt) -> io::Result<()> {
        self.0.register(poll, token, interest, opts)
    }

    fn reregister(&self, poll: &mio::Poll, token: mio::Token, interest: mio::Ready, opts: mio::PollOpt) -> io::Result<()> {
        self.0.reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &mio::Poll) -> io::Result<()> {
        self.0.deregister(poll)
    }
}

impl io::Read for StdioFd {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            match nix::unistd::read(*(self.0).0, buf) {
                Err(nix::Error::Sys(nix::errno::EINTR)) => {
                    // continue
                },
                Err(nix::Error::Sys(e)) => return Err(e.into()),
                Err(nix::Error::InvalidPath) => return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid path")),
                Ok(count) => return Ok(count),
            }
        }
    }
}

impl io::Write for StdioFd {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        loop {
            match nix::unistd::write(*(self.0).0, buf) {
                Err(nix::Error::Sys(nix::errno::EINTR)) => {
                    // continue
                },
                Err(nix::Error::Sys(e)) => return Err(e.into()),
                Err(nix::Error::InvalidPath) => return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid path")),
                Ok(count) => return Ok(count),
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        // No buffering, no flushing
        Ok(())
    }
}

pub struct RawStdio {
    stdin : PollEvented<StdioFd>,
    stdout : PollEvented<StdioFd>,
    stderr : PollEvented<StdioFd>,
    stdin_isatty : bool,
}

pub type PollFd = PollEvented<StdioFd>;

impl RawStdio {

    pub fn new(handle : &tokio_core::reactor::Handle) -> io::Result<Self> {
        let stdin_poll_evented = PollEvented::new(StdioFd::stdin(), handle)?;
        let stdout_poll_evented = PollEvented::new(StdioFd::stdout(), handle)?;
        let stderr_poll_evented = PollEvented::new(StdioFd::stderr(), handle)?;
        let raw_stdio = RawStdio {
            stdin : stdin_poll_evented,
            stdout : stdout_poll_evented,
            stderr : stderr_poll_evented,
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

    pub fn split(self) -> (PollEvented<StdioFd>, PollEvented<StdioFd>, PollEvented<StdioFd>) {
        (self.stdin, self.stdout, self.stderr)
    }
}
