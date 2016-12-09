//! An echo server that just writes back everything that's written to it.
//!
//! If you're on unix you can test this out by in one terminal executing:
//!
//! ```sh
//! $ cargo run --example echo
//! ```
//!
//! and in another terminal you can run:
//!
//! ```sh
//! $ nc localhost 8080
//! ```
//!
//! Each line you type in to the `nc` terminal should be echo'd back to you!

extern crate futures;
extern crate tokio_core;
extern crate tokio_readline;

use std::env;
use std::net::SocketAddr;

use futures::Future;
use futures::stream::Stream;
use tokio_core::io::{copy, Io};
use tokio_core::net::TcpListener;
use tokio_core::reactor::Core;

fn main() {
    // Create the event loop that will drive this server
    let mut l = Core::new().unwrap();
    let handle = l.handle();

    let mut buf = [0u8; 256];
    let stdio = tokio_readline::RawAsyncStdio::new(&handle).unwrap();
    let stdin = stdio.stdin(&mut buf);

    // Pull out the stream of incoming connections and then for each new
    // one spin up a new task copying data.
    //
    // We use the `io::copy` future to copy all data from the
    // reading half onto the writing half.
    let done = stdin.for_each(move |count| {
        println!("got: {} {:?}", count, &buf[..count]);
        Ok(())
    });
    l.run(done).unwrap();
}
