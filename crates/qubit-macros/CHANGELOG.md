# Changelog

## \[0.6.2]

### feat

- [`e426945`](https://github.com/andogq/qubit/commit/e426945cda8cacd9a33c7cc8705945324dc5c305) allow for `query` handlers to be accessed via `GET` as well as `POST`

## \[0.6.1]

### fix

- [`dbf8fd5`](https://github.com/andogq/qubit/commit/dbf8fd5ee5745f070be7842a68d8fb6e8eb70cdf) update readme with correct instructions

## \[0.6.0]

### feat

- [`7274cb0`](https://github.com/andogq/qubit/commit/7274cb059af6ab1d00d92099fab2a7ee8ea2b6be) **BREAKING** replace `FromContext` with `FromRequestExtensions` to build ctx from request information (via tower middleware)

## \[0.5.1]

### fix

- [`bf93414`](https://github.com/andogq/qubit/commit/bf93414c1e2732d5e0ae5c13425529038303a935) use absolute paths in the macro

## \[0.5.0]

### feat

- [`429c19f`](https://github.com/andogq/qubit/commit/429c19f3506bdd225b2c2762907d8c880a07bbca) require handlers to have attribute of `query`, `mutation`, or `subscription`

## \[0.4.1]

### feat

- [`f227cc9`](https://github.com/andogq/qubit/commit/f227cc96e6170cb039905fe0b55b5585ca5b81ee) handlers with no parameters no longer need to take `ctx`

## \[0.4.0]

### feat

- [`625df36`](https://github.com/andogq/qubit/commit/625df3640b3a1134866040de56a1e29943c15e76) remove `ExportType` macro, to now only rely on `ts-rs::TS` (close #26)

## \[0.3.0]

- [`3f015f9`](https://github.com/andogq/qubit/commit/3f015f95de5776d2d07472f15cada703950e658a) pass all CI checks

## \[0.2.1]

- [`d2bf039`](https://github.com/andogq/qubit/commit/d2bf03992c9ea1b160497e371882b51377f4c2ec) implement `ExportType` derive for enums (close #20)

## \[0.2.0]

### feat

- [`0758fe3`](https://github.com/andogq/qubit/commit/0758fe32bcf6b702177b88e3dbf7158acaf42523) alter `FromContext` trait to be `async`

## \[0.1.0]

### feat

- [`ff7bf89`](https://github.com/andogq/qubit/commit/ff7bf89cb2b419aba7fd8fd98685abaccd407753) specify custom names for handlers using `#[handler(name = "my_handler")]`
- [`2aafe80`](https://github.com/andogq/qubit/commit/2aafe80cc0e3ad74f9182da20e8ea9bb8110fcad) switch over to `TypeRegistry` to export client, and now optionally export `Stream` as required

### refactor

- [`d6ccc9a`](https://github.com/andogq/qubit/commit/d6ccc9a4431656df2dc35d1d1326a8b4358a7c4b) Refactor macros
- [`99c8fd3`](https://github.com/andogq/qubit/commit/99c8fd3d5cfa4e2e662adf72ed7d410aee6bf73c) refactor `TypeDependencies` trait into `ExportType` trait

### fix

- [`b399c8b`](https://github.com/andogq/qubit/commit/b399c8bfa38f8c82a819668b4139b936905263c8) respect visibilitly modifier on handler function when macro-ing

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## \[Unreleased]

## [0.0.7](https://github.com/andogq/qubit/compare/qubit-macros-v0.0.6...qubit-macros-v0.0.7) - 2024-05-23

### Other

- rename proc macro implementation for `TypeDependencies`
- turn `exported_type` into a proc macro

## [0.0.6](https://github.com/andogq/qubit/compare/qubit-macros-v0.0.5...qubit-macros-v0.0.6) - 2024-05-22

### Fixed

- properly handle unit return type from handlers

## [0.0.5](https://github.com/andogq/qubit/compare/qubit-macros-v0.0.4...qubit-macros-v0.0.5) - 2024-05-22

### Other

- continue adding documentation and re-factoring
