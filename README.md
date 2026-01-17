# Schemer

Horrible Scheme implementation written in Rust. Currently incomplete and bad, is not reccomended for use of any kind.

Current goal is a minimal implementation of [R7RS](https://standards.scheme.org/official/r7rs.pdf)


## Building

```sh
cargo build --bin cli
```

## CLI Usage (with Cargo)

To start the repl:
```sh
cargo run --bin schemer
```

Providing a file to the enviroment:
```sh
cargo run --bin schemer -- --path examples/game-of-life.scm
```

**install scripts are not yet created, the same options are availble on the binraries within `./target`**
