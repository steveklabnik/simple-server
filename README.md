# simple-server

> a crate for building a simple blocking HTTP server

[![Build Status](https://travis-ci.org/steveklabnik/simple-server.svg?branch=master)](https://travis-ci.org/steveklabnik/simple-server)

## up and running

this crate is written in [the rust programming language]. you'll need rust to run
this crate. you can install rust using [rustup]. `simple-server` requires that you
use **rust version 1.20+**.

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

## tests

to test this crate locally, run:

```
cargo test
```

## docs

this crate has documentation. to build and open the docs locally:

```
cargo doc --open
```

## example

an example is provided with this crate. to run the example:

```
cargo run --example simple-server
```
