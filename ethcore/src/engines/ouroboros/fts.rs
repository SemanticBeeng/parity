//! Follow the Satoshi algorithm to produce a list of slots and slot leaders
//! based on how much stake each of the slot leaders has.

use rand::{self, SeedableRng};

use engines::ouroboros::{Coin, SlotLeaders, StakeholderId};
use util::*;

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
pub fn follow_the_satoshi<'a, I>(
    seed: Option<I>,
    genesis_balances: &[(StakeholderId, Coin)],
    epoch_slots: u64,
    total_coins: Coin) -> SlotLeaders
where I: IntoIterator<Item=&'a u8> {

    let seed_bytes: Vec<u8> = match seed {
        Some(seed) => seed.into_iter().map(|&u| u).collect(),
        None => GENESIS_SEED.bytes().into_iter().collect(),
    };
    let seed_slice = as_u32_seed(&seed_bytes);
    println!("fts seed is {:?}", seed_slice);

    let mut rng = rand::ChaChaRng::from_seed(seed_slice);

    assert!(total_coins != Coin::zero(), "Total amount of coin held by the validators is 0!");

    let range = Coin::range(Coin::zero(), total_coins);

    let mut coin_indices: Vec<_> = (0..epoch_slots)
        .map(|i| (i, range.independent_sample(&mut rng)))
        .collect();

    coin_indices.sort_by_key(|&(_, r)| r);

    println!("coin_indices is {:?}", coin_indices);

    let mut max_coins = Coin::zero();
    let mut ci = coin_indices.iter().peekable();
    let mut slot_leaders = Vec::with_capacity(epoch_slots as usize);

    for &(stakeholder, coins) in genesis_balances {
        max_coins = max_coins + coins;

        while let Some(&&(slot, coin)) = ci.peek() {
            if coin < max_coins {
                slot_leaders.push((slot, stakeholder.clone()));
                ci.next();
            } else {
                break;
            }
        }
    }

    slot_leaders.sort_by_key(|&(i, _)| i);

    slot_leaders.into_iter().map(|(_, v)| v).collect()
}

// The ChaChaRng::from_seed implementation
// (https://docs.rs/rand/0.3.15/rand/chacha/struct.ChaChaRng.html)
// takes a slice of u32s and only uses up to 8 words.
//
// This function takes the first 8*4=32 u8 values in a slice of u8s
// and turns them into a slice of 8 u32s.
fn as_u32_seed(u8s: &[u8]) -> &[u32] {
    assert!(u8s.len() >= 32);
    let first_32 = &u8s[..32];
    assert!(first_32.len() == 32);
    unsafe {
        ::std::slice::from_raw_parts(
            first_32.as_ptr() as *const u32,
            8
        )
    }
}

#[cfg(test)]
mod tests {
    use super::follow_the_satoshi;
    use engines::ouroboros::Coin; //, SlotLeaders, StakeholderId};
	use util::*;

    #[test]
    fn one_stakeholder_is_always_the_leader() {
        let address = Address::from_str("0000000000000000000000000000000000000005").unwrap();
        let balances = vec![
            (address.clone(), Coin::from(10))
        ];
        let seed: Option<&[u8]> = None;

        let result = follow_the_satoshi(seed, &balances, 3, Coin::from(10));
        assert_eq!(result, vec![address.clone(), address.clone(), address.clone()]);
    }

    #[test]
    fn two_stakeholders_equal_stake() {
        let aaa = Address::from_str("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
        let bbb = Address::from_str("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap();
        let balances = vec![
            (aaa.clone(), Coin::from(50)),
            (bbb.clone(), Coin::from(50)),
        ];
        let seed: Option<&[u8]> = None;

        let result = follow_the_satoshi(seed, &balances, 10, Coin::from(100));
        assert_eq!(result, [
            aaa.clone(), aaa.clone(), aaa.clone(),
            bbb.clone(),
            aaa.clone(), aaa.clone(), aaa.clone(),
            bbb.clone(), bbb.clone(), bbb.clone()]);
    }

    #[test]
    fn two_stakeholders_skewed_stake() {
        let aaa = Address::from_str("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
        let bbb = Address::from_str("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap();
        let balances = vec![
            (aaa.clone(), Coin::from(80)),
            (bbb.clone(), Coin::from(20)),
        ];
        let seed: Option<&[u8]> = None;

        let result = follow_the_satoshi(seed, &balances, 10, Coin::from(100));
        assert_eq!(result, [
            aaa.clone(), aaa.clone(), aaa.clone(),
            bbb.clone(),
            aaa.clone(), aaa.clone(), aaa.clone(),
            bbb.clone(),
            aaa.clone(), aaa.clone()]);
    }
}
