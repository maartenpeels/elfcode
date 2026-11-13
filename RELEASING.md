# Releasing

All crates share one version and are always released together.

1. Set the new version in the root `Cargo.toml`, both under `[workspace.package]` and in the three exact `=` requirements of the internal crates.
2. In `CHANGELOG.md`, turn `Unreleased` into the new version with today's date.
3. If any message type in `elfcode-core` changed shape, bump `PROTOCOL_VERSION`.
4. Check that everything packages, in dependency order:

   ```sh
   cargo publish --workspace --dry-run
   ```

5. Commit, tag `vX.Y.Z`, and push both.
6. Publish:

   ```sh
   cargo publish --workspace
   ```
