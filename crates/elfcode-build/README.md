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

Licensed under [MIT license](LICENSE-MIT).
