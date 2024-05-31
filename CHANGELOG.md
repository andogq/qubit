# Changelog

## \[0.4.0]

- [`ea54e2b`](https://github.com/andogq/qubit/commit/ea54e2b76ab11c2dae21eda5dfa7188cfcdb717a) change exported server type to `QubitServer` (close #28)
- [`3f015f9`](https://github.com/andogq/qubit/commit/3f015f95de5776d2d07472f15cada703950e658a) pass all CI checks

### Dependencies

- Upgraded to `qubit-macros@0.3.0`

## \[0.3.0]

### fix

- [`55f4b31`](https://github.com/andogq/qubit/commit/55f4b31bfef67345e94a815c3c38062494bc1327) allow for `to_service` to return a future which produces the context

### feat

- [`be65ee3`](https://github.com/andogq/qubit/commit/be65ee311aea16002d2311694bb2e30958f8f28b) add `HashSet`, `BTreeSet`, and `BTreeMap` to types that implement `ExportType`

## \[0.2.1]

- [`3840c3b`](https://github.com/andogq/qubit/commit/3840c3b0854e59626410b15fb5eb57739fbd1902) automatically dervie `ExportType` for `f32` and `f64`

### Dependencies

- Upgraded to `qubit-macros@0.2.1`

## \[0.2.0]

### Dependencies

- Upgraded to `qubit-macros@0.2.0`

### feat

- [`0758fe3`](https://github.com/andogq/qubit/commit/0758fe32bcf6b702177b88e3dbf7158acaf42523) alter `FromContext` trait to be `async`

## \[0.1.0]

### Dependencies

- Upgraded to `qubit-macros@0.1.0`
- [`a5f8e49`](https://github.com/andogq/qubit/commit/a5f8e49c70a1e82a983f4841482671ec16eab765) update dependencies

### refactor

- [`99c8fd3`](https://github.com/andogq/qubit/commit/99c8fd3d5cfa4e2e662adf72ed7d410aee6bf73c) refactor `TypeDependencies` trait into `ExportType` trait

### feat

- [`2aafe80`](https://github.com/andogq/qubit/commit/2aafe80cc0e3ad74f9182da20e8ea9bb8110fcad) switch over to `TypeRegistry` to export client, and now optionally export `Stream` as required

## \[0.0.10]

### fix

- [`43eb9c4`](https://github.com/andogq/qubit/commit/43eb9c4ff8d1894cfc4256e8cd1d10a112bb6275) Make sure that the subscription and channel are both still active before attempting to send data
  down them.

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## \[Unreleased]

## [0.0.9](https://github.com/andogq/qubit/compare/qubit-v0.0.8...qubit-v0.0.9) - 2024-05-23

### Other

- bump client version
- synchronously return unsubscribe function from client
- improve build script for client lib
- rename proc macro implementation for `TypeDependencies`
- turn `exported_type` into a proc macro
- properly generate `TypeDependencies` trait for built-in generic types

## [0.0.8](https://github.com/andogq/qubit/compare/qubit-v0.0.7...qubit-v0.0.8) - 2024-05-22

### Fixed

- properly handle unit return type from handlers

### Other

- remove whitespace in readme
- add badges to readme

## [0.0.7](https://github.com/andogq/qubit/compare/qubit-v0.0.6...qubit-v0.0.7) - 2024-05-22

### Fixed

- make some sub-modules with documentation public

### Other

- try add github actions
- continue adding documentation and re-factoring
- begin refactoring and moving files into more reasonable layout
