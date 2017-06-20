//! Follow the Satoshi algorithm to produce a list of slots and slot leaders
//! based on how much stake each of the slot leaders has.

use rand::{self, Rng};

use std::collections::HashMap;

use util::*;

// Type aliases to match cardano types
type Coin = u64;
type StakeholderId = Address;
type SlotLeaders<'a> = Vec<&'a StakeholderId>;

/// Genesis seed that will be used for the 0th epoch. We must hardcode a seed
/// because we need to somehow determine leaders for the first ever epoch
/// (stakes are hardcoded as well so we can run FTS on those stakes using this
/// seed). Value from https://github.com/input-output-hk/cardano-sl/blob/8fc5b34e238de6f3510da8e5956b5f55b93715e4/lrc/Pos/Lrc/Genesis.hs#L12
pub const GENESIS_SEED: &str = "vasa opasa skovoroda Ggurda boroda provoda";

/// Implementation of the algorithm described in [cardano-sl](https://github.com/input-output-hk/cardano-sl/blob/1f866450a8a530c119e3fc9edb84c97c56417aa2/lrc/Pos/Lrc/Fts.hs).
/// Takes a sorted list of stakeholders by address with their stake, and returns
/// a list of slots with the leader for that slot. Leaders are chosen randomly
/// weighted by the amount of stake they have.
// TODO: take a random number generator instead of a seed:
// <R>, mut rng: R, where R: rand::Rng
pub fn follow_the_satoshi<'a>(
    seed: &str,
    genesis_balances: &[(&'a StakeholderId, Coin)],
    epoch_slots: u64, total_coins: u64) -> SlotLeaders<'a> {

    let mut rng = rand::thread_rng();

    let mut coin_indices: Vec<_> = (0..epoch_slots)
        .map(|i| (i, rng.gen_range(0, total_coins)))
        .collect();

    coin_indices.sort_by_key(|&(_, r)| r);

    let mut max_coins = 0;
    let mut ci = coin_indices.iter().peekable();
    let mut slot_leaders = Vec::with_capacity(epoch_slots as usize);

    for &(stakeholder, coins) in genesis_balances {
        max_coins += coins;

        while let Some(&&(slot, coin)) = ci.peek() {
            if coin < max_coins {
                slot_leaders.push((slot, stakeholder));
                ci.next();
            } else {
                break;
            }
        }
    }

    slot_leaders.sort_by_key(|&(i, _)| i);

    slot_leaders.into_iter().map(|(_, v)| v).collect()
}

#[cfg(test)]
mod tests {
    use super::{follow_the_satoshi, GENESIS_SEED};
	use util::*;

    #[test]
    fn one_stakeholder_is_always_the_leader() {
        let address = Address::from_str("0000000000000000000000000000000000000005").unwrap();
        let balances = vec![
            (&address, 10)
        ];
        let result = follow_the_satoshi(GENESIS_SEED, &balances, 3, 10);
        assert_eq!(result, vec![&address, &address, &address]);
    }

    #[test]
    fn two_stakeholders_equal_stake() {
        let aaa = Address::from_str("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
        let bbb = Address::from_str("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap();
        let balances = vec![(&aaa, 50), (&bbb, 50)];

        let result = follow_the_satoshi(GENESIS_SEED, &balances, 1_000, 100);
    }

    #[test]
    fn two_stakeholders_skewed_stake() {
        let aaa = Address::from_str("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
        let bbb = Address::from_str("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap();
        let balances = vec![(&aaa, 99), (&bbb, 1)];

        let result = follow_the_satoshi(GENESIS_SEED, &balances, 1_000, 100);
    }
}
