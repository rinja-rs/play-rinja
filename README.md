# play-rinja

This is the source code for https://rinja-rs.github.io/play-rinja/ which allows you to
test the [rinja](https://crates.io/crates/rinja) directly in your web browser.

To run this website locally:

```
git submodule update --remote --no-recommend-shallow
rustup target add wasm32-unknown-unknown
cargo install trunk
trunk serve
```
