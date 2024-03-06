# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
