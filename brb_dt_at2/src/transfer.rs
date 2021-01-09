use brb::Actor;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::Money;

// TODO: introduce decomp. of Account from Actor
// pub type Account = Actor; // In the paper, Actor and Account are synonymous

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Transfer {
    pub(crate) from: Actor,
    pub(crate) to: Actor,
    pub(crate) amount: Money,

    /// set of transactions that need to be applied before this transfer can be validated
    /// ie. a proof of funds
    pub(crate) deps: BTreeSet<Transfer>,
}
