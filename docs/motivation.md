# Motivation

Bitcoin's original P2P protocol, "V1", was designed without any encryption. Even
though the data exchanged over the bitcoin P2P network is public to some degree,
encrypted communications offers a number of benefits over plaintext communication.

* **Censorship resistance.** Internet Service Providers (ISPs) can easily detect
  and censor plaintext bitcoin communication. Encryption prevents deep packet
  inspection from identifying bitcoin traffic based on its wire format.
* **Integrity.** Plaintext message tampering, without detection, is trivial for a
  man-in-the-middle (MitM) attacker. The AEAD cipher used in BIP-324 provides
  authenticated encryption so any in-flight modification is immediately detectable.
* **Metadata privacy.** Nefarious actors may associate metadata, such as IP
  addresses and transaction origins, without explicitly having to connect
  directly to peers. Encryption raises the bar for passive network surveillance.
* **Forward secrecy.** Each BIP-324 session negotiates fresh ephemeral keys via
  an Elligator Swift-encoded ECDH handshake. Compromise of a long-term identity
  key does not decrypt past sessions.

BIP-324 — "V2" encrypted communication — protects against the above issues,
increasing the privacy and censorship-resistance of the bitcoin ecosystem. Any
application communicating with bitcoin nodes, including light clients, should
make use of the V2 protocol.

## Further Reading

* [BIP-324 specification](https://github.com/bitcoin/bips/blob/master/bip-0324.mediawiki)
* [Elligator Swift paper](https://eprint.iacr.org/2022/759.pdf)
