# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [6.0.0](https://github.com/open-schnick/DatadogFormattingLayer/compare/v5.0.0...v6.0.0) - 2025-06-25

### Other

- *(deps)* [**breaking**] Upgrade to opentelemetry 0.29
- fix clippy lints

## [5.0.0](https://github.com/open-schnick/DatadogFormattingLayer/compare/v4.0.1...v5.0.0) - 2025-05-29

### Other

- *(deps)* [**breaking**] Upgrade to opentelemetry 0.29
- add missing link to crates.io

## [4.0.1](https://github.com/open-schnick/DatadogFormattingLayer/compare/v4.0.0...v4.0.1) - 2025-03-02

### Fixed

- the first span now generates the trace id if none is present

## [4.0.0](https://github.com/open-schnick/DatadogFormattingLayer/compare/v3.0.0...v4.0.0) - 2025-03-02

### Other

- update README in preparation of releasing new version
- *(deps)* [**breaking**] Upgrade to opentelemetry 0.28
- move tests from layer into tests folder
- fix typos
- move test into doc test
- update smoothy and restructure tests

## [3.0.0](https://github.com/open-schnick/DatadogFormattingLayer/compare/v2.2.1...v3.0.0) - 2024-09-03

### Added
- [**breaking**] bump opentelemetry to 0.23.0

### Other
- adapt README to reflect new version
- bump smoothy
- remove redundant version specifiers from dependencies
- update and fix ci to use cargo test
- migrate clippy lints into Cargo.toml

## [2.2.1](https://github.com/open-schnick/DatadogFormattingLayer/compare/v2.2.0...v2.2.1) - 2024-05-30

### Other
- update README

## [2.2.0](https://github.com/open-schnick/DatadogFormattingLayer/compare/v2.1.0...v2.2.0) - 2024-05-30

### Added
- include fields into nested json object in logs
- use preserve_order serde_json feature flag

### Other
- *(tests)* improve extracting of datadog ids from logs and print logs in test sink
- move IDEA
- simplify parsing and handling of DatadogIds
- extract formatting of log messages and improve tests
- update smoothy
- add ord and eq to FieldPair
- DatadogId to Trace and SpanId for better typing

## [2.1.0](https://github.com/open-schnick/DatadogFormattingLayer/compare/v2.0.0...v2.1.0) - 2024-03-06

### Added
- bump opentelemetry to version 0.22.0

## [2.0.0](https://github.com/open-schnick/DatadogFormattingLayer/compare/v1.1.0...v2.0.0) - 2024-03-05

### Other
- use README as crate doc to also test README snippets
- use cargo nextest instead of cargo test
- move option unwrap into datadog_ids
- add missing unit-tests for layer
- increase readability of layer and field logic
- [**breaking**] introduce custom event sinks to enable testing
- disable certain lints in tests
- introduce nightly formatting options and apply to code
- *(deps)* only specify minor or patch versions of dependencies when needed
- update and split workflows into multiple files
- add important badges
