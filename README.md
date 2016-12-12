# async-readline

Asynchronous readline-like interface.

This is a PoC library implementing a CLI interface that supports asynchronous
command editing and terminal output. In other words: user can keep editing the input
while the terminal output can continue to be added in the same time.

Everything is asynchronous and reactive to stdio. No additional threads are involved.

It's implemented in Rust, and on top of Rust's futures and tokio library.

## Run the demo example:

```
cargo run --example readline
```
