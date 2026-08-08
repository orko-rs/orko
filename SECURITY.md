# Security Policy

## Supported Versions

orko is pre-1.0. Security fixes are issued only for the most recent release
of each crate published on crates.io; earlier 0.x releases are not patched.

| Version            | Supported |
| ------------------ | --------- |
| Latest 0.x release | Yes       |
| Earlier releases   | No        |

## Reporting a Vulnerability

**Do not report suspected vulnerabilities through public GitHub issues,
discussions, or pull requests.**

Report privately through GitHub's private vulnerability reporting:
[github.com/orko-rs/orko/security/advisories/new](https://github.com/orko-rs/orko/security/advisories/new)
(the repository's **Security** tab → **Report a vulnerability**).

A report should include, where applicable:

- the affected crate(s) and version(s);
- a description of the vulnerability and its impact;
- reproduction steps or a proof of concept;
- any suggested remediation.

Reports are acknowledged within 7 days. We practice coordinated disclosure:
please allow up to 30 days from acknowledgment for a fix and advisory before
any public disclosure. We will keep you informed of progress and agree on a
disclosure date with you.

## Disclosure Process

1. The report is acknowledged and triaged; severity is assessed.
2. A fix is developed and reviewed privately.
3. A patched release is published to crates.io.
4. A security advisory is published (GitHub advisory, and RUSTSEC via the
   [RustSec Advisory Database](https://github.com/rustsec/advisory-db) where
   applicable), crediting the reporter unless anonymity is requested.

## Threat Model and Scope

orko is an agent-orchestration toolkit: it forwards conversations to language
model providers and executes **registered tools** selected by the model's
responses. The following boundaries define what constitutes a vulnerability
in orko itself.

### In scope

- Memory unsafety or undefined behavior in any orko crate.
- Violation of a documented invariant, including but not limited to:
  dispatch of a tool that was not registered, execution beyond the configured
  tool-turn cap, or delivery of one tool call's arguments to another tool.
- Incorrect or unsound code emitted by the `#[tool]` procedural macro.
- Panics, non-termination, or unbounded resource consumption triggerable by
  untrusted provider responses or model output.

### Out of scope

- **Conduct of user-registered tools.** Tools execute with the full
  privileges of the host process; orko provides no sandboxing, permission
  system, or capability restriction. The decision to register a tool, and every action that tool can perform is the integrator's responsibility.
- **Prompt injection.** Untrusted input that induces the model to invoke
  registered tools, with arguments of the attacker's choosing, is inherent to
  tool-calling agents and must be mitigated at the application layer (tool
  selection, argument validation inside tools, human confirmation for
  destructive actions).
- Vulnerabilities in third-party dependencies (report these upstream; they
  are addressed here by dependency updates).
- Handling of provider credentials by the embedding application.

## Hardening

- `unsafe` code is forbidden in `orko-core` (`#![forbid(unsafe_code)]`) and
  denied in all other crates.
- The workspace pins a stable toolchain and is kept warning-free under
  `clippy -D warnings`.
