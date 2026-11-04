# BIP-324 Encrypted Transport Protocol

[BIP-324](https://github.com/bitcoin/bips/blob/master/bip-0324.mediawiki) describes the V2 encrypted communication protocol for the bitcoin P2P network. This workspace publishes two crates.

* [`bip324`](./protocol): The core library which supports the BIP-324 handshake and cipher session with sans-I/O core, plus sync and async I/O wrappers.
* [`bip324-traffic`](./traffic): Traffic shape hiding layer over `bip324`, using decoy packets and padding to obscure communication patterns.

## Contributing

We have a dedicated developer channel on IRC, #bitcoin-rust@libera.chat where you may get helpful advice if you have questions.
