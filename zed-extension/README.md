# Shine for Zed

This directory is a local Zed language extension. It associates `.shn` files
with Shine and provides non-monotone Tree-sitter highlighting for comments,
strings, numbers, booleans, keywords, built-ins, types, operators, and names.

The grammar is kept as a small local Git repository because Zed's extension
builder checks out every grammar by a pinned Git revision. The manifest points
to the current absolute path on this machine; when publishing, replace it
with a public Tree-sitter grammar repository URL.

Install it from Zed's command palette with `zed: install dev extension`, then
select this directory. Zed will automatically use the local Tree-sitter
grammar during development.

Zed compiles Tree-sitter C parsers to WASM. The first install needs its WASI
SDK once; the repaired cache on this machine is now at Zed's normal build
directory, so subsequent reloads do not download the SDK again.

The file icon is intentionally shipped as a separate package in
`../zed-icon-theme`, because Zed publishes language extensions and icon-theme
extensions separately. Install that directory the same way, then select
`Shine Icons` in `icon theme selector: toggle`.
