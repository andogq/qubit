# Changelog

## \[0.4.5]

- [`b6ef950`](https://github.com/andogq/qubit/commit/b6ef95077345cb4db4143e364c48eef010f41fc8) Bump dependencies

## \[0.4.4]

### bug

- [`ba75bd4`](https://github.com/andogq/qubit/commit/ba75bd43fab2b2421fcf27694fcf9deca59860ea) Fix memory leak in promise manager due to holding references to resolved promises.

## \[0.4.3]

### feat

- [`e72078a`](https://github.com/andogq/qubit/commit/e72078a340b4f61703770036327b39a6abeedd5d) throw error value from RPC if encountered

## \[0.4.2]

### feat

- [`0b7457a`](https://github.com/andogq/qubit/commit/0b7457ab5f2647892880fdb3d45ee4f2a9d3adfc) create multi tranasport
- [`2591127`](https://github.com/andogq/qubit/commit/2591127f0cfb78b1917bac317552099475d1fc72) create svelte integration

## \[0.4.1]

### feat

- [`886106b`](https://github.com/andogq/qubit/commit/886106b27b68fb1e2a24f7cd0f3a2e929032151b) support `GET` for queries in client

## \[0.4.0]

### feat

- [`e17bbf0`](https://github.com/andogq/qubit/commit/e17bbf0fb8adce5f488247f298278342add2e478) refactor client to introduct plugins, simplify types, and prepare for future work

## \[0.3.3]

### feat

- [`f8ff07b`](https://github.com/andogq/qubit/commit/f8ff07b8d3b92aef60687b868a04ff08f4a8de2f) feat: allow for polyfilling `fetch` for `http` transport (close #55)

## \[0.3.2]

### fix

- [`61f46c3`](https://github.com/andogq/qubit/commit/61f46c3ad82f4b869579d896697c3c4312154ac2) remove test import

## \[0.3.1]

### fix

- [`8fca0ce`](https://github.com/andogq/qubit/commit/8fca0ceee34786f28c17f5e979dad7f4125d517a) remove old client builder

## \[0.3.0]

### feat

- [`200efef`](https://github.com/andogq/qubit/commit/200efef21d10ed674afb27c336b6a9e2d02f58ad) output a Qubit logo header to binding files
- [`45510bf`](https://github.com/andogq/qubit/commit/45510bfc270c076012f6179a2567ae9c6c9fbff4) change handler syntax within the client

## \[0.2.1]

- [`39fb781`](https://github.com/andogq/qubit/commit/39fb781d89b47b97780cc8683976027a5f127dc7) update package.json with repo and keywords

### feat

- [`223833d`](https://github.com/andogq/qubit/commit/223833d94baf47ac6200bd9db44a7a39af102019) implement reconnecting web socket in client

## \[0.2.0]

- [`032d01e`](https://github.com/andogq/qubit/commit/032d01ef832b437d21b04e9d422204d216fc0397) run `pnpm build` before publishing (close #29)

## \[0.1.0]

### feat

- [`46ea1a9`](https://github.com/andogq/qubit/commit/46ea1a97483357a031ce5229e31d7de3c690e16a) allow for subscription method to be overloaded if only `on_data` is required.
