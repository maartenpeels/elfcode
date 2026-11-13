# elfcode-build

Build script support for [elfcode](https://github.com/maartenpeels/elfcode). It finds the day files of a solutions crate and generates their modules, their registry, and one test per example.

```rust
// build.rs
fn main() {
    elfcode_build::generate();
}
```

See the [elfcode README](https://github.com/maartenpeels/elfcode#readme) for the whole setup.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.
