# Contributing to orko

Thank you for your interest in contributing to orko. This document sets out
the requirements and procedures for contributions. Submissions that follow it
can be reviewed and merged quickly; submissions that do not take more
time to review and maybe returned for revision.

## 1. Project Structure

orko is a Cargo workspace. Crates live under `crates/`:

| Crate            | Responsibility                                                          |
| ---------------- | ----------------------------------------------------------------------- |
| `orko-core`      | Core traits and types (`Provider`, `Tool`, `Agent`, `Message`). No I/O. |
| `orko-macros`    | The `#[tool]` procedural macro.                                         |
| `orko-providers` | Provider implementations and presets.                                   |
| `orko-graph`     | Event-graph building blocks.                                            |
| `orko-mcp`       | Model Context Protocol client and server.                               |
| `orko-runtime`   | Tokio-backed execution.                                                 |
| `orko`           | Batteries-included facade re-exporting the above behind features.       |

## 2. Architectural Invariants

The following rules are binding. A pull request that violates any of them
will not be merged, regardless of its other merits.

1. **Dependency direction.** Crate dependencies flow toward `orko-core`
   only. Sole exception: `orko-runtime` may additionally depend on
   `orko-graph`.
2. **Runtime agnosticism.** Tokio is permitted only inside `orko-runtime`
   implementations and must never appear in any public signature. Public
   traits use `std::future::Future` and `futures::Stream`; boxed futures are
   used only where object safety requires them.
3. **Core purity.** `orko-core` contains no HTTP client, no base URLs,
   and no I/O. Its dependencies are limited to `serde`, `serde_json`,
   `thiserror`, and `futures`.
4. **Unsafe code.** `unsafe` is forbidden in `orko-core`
   (`#![forbid(unsafe_code)]`) and denied in every other crate. Do not
   introduce `#[allow(unsafe_code)]` without stating clear reasoning for
   behind it.

## 3. Development Environment

A stable toolchain with `rustfmt` and `clippy` is pinned by
`rust-toolchain.toml` and `clippy.toml`. Installing [rustup](https://rustup.rs) is sufficient,
as the correct toolchain is selected automatically on first build.

```sh
git clone https://github.com/orko-rs/orko
cd orko
cargo build --workspace --all-features
```

## 4. Required Checks

Every change must pass all of the following before submission. These are the
same checks CI applies; running them locally first is expected.

```sh
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

In addition, every crate must continue to build with
`cargo build -p <crate> --no-default-features`.

## 5. Standards for Changes

1. **Scope.** One pull request addresses one concern. Unrelated
   refactoring, reformatting, or drive-by fixes belong in separate pull
   requests.
2. **Tests.** A behavioral change carries a test that fails without it.
   Bug fixes carry a regression test.
3. **Documentation.** All public items are documented; `orko-core` and
   `orko-macros` compile under `#![warn(missing_docs)]` and must remain
   warning-free. Rustdoc examples are preferred where they clarify usage.
4. **Dependencies.** New third-party dependencies require clear reasoning
   for their inclusion.
5. **Commit messages.** Use the imperative mood with a concise subject
   line. Reference related issues in the body where applicable.

## 6. Proposing Larger Changes

For any change that affects public API, adds a dependency, or spans multiple
crates, open an issue describing the motivation and intended design before
writing code. This protects contributors from investing effort in work that
cannot be accepted.

## 7. Reporting Issues

- **Bugs and feature requests**: open a GitHub issue with a minimal
  reproduction or a precise description of the desired behavior.
- **Security vulnerabilities**: do **not** open a public issue. Follow the
  procedure in [SECURITY.md](SECURITY.md).

## 8. Conduct

Participation in this project is governed by the
[Code of Conduct](CODE_OF_CONDUCT.MD). By participating, you agree to abide
by its terms.

## 9. License

orko is licensed under the Apache License, Version 2.0. Unless you
explicitly state otherwise, any contribution intentionally submitted for
inclusion in the work by you, as defined in the Apache-2.0 license, shall be
licensed as above, without any additional terms or conditions.
