# Sovra specification

## Status

This document describes the M0 contract. It is intentionally a foundation,
not a language grammar. M0 must not claim to lex, parse, type-check, interpret,
or compile Sovra source.

## Toolchain contract

* The package is named `sovra`; its canonical binary is `svr`.
* `svr --version` prints `svr <package-version>` and exits successfully.
* `svr --help` documents options and planned commands.
* `build`, `check`, `fmt`, `init`, and `run` are reserved commands. In M0 they
  return a non-zero status and explain that they are not implemented.
* Unknown commands are rejected with a non-zero status and a help hint.

## Milestone boundary

M1 owns token definitions and lexer behavior. No token or lexer functionality
belongs in M0; the `src/compiler/lexer.rs` module is only an extension point.

