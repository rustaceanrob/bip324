# Changelog

## [Unreleased]

## [v0.11.0]

* Fix async read hanging indefinitely when connection closes with zero bytes read. [#160](https://github.com/rust-bitcoin/bip324/pull/160)
* [BREAKING] Remove bitcoin dependency in favor of secp256k1 for a smaller dependency tree. Remove `Network` type re-export. [#157](https://github.com/rust-bitcoin/bip324/pull/157)

## [v0.10.0]

Optimizations to the I/O interfaces.

* Use the `Payload` type on the read and write path.
* Take ownership of the garbage and decoy bytes instead of referencing slices.

## [v0.9.0]

The "I regret RPIT" release.

* Updating the `io::Protocol` and `futures::Protocol` interfaces to use newtypes for the session reader transformation instead of RPIT so that the Protocol types can be named by callers.

## [v0.8.0]

Major breaking changes!

* **Handshake API** Introduced compile-time type safety using the typestate pattern. The handshake now uses typed states (`Initialized`, `SentKey`, `ReceivedKey`, `SentVersion`) that prevent out-of-order method calls at compile time.
* **Crate Feature Flags** Removed the `alloc` feature flag, simplifying to `no_std` (core) and `std` (I/O) features. Removed the `futures` feature flag in favor of a single `tokio` feature for async support, reflecting real-world usage patterns.
* **I/O Interfaces** Added new synchronous I/O interface (`io::Protocol`) alongside the existing async interface (moved to `futures::Protocol`). Both interfaces take ownership of underlying I/O readers and writers.
* **Infallible Serialization** The `serialize()` function now returns `Vec<u8>` directly instead of `Result<Vec<u8>, Error>`, eliminating unnecessary error handling for in-memory operations.

### Migration Guide

* Remove error handling from `serialize()` calls.
* Adapt `AsyncProtocol` code to the new `futures::Protocol` interface which owns the underlying reader and writer.
* If using the lower level `handshake` interface, update code to use the new typed state machine.
* If using the lower level `PacketHandler` interface, update to the new `CipherSession` interface.

## [v0.7.0]

* Loosen tokio version restrictions allowing the consumer to dictate the tokio version best for them. The version could effect the MSRV of the library.
* Rename the `async` feature to `futures` to better follow ecosystem conventions.

## [v0.6.0]

* Switch out the chacha20-poly1305 implementation with the SIMD-enabled rust-bitcoin version.
* Expose underlying packet handler types in `AsyncProtocol` so callers can leverage the automatic handshake while maintaining fine grained control of the packet handling.
* Pass along more specific I/O errors to caller.

## [v0.5.0]

* Replace the ownership-based interface of `AsyncProtocol` with mutable references which fit in the asynchronous ecosystem better.
* Add the `tokio` feature flag for easier asynchronous integration if caller is using the Tokio runtime.
* Fix a serialization bug in bitcoin network message.

## v0.4.0

* Adds the `AsyncProtocol` high level interface for less boilerplate integration when using an async runtime (e.g. Tokio). Codes against the `futures-rs` traits, so any runtime which is compatible with those should be supported.
* Aync read functions should now be cancellation safe.
* The high level `Io` variant of the `ProtocolError` exposes if it is worth retrying with the V1 protocol with the new `ProtocolFailureSuggestion` type.

[Unreleased]: https://github.com/rust-bitcoin/bip324/compare/protocol-v0.11.0...HEAD
[v0.11.0]: https://github.com/rust-bitcoin/bip324/compare/protocol-v0.10.0...protocol-v0.11.0
[v0.10.0]: https://github.com/rust-bitcoin/bip324/compare/protocol-v0.9.0...protocol-v0.10.0
[v0.9.0]: https://github.com/rust-bitcoin/bip324/compare/protocol-v0.8.0...protocol-v0.9.0
[v0.8.0]: https://github.com/rust-bitcoin/bip324/compare/protocol-v0.7.0...protocol-v0.8.0
[v0.7.0]: https://github.com/rust-bitcoin/bip324/compare/protocol-v0.6.0...protocol-v0.7.0
[v0.6.0]: https://github.com/rust-bitcoin/bip324/compare/protocol-v0.5.0...protocol-v0.6.0
[v0.5.0]: https://github.com/rust-bitcoin/bip324/compare/protocol-v0.4.0...protocol-v0.5.0
