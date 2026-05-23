# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-05-22

### Added

- `--strict` / `--no-strict`, `--jsonl`, `--field`, `--group-by`, `--no-color`, multi-file `--follow`, structured stderr diagnostics, `ExitCode` (grep convention 0/1/2)

### Changed

- `--input=auto` is now evaluated per-line (previously whole-file)

### Fixed

- Parser no longer aborts on the first malformed line (`--strict` opts in to strict behavior)

## [0.1.0] - 2026-05-22

Initial release via nfltr distributed orchestration.
