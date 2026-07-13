# Flynt patch for `block` 0.1.6

This directory vendors the `block` 0.1.6 crate (published as MIT) from
<https://github.com/SSheldon/rust-block> because it is an unmaintained
transitive dependency of Dioxus Desktop through `cocoa`.

Flynt's patch is intentionally narrow:

- model `_NSConcreteStackBlock` with an opaque inhabited `#[repr(C)]` marker
  instead of an uninhabited enum, avoiding Rust future-incompatibility lint
  `uninhabited_static` (rust-lang/rust#74840);
- make the crate's implicit C ABIs explicit for Rust 2024 compatibility.

Remove `[patch.crates-io] block = ...` and this directory once the Dioxus/Cocoa
dependency chain no longer resolves to `block` 0.1.6.
