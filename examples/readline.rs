extern crate futures;
extern crate tokio_core;
extern crate async_readline;

use futures::stream::Stream;
use tokio_core::reactor::Core;

use std::io::Write;

use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    // Create the event loop that will drive this server
    let mut l = Core::new().unwrap();
    let handle = l.handle();

    let periodic_timer1 = tokio_core::reactor::Interval::new(std::time::Duration::from_millis(2500), &handle).unwrap();
    let periodic_timer2 = tokio_core::reactor::Interval::new(std::time::Duration::from_millis(500), &handle).unwrap();
    let stdio = async_readline::RawStdio::new(&handle).unwrap();
    let (stdin, stdout, _) = stdio.split();


    let (commands, rl_writer) = async_readline::init(stdin, stdout);

    let acc1 = Rc::new(RefCell::new(0));
    let acc2 = acc1.clone();
    let acc3 = acc1.clone();
    let connected1 = Rc::new(RefCell::new(false));
    let connected2 = connected1.clone();
    let connected3 = connected1.clone();

    let done = commands
        .map(move |line| {
            *connected1.borrow_mut() = false;
            *acc1.borrow_mut() = 0;

            let mut v = vec!();
            let _ = write!(v, "\n> ");
            v.append(&mut line.line.clone());
            v
        })
        .select(
            periodic_timer1
            .map(|_| {
                let mut v = vec!();
                if *connected2.borrow() {
                    *acc2.borrow_mut() += 1;
                    let _ = write!(v, "\nFiles ready: {} ", *acc2.borrow());
                }
                v
            })
            )
        .select(
            periodic_timer2.map(|_| {
                let mut v = vec!();
                if !*connected3.borrow() {
                    *acc3.borrow_mut() += 1;
                    if *acc3.borrow() > 3 {
                        let _ = write!(v, "\nConnected!");
                        *acc3.borrow_mut() = 0;
                        *connected3.borrow_mut() = true;
                    } else {
                        let _ = write!(v, "\nConnecting... ");
                    }
                } else if *acc3.borrow() > 10 {
                    let _ = write!(v, "Disconnecting... ");
                    *acc3.borrow_mut() = 0;
                    *connected3.borrow_mut() = false;
                } else if *acc3.borrow() > 5 {
                    let _ = write!(v, "Error... ");
                }

                v
            })
            )
            .forward(rl_writer);

    l.run(done).unwrap();
}
