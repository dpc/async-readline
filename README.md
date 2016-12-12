# async-readline

Asynchronous readline-like interface.

This is a PoC library implementing a CLI interface that supports asynchronous
command editing and terminal output. In other words: user can keep editing the input
while the terminal output can be added at the same time.

Everything is asynchronous and reactive to stdio. No additional threads are involved.

It's implemented in Rust, and on top of Rust's futures and tokio library.

## Run the demo example:

```
cargo run --example readline
```

And you should see something like this:

![async-readline](http://i.imgur.com/nzL3gwz.gif)
