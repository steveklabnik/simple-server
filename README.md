# simple-server

> a crate for building a simple blocking HTTP server

## up and running

this crate is written in [the rust programming language]. you'll need rust to run
this crate. you can install rust using [rustup].

to get this crate running locally:

1. fork and clone this repository
2. `cd simple-server`
3. `cargo build`

to use this crate in your project, add the following line to your `Cargo.toml`:

```
"simple-server" = "https://github.com/steveklabnik/simple-server.git"
```

to see this crate in action, check out the [example].

[the rust programming language]: https://www.rust-lang.org
[rustup]: https://www.rustup.rs/
[example]: #example

## example

an example is provided with this crate. to run the example:

```
cargo run --example simple-server
```
