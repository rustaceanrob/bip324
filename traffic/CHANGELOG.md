# Changelog

## [Unreleased]

* Upgrade MSRV to 1.74.0, inherited from the `bip324` protocol crate.
* Upgrade `rand` dependency to 0.9.

## v0.1.0

Initial release.

* The heart of the API is the sans-io `TrafficShaper`.
* I/O driving wrappers are also exposed `io::ShapedProtocol` and `futures::ShapedProtocol`.
