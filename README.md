# BRB: Byzantine Reliable Broadcast

[MaidSafe website](http://maidsafe.net) | [Safe Network Forum](https://safenetforum.org/)
:-------------------------------------: | :---------------------------------------------:

## About

This crate provides traits that are implemented in the `brb_algo_*`, `brb_impl_*`, and `brb_net_*` crates.

This crate and its related crates (`brb_*`) implement a loosely-coupled Byzantine Fault Tolerant (BFT) system for achieving network agreement over eventually consistent algorithms.

Each `brb_algo_*` crate provides a particular data type with its own operations that are wrapped in such a way that it can be transferred via Secure Broadcast.  At present CRDT data types are well suited for this.  We intend to wrap many such data types, with each wrapper in its own crate.

Each `brb_impl_*` crate provides a particular implementation of Byzantine Reliable Broadcast.  For now, the only implementation is Deterministic Secure Broadcast (DSB).  However, it is envisioned that Probablistic Secure
Broadcast (PSB) as described in the AT2 research paper may be implemented in the future.  Other BFT algorithms could also be implemented.

Each `brb_net_*` crate provides a network transport implementation.  For now, we are working on a
Quic ([qp2p](https://github.com/maidsafe/qp2p)) implementation as well as an in-memory network for test cases.  In the future, we'd like to
also include a TCP/IP sockets implementation.  pull-requests welcome!

The loosely coupled nature of these crates make it easy** to pick any combination of:  (a) secure broadcast mechanism, (b) data type that is being secured, and (c) network transport layer.

## BRB Crates

As of this initial writing, the crates are:

|crate|description|
|-----|-----------|
|brb   |this crate. provides traits that other crates implement and depend on|
|[brb_algo_at2](https://github.com/maidsafe/brb_algo_at2)|The [AT2 algorithm](https://arxiv.org/pdf/1812.10844.pdf) in a brb wrapper|
|[brb_algo_orswot](https://github.com/maidsafe/brb_algo_orswot)|an brb wrapper for the Orswot CRDT algorithm in [rust-crdt](https://github.com/rust-crdt/rust-crdt/)|
|[brb_impl_dsb](https://github.com/maidsafe/brb_impl_dsb)|brb implementation: deterministic secure broadcast|
|[brb_net_mem](https://github.com/dan-da/brb_net_mem)|brb network layer:  in memory network simulator|
|[brb_net_qp2p](https://github.com/dan-da/brb_net_qp2p)|brb network layer: qp2p network delivery|


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
