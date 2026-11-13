# Changelog

All notable changes to this project are documented here. The project follows [Semantic Versioning](https://semver.org).

## Unreleased

The first release.

### Added

- `Solution` trait with a borrowed parsed input, and `Unsolved` for parts not written yet.
- Build script support that finds `src/y{YYYY}/day{DD}.rs` files and generates their registry, failing the build on files that break the naming.
- One `cargo test` test per example per part, from `data/{YYYY}/{DD}/examples.toml`.
- `cargo elfcode login` to verify and store the session cookie.
- `cargo elfcode scaffold` to create a day's files and download its input and puzzle text.
- `cargo elfcode run` to run a day with timings, downloading its input when missing.
- `cargo elfcode run --submit` with local checks against answers that cannot be right, and a history of every submission.
