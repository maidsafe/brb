# BRB: Byzantine Reliable Broadcast

[MaidSafe website](http://maidsafe.net) | [Safe Network Forum](https://safenetforum.org/)
:-------------------------------------: | :---------------------------------------------:

## About

This crate provides a deterministic implementation of Byzantine Reliable Broadcast (BRB).

This crate and its related crates (`brb_*`) implement a loosely-coupled Byzantine Fault Tolerant (BFT) system for achieving network agreement over eventually consistent algorithms.

Each `brb_algo_*` crate provides a particular data type with its own operations that are wrapped in such a way that it can be transferred via BRB.  At present CRDT data types are well suited for this.  We intend to wrap many such data types,with each wrapper in its own crate.

## BRB Crates

As of this initial writing, the crates are:

|crate|description|
|-----|-----------|
|brb   |this crate. provides brb implementation and SecureBroadcastAlgo trait that brb_algo_* crates implement|
|[brb_membership](https://github.com/dan-da/brb_membership)|brb dynamic membership: support for peers joining and leaving a brb group|
|[brb_algo_at2](https://github.com/maidsafe/brb_algo_at2)|The [AT2 algorithm](https://arxiv.org/pdf/1812.10844.pdf) in a brb wrapper|
|[brb_algo_orswot](https://github.com/maidsafe/brb_algo_orswot)|an brb wrapper for the Orswot CRDT algorithm in [rust-crdt](https://github.com/rust-crdt/rust-crdt/)|


## Traits

trait | description
----- | -----------
|[SecureBroadcastAlgorithm](src/secure_broadcast_algorithm.rs)| Data types to be secured should impl this|

## Prior Work

This crate and its sibling have been broken out of the original [bft-crdts](https://github.com/davidrusu/bft-crdts/) crate.  Additional documentation and source code can be found there.


## License

This Safe Network software is dual-licensed under the Modified BSD (<LICENSE-BSD> <https://opensource.org/licenses/BSD-3-Clause>) or the MIT license (<LICENSE-MIT> <https://opensource.org/licenses/MIT>) at your option.

## Contributing

Want to contribute? Great :tada:

There are many ways to give back to the project, whether it be writing new code, fixing bugs, or just reporting errors. All forms of contributions are encouraged!

For instructions on how to contribute, see our [Guide to contributing](https://github.com/maidsafe/QA/blob/master/CONTRIBUTING.md).
