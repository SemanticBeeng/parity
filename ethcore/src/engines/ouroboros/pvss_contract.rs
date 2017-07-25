use bincode::{serialize, deserialize, Infinite};

use std::sync::Weak;
use client::{Client, BlockChainClient};
use util::*;
// TODO: cache
// use util::cache::MemoryLruCache;
use pvss;

#[derive(Deserialize, PartialEq)]
pub struct PvssCommitInfo {
    pub commitments: Vec<pvss::simple::Commitment>,
    pub shares: Vec<pvss::simple::EncryptedShare>,
}

#[derive(Serialize, Deserialize, PartialEq)]
struct PvssRevealInfo {
    secret: pvss::simple::Secret,
}

unsafe impl Send for PvssCommitInfo {}
unsafe impl Sync for PvssCommitInfo {}

unsafe impl Send for PvssRevealInfo {}
unsafe impl Sync for PvssRevealInfo {}

// TODO: cache
// struct PvssInfo {
//     commit_info: HashMap<Address, PvssCommitInfo>,
//     reveal_info: HashMap<Address, PvssRevealInfo>,
// }
//
// impl HeapSizeOf for PvssInfo {
//     // TODO: is this correct? Vec?
//     fn heap_size_of_children(&self) -> usize { 0 }
// }

// TODO: cache
// const MEMOIZE_CAPACITY: usize = 500;

pub struct PvssContract {
	pub address: Address,
    // TODO: cache
    // by_epoch: RwLock<MemoryLruCache<usize, PvssInfo>>,
	read_provider: RwLock<Option<provider::Contract>>,
    write_provider: RwLock<Option<provider::Contract>>,
}

impl PvssContract {
	pub fn new() -> Self {
		PvssContract {
			address: Address::from_str("0000000000000000000000000000000000000005").unwrap(),
            // TODO: cache
            // by_epoch: RwLock::new(MemoryLruCache::new(MEMOIZE_CAPACITY)),
			read_provider: RwLock::new(None),
            write_provider: RwLock::new(None),
		}
	}

	pub fn register_contract(&self, client: Weak<Client>) {
        let client1 = client.clone();
	    *self.read_provider.write() = Some(provider::Contract::new(self.address, move |a, d| {
            client1
			    .upgrade()
			    .ok_or("No client!".into())
			    .and_then(|c| {
                    c.call_contract(::client::BlockId::Latest, a, d)
                        .map_err(|e| format!("Transaction call error: {}", e))
                })
	    }));

    	*self.write_provider.write() = Some(provider::Contract::new(self.address, move |a, d| {
            client
			    .upgrade()
			    .ok_or("No client!".into())
			    .and_then(|c| {
                    c.transact_contract(a, d)
                        .map_err(|e| format!("Transaction call error: {}", e))
                        .map(|_| Default::default())
                })
	    }));
    }

	pub fn broadcast_commitments_and_shares(&self, epoch_number: usize, _commitments: &[pvss::simple::Commitment], _shares: &[pvss::simple::EncryptedShare]) {
        println!("in broadcast");
		if let Some(ref provider) = *self.write_provider.read() {

			match provider.save_commitment(epoch_number as u64, 3) {
				Ok(_) => println!("a-ok"),
				Err(s) => warn!(target: "engine", "Could not broadcast commitments and shares: {}", s),
			}
		} else {
			warn!(target: "engine", "Could not broadcast commitments and shares: no provider contract.")
		}
	}
        //
        //     pub fn get_commitments_and_shares(&self, epoch_number: usize, address: &Address) -> Option<PvssCommitInfo> {
        // if let Some(ref provider) = *self.provider.read() {
        //             match provider.get_commitments_and_shares(epoch_number as u64, address) {
        //                 Ok((commitment_bytes, share_bytes)) => {
        //                     let commitments: Vec<pvss::simple::Commitment> = deserialize(&commitment_bytes).expect("Could not deserialize commitments");
        //                     let shares: Vec<pvss::simple::EncryptedShare> = deserialize(&share_bytes).expect("Could not deserialize shares");
        //                     Some(PvssCommitInfo {
        //                         commitments, shares
        //                     })
        //                 },
        //         Err(s) => {
        //                     println!("Could not get commitments and shares: {}", s);
        //                     None
        //                 },
        //     }
        // } else {
        //     warn!(target: "engine", "Could not get commitments and shares: no provider contract.");
        //             None
        // }
        //     }


        pub fn get_commitment(&self, epoch_number: usize, address: &Address) -> Option<u64> {
    		if let Some(ref provider) = *self.read_provider.read() {
                let x = provider.get_commitment(42, address).expect("42 is there");
                println!("hack_answer: {:?}", x);

                match provider.get_commitment(epoch_number as u64, address) {
                    Ok(commitment) => {
                        Some(commitment)
                    },
    				Err(s) => {
                        println!("Could not get commitment: {}", s);
                        None
                    },
    			}
    		} else {
    			warn!(target: "engine", "Could not get commitment: no provider contract.");
                None
    		}
        }
}

mod provider {
    #![allow(unused_imports)]
    use std::string::String;
    use std::result::Result;
    use std::fmt;
    use {util, ethabi};
    use util::{FixedHash, Uint};

    pub struct Contract {
    	contract: ethabi::Contract,
    	pub address: util::Address,
    	do_call: Box<Fn(util::Address, Vec<u8>) -> Result<Vec<u8>, String> + Send + Sync + 'static>,
    }
    impl Contract {
    	pub fn new<F>(address: util::Address, do_call: F) -> Self
    		where F: Fn(util::Address, Vec<u8>) -> Result<Vec<u8>, String> + Send + Sync + 'static {
    		Contract {
    			contract: ethabi::Contract::new(ethabi::Interface::load(b"[{\"constant\":false,\"inputs\":[{\"name\":\"epochIndex\",\"type\":\"uint64\"},{\"name\":\"commitment\",\"type\":\"uint64\"}],\"name\":\"saveCommitment\",\"outputs\":[],\"payable\":false,\"type\":\"function\"},{\"constant\":false,\"inputs\":[{\"name\":\"epochIndex\",\"type\":\"uint64\"},{\"name\":\"sender\",\"type\":\"address\"}],\"name\":\"getCommitment\",\"outputs\":[{\"name\":\"\",\"type\":\"uint64\"}],\"payable\":false,\"type\":\"function\"}]").expect("JSON is autogenerated; qed")),
    			address: address,
    			do_call: Box::new(do_call),
    		}
    	}
    	fn as_string<T: fmt::Debug>(e: T) -> String { format!("{:?}", e) }

    	/// Auto-generated from: `{"constant":false,"inputs":[{"name":"epochIndex","type":"uint64"},{"name":"commitment","type":"uint64"}],"name":"saveCommitment","outputs":[],"payable":false,"type":"function"}`
    	#[allow(dead_code)]
    	pub fn save_commitment(&self, epoch_index: u64, commitment: u64) -> Result<(), String>
    		 {

    		let call = self.contract.function("saveCommitment".into()).map_err(Self::as_string)?;
    		let data = call.encode_call(
    			vec![ethabi::Token::Uint({ let mut r = [0u8; 32]; util::U256::from(epoch_index as u64).to_big_endian(&mut r); r }), ethabi::Token::Uint({ let mut r = [0u8; 32]; util::U256::from(commitment as u64).to_big_endian(&mut r); r })]
    		).map_err(Self::as_string)?;

                 let xxx = (self.do_call)(self.address.clone(), data.clone());
                 println!(
                     "saveCommitment: address = {}; epoch_index = {}; commitment = {}; data = {:?} => {:?}",
                     self.address,
                     epoch_index,
                     commitment,
                     data,
                     xxx
                 );

    		call.decode_output(xxx?).map_err(Self::as_string)?;

    		Ok(())
    	}

    	/// Auto-generated from: `{"constant":false,"inputs":[{"name":"epochIndex","type":"uint64"},{"name":"sender","type":"address"}],"name":"getCommitment","outputs":[{"name":"","type":"uint64"}],"payable":false,"type":"function"}`
    	#[allow(dead_code)]
    	pub fn get_commitment(&self, epoch_index: u64, sender: &util::Address) -> Result<u64, String>
    		 {

    		let call = self.contract.function("getCommitment".into()).map_err(Self::as_string)?;
    		let data = call.encode_call(
    			vec![ethabi::Token::Uint({ let mut r = [0u8; 32]; util::U256::from(epoch_index as u64).to_big_endian(&mut r); r }), ethabi::Token::Address(sender.clone().0)]
    		).map_err(Self::as_string)?;

            let arg = (self.do_call)(self.address.clone(), data.clone());
                 println!(
                     "getCommitment: address = {}; epoch_index = {}; data = {:?} => {:?}",
                     self.address,
                     epoch_index,
                     data,
                     arg
                 );

    		let output = call.decode_output(arg?).map_err(Self::as_string)?;
    		let mut result = output.into_iter().rev().collect::<Vec<_>>();
    		Ok(({ let r = result.pop().ok_or("Invalid return arity")?; let r = r.to_uint().ok_or("Invalid type returned")?; util::U256::from(r.as_ref()).as_u64() as u64 }))
    	}
    }

}


// mod provider {
//     // Autogenerated from JSON contract definition using Rust contract convertor.
//     #![allow(unused_imports)]
//     use std::string::String;
//     use std::result::Result;
//     use std::fmt;
//     use {util, ethabi};
//     use util::{FixedHash, Uint};
//
//     pub struct Contract {
//         contract: ethabi::Contract,
//         pub address: util::Address,
//         do_call: Box<Fn(util::Address, Vec<u8>) -> Result<Vec<u8>, String> + Send + Sync + 'static>,
//     }
//     impl Contract {
//         pub fn new<F>(address: util::Address, do_call: F) -> Self
//             where F: Fn(util::Address, Vec<u8>) -> Result<Vec<u8>, String> + Send + Sync + 'static {
//             Contract {
//                 contract: ethabi::Contract::new(ethabi::Interface::load(b"[{\"constant\":false,\"inputs\":[{\"name\":\"epochIndex\",\"type\":\"uint64\"},{\"name\":\"sender\",\"type\":\"address\"}],\"name\":\"getCommitmentsAndShares\",\"outputs\":[{\"name\":\"\",\"type\":\"bytes\"},{\"name\":\"\",\"type\":\"bytes\"}],\"payable\":false,\"type\":\"function\"},{\"constant\":false,\"inputs\":[{\"name\":\"epochIndex\",\"type\":\"uint64\"},{\"name\":\"secret\",\"type\":\"bytes\"}],\"name\":\"saveSecret\",\"outputs\":[],\"payable\":false,\"type\":\"function\"},{\"constant\":false,\"inputs\":[{\"name\":\"epochIndex\",\"type\":\"uint64\"},{\"name\":\"sender\",\"type\":\"address\"}],\"name\":\"getSecret\",\"outputs\":[{\"name\":\"\",\"type\":\"bytes\"}],\"payable\":false,\"type\":\"function\"},{\"constant\":false,\"inputs\":[{\"name\":\"epochIndex\",\"type\":\"uint64\"},{\"name\":\"commitments\",\"type\":\"bytes\"},{\"name\":\"shares\",\"type\":\"bytes\"}],\"name\":\"saveCommitmentsAndShares\",\"outputs\":[],\"payable\":false,\"type\":\"function\"}]").expect("JSON is autogenerated; qed")),
//                 address: address,
//                 do_call: Box::new(do_call),
//             }
//         }
//         fn as_string<T: fmt::Debug>(e: T) -> String { format!("{:?}", e) }
//
//         /// Auto-generated from: `{"constant":false,"inputs":[{"name":"epochIndex","type":"uint64"},{"name":"sender","type":"address"}],"name":"getCommitmentsAndShares","outputs":[{"name":"","type":"bytes"},{"name":"","type":"bytes"}],"payable":false,"type":"function"}`
//         #[allow(dead_code)]
//         pub fn get_commitments_and_shares(&self, epoch_index: u64, sender: &util::Address) -> Result<(Vec<u8>, Vec<u8>), String>
//              {
//             // let call = self.contract.function("getCommitmentsAndShares".into()).map_err(Self::as_string)?;
//
//
//             let call = self.contract.function("getCommitmentsAndShares".into()).expect("nope");
//
//             // let data = call.encode_call(
//             //     vec![ethabi::Token::Uint({ let mut r = [0u8; 32]; util::U256::from(epoch_index as u64).to_big_endian(&mut r); r }), ethabi::Token::Address(sender.clone().0)]
//             // ).map_err(Self::as_string)?;
//
//             let data = call.encode_call(
//                 vec![ethabi::Token::Uint({ let mut r = [0u8; 32]; util::U256::from(epoch_index as u64).to_big_endian(&mut r); r }), ethabi::Token::Address(sender.clone().0)]
//             ).expect("data nope");
//
//             // let output = call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
//
//             let part = (self.do_call)(self.address.clone(), data).expect("part nope");
//             let output = call.decode_output(part).expect("output nope");
//
//             let mut result = output.into_iter().rev().collect::<Vec<_>>();
//
//             Ok(({
//                 let r = result.pop().ok_or("Invalid return arity")?;
//                 let r = r.to_bytes().ok_or("Invalid type returned")?;
//                 r
//             }, {
//                 let r = result.pop().ok_or("Invalid return arity")?;
//                 let r = r.to_bytes().ok_or("Invalid type returned")?;
//                 r
//             }))
//         }
//
//         /// Auto-generated from: `{"constant":false,"inputs":[{"name":"epochIndex","type":"uint64"},{"name":"secret","type":"bytes"}],"name":"saveSecret","outputs":[],"payable":false,"type":"function"}`
//         #[allow(dead_code)]
//         pub fn save_secret(&self, epoch_index: u64, secret: &[u8]) -> Result<(), String>
//              {
//             let call = self.contract.function("saveSecret".into()).map_err(Self::as_string)?;
//             let data = call.encode_call(
//                 vec![ethabi::Token::Uint({ let mut r = [0u8; 32]; util::U256::from(epoch_index as u64).to_big_endian(&mut r); r }), ethabi::Token::Bytes(secret.to_owned())]
//             ).map_err(Self::as_string)?;
//             call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
//
//             Ok(())
//         }
//
//         /// Auto-generated from: `{"constant":false,"inputs":[{"name":"epochIndex","type":"uint64"},{"name":"sender","type":"address"}],"name":"getSecret","outputs":[{"name":"","type":"bytes"}],"payable":false,"type":"function"}`
//         #[allow(dead_code)]
//         pub fn get_secret(&self, epoch_index: u64, sender: &util::Address) -> Result<Vec<u8>, String>
//              {
//             let call = self.contract.function("getSecret".into()).map_err(Self::as_string)?;
//             let data = call.encode_call(
//                 vec![ethabi::Token::Uint({ let mut r = [0u8; 32]; util::U256::from(epoch_index as u64).to_big_endian(&mut r); r }), ethabi::Token::Address(sender.clone().0)]
//             ).map_err(Self::as_string)?;
//             let output = call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
//             let mut result = output.into_iter().rev().collect::<Vec<_>>();
//             Ok(({ let r = result.pop().ok_or("Invalid return arity")?; let r = r.to_bytes().ok_or("Invalid type returned")?; r }))
//         }
//
//         /// Auto-generated from: `{"constant":false,"inputs":[{"name":"epochIndex","type":"uint64"},{"name":"commitments","type":"bytes"},{"name":"shares","type":"bytes"}],"name":"saveCommitmentsAndShares","outputs":[],"payable":false,"type":"function"}`
//         #[allow(dead_code)]
//         pub fn save_commitments_and_shares(&self, epoch_index: u64, commitments: &[u8], shares: &[u8]) -> Result<(), String>
//              {
//             let call = self.contract.function("saveCommitmentsAndShares".into()).map_err(Self::as_string)?;
//             let data = call.encode_call(
//                 vec![ethabi::Token::Uint({ let mut r = [0u8; 32]; util::U256::from(epoch_index as u64).to_big_endian(&mut r); r }), ethabi::Token::Bytes(commitments.to_owned()), ethabi::Token::Bytes(shares.to_owned())]
//             ).map_err(Self::as_string)?;
//             call.decode_output((self.do_call)(self.address.clone(), data)?).map_err(Self::as_string)?;
//
//             Ok(())
//         }
//     }
// }
//

#[cfg(test)]
mod tests {
    use util::*;
    use spec::Spec;
    use tests::helpers::generate_dummy_client_with_spec_and_accounts;
    use super::PvssContract;
    use client::BlockChainClient;
	use account_provider::AccountProvider;
	use ethkey::Secret;
    use miner::MinerService;

    #[test]
    fn fetches_commitments() {
        ::env_logger::init();

        let client = generate_dummy_client_with_spec_and_accounts(Spec::new_pvss_contract, None);

		let tap = Arc::new(AccountProvider::transient_provider());
		let addr1 = tap.insert_account(Secret::from_slice(&"1".sha3()).unwrap(), "1").unwrap();


        client.engine().register_client(Arc::downgrade(&client));
		// Make sure reporting can be done.
		client.miner().set_gas_floor_target(1_000_000.into());

		client.engine().set_signer(tap.clone(), addr1, "1".into());

        client.engine().step();
        client.engine().step();
    }
}