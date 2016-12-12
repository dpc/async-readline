extern crate futures;
extern crate tokio_core;
extern crate async_readline;

use futures::stream::Stream;
use tokio_core::io::{Io};
use tokio_core::reactor::Core;

use std::io::{self, Write};

use tokio_core::io::{Codec, EasyBuf};

struct CharCodec;

impl Codec for CharCodec {
    type In = u8;
    type Out = String;

    fn decode(&mut self, buf: &mut EasyBuf) -> Result<Option<Self::In>, io::Error> {
        if buf.len() == 0 {
            return Ok(None)
        }

        let ret = buf.as_ref()[0];
        buf.drain_to(1);
        Ok(Some(ret))
    }

    fn encode(&mut self, msg: Self::Out, buf: &mut Vec<u8>) -> io::Result<()> {
        write!(buf, "{}", msg)?;
        Ok(())
    }
}

fn main() {
    // Create the event loop that will drive this server
    let mut l = Core::new().unwrap();
    let handle = l.handle();

    let stdio = async_readline::RawStdio::new(&handle).unwrap();
    let (stdin, stdout, _) = stdio.split();

    let framed_in = stdin.framed(CharCodec);
    let framed_out = stdout.framed(CharCodec);

    let done = framed_in
        .map(move |ch| {
            format!("got: {}\n", ch)
        })
        .forward(framed_out);

    l.run(done).unwrap();
}
