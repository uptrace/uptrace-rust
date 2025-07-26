# Releasing

## Upgrade dependecies

Install `cargo-edit`:

```shell
cargo install cargo-edit
```

Then run:

```shell
cargo upgrade  --incompatible
```

## Running tests

To run tests:

```shell
cargo test
```

## Publishing a new version

To publish a new version:

1. Bump the version in `Cargo.toml`.
2. Run `cargo publish`.
