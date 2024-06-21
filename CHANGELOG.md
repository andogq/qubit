# Changelog

## \[0.9.4]

### feat

- [`886106b`](https://github.com/andogq/qubit/commit/886106b27b68fb1e2a24f7cd0f3a2e929032151b) support `GET` for queries in client
- [`3f6b12b`](https://github.com/andogq/qubit/commit/3f6b12ba8c088fc266b49ad51fb9d15acf223503) include `// @ts-nocheck` at top of generated files

## \[0.9.3]

### Dependencies

- Upgraded to `qubit-macros@0.6.3`

### feat

- [`e17bbf0`](https://github.com/andogq/qubit/commit/e17bbf0fb8adce5f488247f298278342add2e478) refactor client to introduct plugins, simplify types, and prepare for future work

## \[0.9.2]

### Dependencies

- Upgraded to `qubit-macros@0.6.2`

### feat

- [`e426945`](https://github.com/andogq/qubit/commit/e426945cda8cacd9a33c7cc8705945324dc5c305) allow for `query` handlers to be accessed via `GET` as well as `POST`

## \[0.9.1]

### Dependencies

- Upgraded to `qubit-macros@0.6.1`

### fix

- [`dbf8fd5`](https://github.com/andogq/qubit/commit/dbf8fd5ee5745f070be7842a68d8fb6e8eb70cdf) update readme with correct instructions

## \[0.9.0]

### Dependencies

- Upgraded to `qubit-macros@0.6.0`

### feat

- [`9543d12`](https://github.com/andogq/qubit/commit/9543d126a915d5501a83ba207591858283cebe87) (**BREAKING**) pass single cloneable ctx to builder instead of closure that accepts a request
- [`7274cb0`](https://github.com/andogq/qubit/commit/7274cb059af6ab1d00d92099fab2a7ee8ea2b6be) **BREAKING** replace `FromContext` with `FromRequestExtensions` to build ctx from request information (via tower middleware)

### fix

- [`111db0a`](https://github.com/andogq/qubit/commit/111db0a3fb52c221749f12aeda5757df847df5a8) fix incorrect handling of deeply nested routers

## \[0.8.0]

- [`cb95f67`](https://github.com/andogq/qubit/commit/cb95f67c1457458a7123814d872bcdc7bdb1fba9) fix example dependency versions in README

### feat

- [`64913a8`](https://github.com/andogq/qubit/commit/64913a884e82ee35e6b63ded86755582a8031360) provide mutable reference to request parts, instead of the entire request to the context builder.

## \[0.7.0]

- [`69669f4`](https://github.com/andogq/qubit/commit/69669f4dbb99cc179479ca6a5b2c33c0639b8531) update to jsonrpsee 0.23.0

### Dependencies

- Upgraded to `qubit-macros@0.5.1`

### fix

- [`fe5fd40`](https://github.com/andogq/qubit/commit/fe5fd4049510e7b9847da7518ae7ea01abd1bde6) bring README back up to date

## \[0.6.1]

### Dependencies

- Upgraded to `qubit-macros@0.5.0`

## \[0.6.0]

### feat

- [`57e124f`](https://github.com/andogq/qubit/commit/57e124faf3fc4f7af0e5b25f5ac18f982e1d820a) add `on_close` callback to `to_service`, which will be run when the client connection closes (close #44)

## \[0.5.2]

### fix

- [`0bb7ac9`](https://github.com/andogq/qubit/commit/0bb7ac934730cca49acc3785074c65a356b5ebe5) ([#41](https://github.com/andogq/qubit/pull/41)) fix exporting tuple types returned from handlers (close #41)

## \[0.5.1]

### Dependencies

- Upgraded to `qubit-macros@0.4.1`

### minor

- [`a57ec51`](https://github.com/andogq/qubit/commit/a57ec51e05b8b4dc509a401f1a17dee1d3f45b5e) update crate description to match repository

## \[0.5.0]

### Dependencies

- Upgraded to `qubit-macros@0.4.0`

### feat

- [`625df36`](https://github.com/andogq/qubit/commit/625df3640b3a1134866040de56a1e29943c15e76) remove `ExportType` macro, to now only rely on `ts-rs::TS` (close #26)

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
