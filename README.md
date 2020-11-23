# brb

[MaidSafe website](http://maidsafe.net) | [Safe Network Forum](https://safenetforum.org/)
:-------------------------------------: | :---------------------------------------------:

## About

This crate provides traits that are implemented in the `sb_algo_*`, `sb_impl_*`, and `sb_net_*` crates.

This crate and its related crates (`sb_*`) implement a loosely-coupled Byzantine Fault Tolerant (BFT) system for achieving network agreement over eventually consistent algorithms.

Each `sb_algo_*` crate provides a particular data type with its own operations that are wrapped in such a way that it can be transferred via Secure Broadcast.  At present CRDT data types are well suited for this.  We intend to wrap many such data types, with each wrapper in its own crate.

Each `sb_impl_*` crate provides a particular implementation of Secure Broadcast.  For now, the only implementation is Deterministic Secure Broadcast (DSB).  However, it is envisioned that Probablistic Secure
Broadcast (PSB) as described in the AT2 research paper may be implemented in the future.  Other BFT
algorithms could also be implemented.

Each `sb_net_*` crate provides a network transport implementation.  For now, we are working on a
Quic ([qp2p](https://github.com/maidsafe/qp2p)) implementation as well as an in-memory network for test cases.  In the future, we'd like to
also include a TCP/IP sockets implementation.  pull-requests welcome!

The loosely coupled nature of these crates make it easy** to pick any combination of:  (a) secure broadcast mechanism, (b) data type that is being secured, and (c) network transport layer.

## SB Crates

As of this initial writing, the crates are:

|crate|description|
|-----|-----------|
|sb   |this crate. provides traits that other crates implement and depend on|
|[sb_algo_at2](https://github.com/maidsafe/sb_algo_at2)|The [AT2 algorithm](https://arxiv.org/pdf/1812.10844.pdf) in an sb wrapper|
|[sb_algo_orswot](https://github.com/maidsafe/sb_algo_orswot)|an sb wrapper for the Orswot CRDT algorithm in [rust-crdt](https://github.com/rust-crdt/rust-crdt/)|
|[sb_impl_dsb](https://github.com/maidsafe/sb_impl_dsb)|sb implementation: deterministic secure broadcast|
|[sb_net_mem](https://github.com/dan-da/sb_net_mem)|sb network layer:  in memory network simulator|

## Traits

trait | description
----- | -----------
|[SecureBroadcastAlgorithm](src/secure_broadcast_algorithm.rs)| Data types to be secured should impl this|
|[SecureBroadcastProc](src/secure_broadcast_impl.rs)     | DSB implementations should impl this|
|[SecureBroadcastNetwork](src/secure_broadcast_network.rs)  | Network transports should impl this |
|[SecureBroadcastNetworkSimulator](src/secure_broadcast_network.rs) | Network Simulators should impl this, for use in tests |

## Prior Work

This crate and its sibling have been broken out of the original [bft-crdts](https://github.com/davidrusu/bft-crdts/) crate.  Additional documentation and source code can be found there.


## License

This Safe Network software is dual-licensed under the Modified BSD (<LICENSE-BSD> <https://opensource.org/licenses/BSD-3-Clause>) or the MIT license (<LICENSE-MIT> <https://opensource.org/licenses/MIT>) at your option.

## Contributing

Want to contribute? Great :tada:

There are many ways to give back to the project, whether it be writing new code, fixing bugs, or just reporting errors. All forms of contributions are encouraged!

For instructions on how to contribute, see our [Guide to contributing](https://github.com/maidsafe/QA/blob/master/CONTRIBUTING.md).
