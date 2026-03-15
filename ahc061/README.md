# AHC061

## Run

Download local tester from AHC061 page and move `tools` to under the this `ahc061` directory.

```console
cargo build --release
cd tools
cargo run -r --bin tester ../../target/release/a < in/0000.txt
```

or with [pahcer](https://github.com/terry-u16/pahcer)

```console
pahcer run
```
