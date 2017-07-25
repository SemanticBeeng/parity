// Copyright 2015-2017 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

//! A blockchain engine using the Ouroboros protocol.

use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering as AtomicOrdering};
use std::sync::Weak;
use std::time::{UNIX_EPOCH, Duration};
use util::*;
use ethkey::{verify_address, Signature};
use rlp::{UntrustedRlp, View, encode};
use account_provider::AccountProvider;
use block::*;
use spec::CommonParams;
use engines::{Engine, Seal, EngineError};
use header::{Header, BlockNumber};
use error::{Error, TransactionError, BlockError};
use evm::Schedule;
use ethjson;
use io::{IoContext, IoHandler, TimerToken, IoService};
use env_info::EnvInfo;
use builtin::Builtin;
use transaction::UnverifiedTransaction;
use client::{Client, EngineClient, BlockId};
use super::signer::EngineSigner;
use super::validator_set::{ValidatorSet, new_validator_set};

use pvss;

mod fts;
mod pvss_contract;

// Type aliases to match cardano types
type Coin = U256;
type StakeholderId = Address;
type SlotLeaders = Vec<StakeholderId>;
type Stakes = HashMap<StakeholderId, Coin>;

/// Stage in the pvss process. Intentionally not implementing the recover
/// phase; for the purposes of performance testing, we are assuming all
/// nodes are honest and available, which cannot be assumed in a production
/// environment.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
enum PvssStage {
    Commit,
    CommitBroadcast,
    Reveal,
    // Recover, TODO
}

/// `Ouroboros` params.
#[derive(Debug, PartialEq)]
pub struct OuroborosParams {
	/// Time to wait before next block or authority switching, in seconds.
    /// Equivalent to slot duration in the Ouroboros paper.
	pub step_duration: Duration,
	/// Validators. Equivalent to stakeholders/leaders in the Ouroboros paper.
	pub validators: ethjson::spec::ValidatorSet,
    /// Security parameter k. A transaction is declared stable if and only if
    /// it is in a block that is more than this many blocks deep in the
    /// ledger. Equivalent to blkSecurityParam in cardano.
    pub security_parameter_k: u64,
    /// Security parameter expressed in number of slots. Equivalent to
    /// slotSecurityParam in cardano.
    pub slot_security_parameter: u64,
    /// Number of slots inside one epoch. Equivalent to epochSlots in cardano.
    pub epoch_slots: u64,
	/// Time that the chain began
	pub network_wide_start_time: Option<u64>,
	/// Namereg contract address.
	pub registrar: Address,
	/// Starting step, only used for testing.
	pub start_step: Option<u64>,
	/// Gas limit divisor. Needed by Parity/Authority Round, so including to be comparable.
	pub gas_limit_bound_divisor: U256,
	/// Number of first block where EIP-155 rules are validated.
    /// Needed by Parity/Authority Round, so including to be comparable.
	pub eip155_transition: u64,
}

impl From<ethjson::spec::OuroborosParams> for OuroborosParams {
	fn from(p: ethjson::spec::OuroborosParams) -> Self {
		OuroborosParams {
			step_duration: Duration::from_secs(p.step_duration.into()),
			validators: p.validators,
            security_parameter_k: p.security_parameter_k,
            slot_security_parameter: 2 * p.security_parameter_k,
            epoch_slots: 10 * p.security_parameter_k,
            network_wide_start_time: p.network_wide_start_time.map(Into::into),
			registrar: Address::new(),
			start_step: p.start_step.map(Into::into),
			gas_limit_bound_divisor: p.gas_limit_bound_divisor.into(),
            eip155_transition: p.eip155_transition.map_or(0, Into::into),
		}
	}
}

/// Engine using `Ouroborous` proof-of-work consensus algorithm
pub struct Ouroboros {
	params: CommonParams,
    epoch_slots: u64,
	step_duration: Duration,
	step: AtomicUsize,
    network_wide_start_time: u64,
	proposed: AtomicBool,
	signer: EngineSigner,
	validators: Box<ValidatorSet>,
    pvss_secret: PvssSecret,
    pvss_stage: RwLock<PvssStage>,
    pvss_contract: pvss_contract::PvssContract,
    security_parameter_k: u64,
	transition_service: IoService<()>,
	registrar: Address,
	builtins: BTreeMap<Address, Builtin>,
	client: RwLock<Option<Weak<EngineClient>>>,
    slot_leaders: RwLock<SlotLeaders>,
	gas_limit_bound_divisor: U256,
	eip155_transition: u64,
}

fn header_step(header: &Header) -> Result<usize, ::rlp::DecoderError> {
	UntrustedRlp::new(&header.seal().get(0).expect("was either checked with verify_block_basic or is genesis; has 2 fields; qed (Make sure the spec file has a correct genesis seal)")).as_val()
}

fn header_signature(header: &Header) -> Result<Signature, ::rlp::DecoderError> {
	UntrustedRlp::new(&header.seal().get(1).expect("was checked with verify_block_basic; has 2 fields; qed")).as_val::<H520>().map(Into::into)
}

trait AsMillis {
	fn as_millis(&self) -> u64;
}

impl AsMillis for Duration {
	fn as_millis(&self) -> u64 {
		self.as_secs()*1_000 + (self.subsec_nanos()/1_000_000) as u64
	}
}

struct PvssSecret {
    escrow: pvss::simple::Escrow,
    commitments: Vec<pvss::simple::Commitment>,
    shares: Vec<pvss::simple::EncryptedShare>,
}

unsafe impl Send for PvssSecret {}
unsafe impl Sync for PvssSecret {}

impl PvssSecret {
    pub fn new(validator_set: &Box<ValidatorSet>, accounts: &ethjson::spec::State) -> Self {
        // Calculate the threshold in the same way as cardano does https://github.com/input-output-hk/cardano-sl/blob/9d527fd/godtossing/Pos/Ssc/GodTossing/Functions.hs#L138-L141
        let num_validators = validator_set.validators().len();
        let threshold = num_validators / 2 + num_validators % 2;

        let escrow = pvss::simple::escrow(threshold as u32);
        let commitments = pvss::simple::commitments(&escrow);

        let public_keys: Vec<_> = validator_set.validators()
            .iter()
            .flat_map(|&v| {
                accounts.0.get(&From::from(v)).map(|account| {
                    account.pvss.as_ref().expect(
                        &format!("could not find pvss for {}", v)
                    ).public_key.as_ref().expect(
                        &format!("could not find public key for {}", v)
                    )
                })
            }).collect();

        let shares = pvss::simple::create_shares(&escrow, public_keys);

        PvssSecret {
            escrow,
            commitments,
            shares,
        }
    }
}

impl Ouroboros {
	/// Create a new instance of the Ouroboros engine.
	pub fn new(params: CommonParams, our_params: OuroborosParams, builtins: BTreeMap<Address, Builtin>, accounts: &ethjson::spec::State) -> Result<Arc<Self>, Error> {

        // Turn off timeouts during testing so that a test always runs within
        // one step.
		let should_timeout = our_params.start_step.is_none();

        // Set the initial step number based on the start step parameter if
        // we're testing, or 0.
		let initial_step = our_params.start_step.unwrap_or(0) as usize;

        let validators = new_validator_set(our_params.validators);

        let stakeholders = Ouroboros::stakeholders(&validators, accounts);
        let mut stakeholders: Vec<(StakeholderId, Coin)> = stakeholders.into_iter().collect();
        stakeholders.sort_by_key(|&(id, _)| id);

        let sorted_stakeholders: Vec<_> = stakeholders.iter().map(|&(id, _)| id.clone()).collect();
        // TODO: what is my index?

        let total_stake = stakeholders.iter()
            .map(|&(_, amount)| amount)
            .fold(Coin::from(0), |acc, c| acc + c.into());

        // TODO: pass sorted_stakeholders instead
        let pvss_secret = PvssSecret::new(&validators, accounts);

        let seed: Option<&[u8]> = None;

		let engine = Arc::new(
			Ouroboros {
				params: params,
                epoch_slots: our_params.epoch_slots,
				step_duration: our_params.step_duration,
				step: AtomicUsize::new(initial_step),
                network_wide_start_time: our_params.network_wide_start_time.unwrap_or(0),
				proposed: AtomicBool::new(false),
				signer: Default::default(),
				validators: validators,
				transition_service: IoService::<()>::start()?,
				registrar: our_params.registrar,
				builtins: builtins,
				client: RwLock::new(None),
                slot_leaders: RwLock::new(fts::follow_the_satoshi(
                    seed,
                    &stakeholders,
                    our_params.epoch_slots,
                    total_stake,
                )),
                pvss_secret: pvss_secret,
                pvss_stage: RwLock::new(PvssStage::Commit),
                pvss_contract: pvss_contract::PvssContract::new(),
                security_parameter_k: our_params.security_parameter_k,
				gas_limit_bound_divisor: our_params.gas_limit_bound_divisor,
				eip155_transition: our_params.eip155_transition,
			});
		// Do not initialize timeouts for tests.
		if should_timeout {
			let handler = TransitionHandler { engine: Arc::downgrade(&engine) };
			engine.transition_service.register_handler(Arc::new(handler))?;
		}
		Ok(engine)
	}

    fn epoch_number(&self) -> usize {
        let step = self.step.load(AtomicOrdering::SeqCst);
        step / self.epoch_slots as usize
    }

    fn slot_in_epoch(&self) -> usize {
        let step = self.step.load(AtomicOrdering::SeqCst);
        step % self.epoch_slots as usize
    }

    fn after_4k_slots(&self) -> bool {
        self.slot_in_epoch() > 4 * self.security_parameter_k as usize
    }

    fn first_slot_in_new_epoch(&self) -> bool {
        self.slot_in_epoch() == 0
    }

    fn compute_new_slot_leaders(&self) {
        let step = self.step.load(AtomicOrdering::SeqCst);
        let back_2k_slots = step - 2 * self.security_parameter_k as usize;
        let last_epoch = self.epoch_number() - 1;

        if let Some(ref weak) = *self.client.read() {
            if let Some(client) = weak.upgrade() {
                // TODO: save sorted_stakeholders and use that instead
                let mut stakeholders: Vec<(StakeholderId, Coin)> = self.validators
                    .validators()
                    .into_iter()
                    .map(|&validator| {
                        (
                            validator,
                            client.balance(
                                &validator,
                                BlockId::Number(back_2k_slots as BlockNumber)
                            ).unwrap_or(Coin::from(0))
                        )
                    }).collect();
                stakeholders.sort_by_key(|&(id, _)| id);

                let total_stake = stakeholders.iter().map(|&(_, amount)| amount).fold(Coin::from(0), |acc, c| acc + c.into());

                // for &(address, _) in &stakeholders {
                //     let commit_info = self.pvss_contract
                //         .get_commitments_and_shares(last_epoch, &address)
                //         .expect(&format!("could not get commitments and shares for epoch {}, address {:?}", last_epoch, address));
                //     // TODO: match up my private key with my index and the commitments from other
                //     // nodes for me, decrypt and verify
                // }

                // let zero = vec![0];
                // let seed: Vec<u8> = stakeholders.iter()
                //     .map(|&(address, _)| address)
                //     .fold(zero, |acc, address| {
                //         let secret = self.pvss_contract
                //             .get_secret(last_epoch, &address)
                //             .expect(&format!("could not get secret for epoch {}, address {:?}", last_epoch, address)).to_bytes();
                //         acc.iter().zip(secret.iter()).map(|(a, b)| a ^ b).collect()
                //     });
                //
                // println!("shared seed is {:#?}", seed);
                //
                // let slot_leaders = fts::follow_the_satoshi(
                //     Some(&seed),
                //     &stakeholders,
                //     self.epoch_slots,
                //     total_stake,
                // );
                //
                // // placeholder
                // println!("new slot leader schedule: {:#?}", slot_leaders);
                //
                // *self.slot_leaders.write() = slot_leaders;
                // TODO: generate and save new pvss_secret
            }
        }
    }

	fn remaining_step_duration(&self) -> Duration {
		let now = unix_now();
        let network_wide_start_time = Duration::from_secs(self.network_wide_start_time);

        if network_wide_start_time > now {
            return network_wide_start_time - now;
        }

        let duration_seconds = self.step_duration.as_secs() as u64;

        let step_end = Duration::from_secs(
            duration_seconds *
            (self.step.load(AtomicOrdering::SeqCst) as u64 + 1)
        ) + network_wide_start_time;

		if step_end > now {
			step_end - now
		} else {
			Duration::from_secs(0)
		}
	}

	fn step_proposer(&self, step: usize) -> Address {
        let step = step % self.slot_leaders.read().len();
        let address = (*self.slot_leaders.read())[step].clone();
        address
	}

	fn is_step_proposer(&self, step: usize, address: &Address) -> bool {
		self.step_proposer(step) == *address
	}

	fn is_future_step(&self, step: usize) -> bool {
		step > self.step.load(AtomicOrdering::SeqCst) + 1
	}

    fn stakeholders(validator_set: &Box<ValidatorSet>, accounts: &ethjson::spec::State) -> Stakes {
        validator_set.validators().into_iter().flat_map(|&v| {
            accounts.0.get(&From::from(v)).map(|account| {
                (v, account.balance.map_or(Coin::from(0), |c| c.into()))
            })
        }).collect()
    }
}

fn unix_now() -> Duration {
	UNIX_EPOCH.elapsed().expect("Valid time has to be set in your system.")
}

struct TransitionHandler {
	engine: Weak<Ouroboros>,
}

const ENGINE_TIMEOUT_TOKEN: TimerToken = 23;

impl IoHandler<()> for TransitionHandler {
	fn initialize(&self, io: &IoContext<()>) {
		if let Some(engine) = self.engine.upgrade() {
			io.register_timer_once(ENGINE_TIMEOUT_TOKEN, engine.remaining_step_duration().as_millis())
				.unwrap_or_else(|e| warn!(target: "engine", "Failed to start consensus step timer: {}.", e))
		}
	}

	fn timeout(&self, io: &IoContext<()>, timer: TimerToken) {
		if timer == ENGINE_TIMEOUT_TOKEN {
			if let Some(engine) = self.engine.upgrade() {
				engine.step();
				io.register_timer_once(ENGINE_TIMEOUT_TOKEN, engine.remaining_step_duration().as_millis())
					.unwrap_or_else(|e| warn!(target: "engine", "Failed to restart consensus step timer: {}.", e))
			}
		}
	}
}

impl Engine for Ouroboros {
	fn name(&self) -> &str { "Ouroboros" }

	fn version(&self) -> SemanticVersion { SemanticVersion::new(1, 0, 0) }

	/// Two fields:
    ///
    /// - consensus step
    /// - proposer signature
	fn seal_fields(&self) -> usize { 2 }

	fn params(&self) -> &CommonParams { &self.params }

	fn additional_params(&self) -> HashMap<String, String> { hash_map!["registrar".to_owned() => self.registrar.hex()] }

	fn builtins(&self) -> &BTreeMap<Address, Builtin> { &self.builtins }

	fn step(&self) {
		self.step.fetch_add(1, AtomicOrdering::SeqCst);

        let pvss_stage = *self.pvss_stage.read();

        if pvss_stage == PvssStage::Commit {

            println!("epoch number sending = {}", self.epoch_number());
            self.pvss_contract.broadcast_commitments_and_shares(
                self.epoch_number(),
                &self.pvss_secret.commitments,
                &self.pvss_secret.shares,
            );


            *self.pvss_stage.write() = PvssStage::CommitBroadcast;
        } else if pvss_stage == PvssStage::CommitBroadcast && self.after_4k_slots() {
            // self.pvss_contract.broadcast_secret(
            //     self.epoch_number(),
            //     &self.pvss_secret.escrow.secret
            // );
            *self.pvss_stage.write() = PvssStage::Reveal;
        } else if pvss_stage == PvssStage::Reveal && self.first_slot_in_new_epoch() {
            self.compute_new_slot_leaders();
            *self.pvss_stage.write() = PvssStage::Commit;
        }

            let address = self.signer.address();


            let (commitments, shares) = self.pvss_contract
                .get_commitments_and_shares(0, &address)
                .expect(&format!("could not get commitments for epoch 0, address {:?}",  address));

            assert!(commitments == self.pvss_secret.commitments);
            assert!(shares == self.pvss_secret.shares);


		self.proposed.store(false, AtomicOrdering::SeqCst);
		if let Some(ref weak) = *self.client.read() {
			if let Some(c) = weak.upgrade() {
				c.update_sealing();
			}
		}
	}

	/// Additional engine-specific information for the user/developer concerning `header`.
	fn extra_info(&self, header: &Header) -> BTreeMap<String, String> {
		map![
			"step".into() => header_step(header).as_ref().map(ToString::to_string).unwrap_or("".into()),
			"signature".into() => header_signature(header).as_ref().map(ToString::to_string).unwrap_or("".into())
		]
	}

	fn schedule(&self, _env_info: &EnvInfo) -> Schedule {
		Schedule::new_post_eip150(usize::max_value(), true, true, true)
	}

	fn populate_from_parent(&self, header: &mut Header, parent: &Header, gas_floor_target: U256, _gas_ceil_target: U256) {
		// Chain scoring: weak height scoring, backported for compatibility.
		header.set_difficulty(parent.difficulty().clone());
		header.set_gas_limit({
			let gas_limit = parent.gas_limit().clone();
			let bound_divisor = self.gas_limit_bound_divisor;
			if gas_limit < gas_floor_target {
				min(gas_floor_target, gas_limit + gas_limit / bound_divisor - 1.into())
			} else {
				max(gas_floor_target, gas_limit - gas_limit / bound_divisor + 1.into())
			}
		});
	}

	fn seals_internally(&self) -> Option<bool> {
		Some(self.signer.address() != Address::default())
	}

	/// Attempt to seal the block internally.
	///
	/// This operation is synchronous and may (quite reasonably) not be available, in which `false` will
	/// be returned.
	fn generate_seal(&self, block: &ExecutedBlock) -> Seal {
		if self.proposed.load(AtomicOrdering::SeqCst) { return Seal::None; }
		let header = block.header();
		let step = self.step.load(AtomicOrdering::SeqCst);
		if true { //self.is_step_proposer(step, header.author()) {
			if let Ok(signature) = self.signer.sign(header.bare_hash()) {
				trace!(target: "engine", "generate_seal: Issuing a block for step {}.", step);
				self.proposed.store(true, AtomicOrdering::SeqCst);
				return Seal::Regular(vec![
                    encode(&step).to_vec(),
                    encode(&(&H520::from(signature) as &[u8])).to_vec()
                ]);
			} else {
				warn!(target: "engine", "generate_seal: FAIL: Accounts secret key unavailable.");
			}
		} else {
			trace!(target: "engine", "generate_seal: Not a proposer for step {}.", step);
		}
		Seal::None
	}

	/// No block reward
	fn on_close_block(&self, block: &mut ExecutedBlock) {
		let fields = block.fields_mut();
        let res = fields.state.commit();
		// Commit state so that we can actually figure out the state root.
		if let Err(e) = res {
			warn!("Encountered error on closing block: {}", e);
		}
	}

	/// Check the number of seal fields.
	fn verify_block_basic(&self, header: &Header, _block: Option<&[u8]>) -> Result<(), Error> {
		if header.seal().len() != self.seal_fields() {
			trace!(target: "engine", "verify_block_basic: wrong number of seal fields");
			Err(From::from(BlockError::InvalidSealArity(
				Mismatch { expected: self.seal_fields(), found: header.seal().len() }
			)))
		} else {
			Ok(())
		}
	}

	fn verify_block_unordered(&self, _header: &Header, _block: Option<&[u8]>) -> Result<(), Error> {
		Ok(())
	}

	/// Do the validator and gas limit validation.
	fn verify_block_family(&self, header: &Header, parent: &Header, _block: Option<&[u8]>) -> Result<(), Error> {
		let step = header_step(header)?;
		// Give one step slack if step is lagging, double vote is still not possible.
		if self.is_future_step(step) {
			self.validators.report_benign(header.author());
			Err(BlockError::InvalidSeal)?
		} else {
			// Check if the signature belongs to a validator, can depend on parent state.
			let proposer_signature = header_signature(header)?;
			let correct_proposer = self.step_proposer(step);
			if !verify_address(&correct_proposer, &proposer_signature, &header.bare_hash())? {
				trace!(target: "engine", "verify_block_unordered: bad proposer for step: {}", step);
				Err(EngineError::NotProposer(Mismatch { expected: correct_proposer, found: header.author().clone() }))?
			}
		}

		// Do not calculate difficulty for genesis blocks.
		if header.number() == 0 {
			return Err(From::from(BlockError::RidiculousNumber(OutOfBounds { min: Some(1), max: None, found: header.number() })));
		}

		// Check if parent is from a previous step.
		let parent_step = header_step(parent)?;
		if step == parent_step || step <= parent_step {
			trace!(target: "engine", "Multiple blocks proposed for step {}.", parent_step);
			self.validators.report_malicious(header.author());
			Err(EngineError::DoubleVote(header.author().clone()))?;
		}

		let gas_limit_divisor = self.gas_limit_bound_divisor;
		let min_gas = parent.gas_limit().clone() - parent.gas_limit().clone() / gas_limit_divisor;
		let max_gas = parent.gas_limit().clone() + parent.gas_limit().clone() / gas_limit_divisor;
		if header.gas_limit() <= &min_gas || header.gas_limit() >= &max_gas {
			return Err(From::from(BlockError::InvalidGasLimit(OutOfBounds { min: Some(min_gas), max: Some(max_gas), found: header.gas_limit().clone() })));
		}

		Ok(())
	}

	fn verify_transaction_basic(&self, t: &UnverifiedTransaction, header: &Header) -> result::Result<(), Error> {
		t.check_low_s()?;

		if let Some(n) = t.network_id() {
			if header.number() >= self.eip155_transition && n != self.params().chain_id {
				return Err(TransactionError::InvalidNetworkId.into());
			}
		}

		Ok(())
	}

	fn register_client(&self, client: Weak<Client>) {
		*self.client.write() = Some(client.clone());
		self.pvss_contract.register_contract(client);
	}

	fn set_signer(&self, ap: Arc<AccountProvider>, address: Address, password: String) {
		self.signer.set(ap, address, password);
	}

	fn sign(&self, hash: H256) -> Result<Signature, Error> {
		self.signer.sign(hash).map_err(Into::into)
	}
}

#[cfg(test)]
mod tests {
	use util::*;
	use env_info::EnvInfo;
	use header::Header;
	use error::{Error, BlockError};
	use ethkey::Secret;
	use rlp::encode;
	use block::*;
	use tests::helpers::*;
	use account_provider::AccountProvider;
	use spec::Spec;
    use ethjson;
	use engines::Seal;
    use super::*;

	#[test]
	fn has_valid_metadata() {
		let engine = Spec::new_test_ouroboros().engine;
		assert!(!engine.name().is_empty());
		assert!(engine.version().major >= 1);
	}

	#[test]
	fn can_return_schedule() {
		let engine = Spec::new_test_ouroboros().engine;
		let schedule = engine.schedule(&EnvInfo {
			number: 10000000,
			author: 0.into(),
			timestamp: 0,
			difficulty: 0.into(),
			last_hashes: Arc::new(vec![]),
			gas_used: 0.into(),
			gas_limit: 0.into(),
		});

		assert!(schedule.stack_limit > 0);
	}

	#[test]
	fn verification_fails_on_short_seal() {
		let engine = Spec::new_test_ouroboros().engine;
		let header: Header = Header::default();

		let verify_result = engine.verify_block_basic(&header, None);

		match verify_result {
			Err(Error::Block(BlockError::InvalidSealArity(_))) => {},
			Err(_) => { panic!("should be block seal-arity mismatch error (got {:?})", verify_result); },
			_ => { panic!("Should be error, got Ok"); },
		}
	}

	#[test]
	fn can_do_signature_verification_fail() {
		let engine = Spec::new_test_ouroboros().engine;
		let mut header: Header = Header::default();
		header.set_seal(vec![encode(&H520::default()).to_vec()]);

		let verify_result = engine.verify_block_family(&header, &Default::default(), None);
		assert!(verify_result.is_err());
	}

	#[test]
	fn generates_seal_and_does_not_double_propose() {
		let tap = Arc::new(AccountProvider::transient_provider());
		let addr1 = tap.insert_account(Secret::from_slice(&"1".sha3()).unwrap(), "1").unwrap();
		let addr2 = tap.insert_account(Secret::from_slice(&"2".sha3()).unwrap(), "2").unwrap();

		let spec = Spec::new_test_ouroboros();
		let engine = &*spec.engine;
		let genesis_header = spec.genesis_header();
		let db1 = spec.ensure_db_good(get_temp_state_db().take(), &Default::default()).unwrap();
		let db2 = spec.ensure_db_good(get_temp_state_db().take(), &Default::default()).unwrap();
		let last_hashes = Arc::new(vec![genesis_header.hash()]);
		let b1 = OpenBlock::new(engine, Default::default(), false, db1, &genesis_header, last_hashes.clone(), addr1, (3141562.into(), 31415620.into()), vec![]).unwrap();
		let b1 = b1.close_and_lock();
		let b2 = OpenBlock::new(engine, Default::default(), false, db2, &genesis_header, last_hashes, addr2, (3141562.into(), 31415620.into()), vec![]).unwrap();
		let b2 = b2.close_and_lock();

		engine.set_signer(tap.clone(), addr1, "1".into());
		if let Seal::Regular(seal) = engine.generate_seal(b1.block()) {
			assert!(b1.clone().try_seal(engine, seal).is_ok());
			// Second proposal is forbidden.
			assert!(engine.generate_seal(b1.block()) == Seal::None);
		}

		engine.set_signer(tap, addr2, "2".into());
		if let Seal::Regular(seal) = engine.generate_seal(b2.block()) {
			assert!(b2.clone().try_seal(engine, seal).is_ok());
			// Second proposal is forbidden.
			assert!(engine.generate_seal(b2.block()) == Seal::None);
		}
	}

	#[test]
	fn proposer_switching() {
		let tap = AccountProvider::transient_provider();
		let addr = tap.insert_account(Secret::from_slice(&"1".sha3()).unwrap(), "0").unwrap();
		let mut parent_header: Header = Header::default();
        parent_header.set_seal(
			vec![
                encode(&0usize).to_vec()
            ]
        );
		parent_header.set_gas_limit(U256::from_str("222222").unwrap());
		let mut header: Header = Header::default();
		header.set_number(1);
		header.set_gas_limit(U256::from_str("222222").unwrap());
		header.set_author(addr);

		let engine = Spec::new_test_ouroboros().engine;

		let signature = tap.sign(addr, Some("0".into()), header.bare_hash()).unwrap();

		// Two validators.

		header.set_seal(
            vec![
                encode(&2usize).to_vec(),
                encode(&(&*signature as &[u8])).to_vec()
            ]
        );
		assert!(engine.verify_block_family(&header, &parent_header, None).is_err());

		header.set_seal(
            vec![
                encode(&1usize).to_vec(),
                encode(&(&*signature as &[u8])).to_vec()
            ]
        );
		assert!(engine.verify_block_family(&header, &parent_header, None).is_ok());
	}

	#[test]
	fn rejects_future_block() {
		let tap = AccountProvider::transient_provider();
		let addr = tap.insert_account(Secret::from_slice(&"1".sha3()).unwrap(), "0").unwrap();

		let mut parent_header: Header = Header::default();
        parent_header.set_seal(
			vec![
                encode(&0usize).to_vec(),
            ]
        );
		parent_header.set_gas_limit(U256::from_str("222222").unwrap());
		let mut header: Header = Header::default();
		header.set_number(1);
		header.set_gas_limit(U256::from_str("222222").unwrap());
		header.set_author(addr);

		let engine = Spec::new_test_ouroboros().engine;

		let signature = tap.sign(addr, Some("0".into()), header.bare_hash()).unwrap();

		// Two validators.
		header.set_seal(
            vec![
                encode(&1usize).to_vec(),
                encode(&(&*signature as &[u8])).to_vec()
            ]
        );
		assert!(engine.verify_block_family(&header, &parent_header, None).is_ok());

		header.set_seal(
            vec![
                encode(&5usize).to_vec(),
                encode(&(&*signature as &[u8])).to_vec()
            ]
        );
        assert!(engine.verify_block_family(&header, &parent_header, None).is_err());
	}

    fn account_with_balance(balance: u64) -> ethjson::spec::Account {
        ethjson::spec::Account {
        	balance: Some(ethjson::uint::Uint(balance.into())),
            ..ethjson::spec::Account::default()
        }
    }

    #[test]
    fn match_validators_and_accounts() {
        let aaa = ethjson::hash::Address(H160::from_str("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap());
        let bbb = ethjson::hash::Address(H160::from_str("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap());

        let validators = new_validator_set(
            ethjson::spec::ValidatorSet::List(vec![
                aaa.clone(),
                bbb.clone(),
            ]
        ));

        let mut ledger = BTreeMap::new();
        ledger.insert(aaa.clone(), account_with_balance(10));
        ledger.insert(bbb.clone(), account_with_balance(50));
        let accounts = ethjson::spec::State(ledger);

        let result = Ouroboros::stakeholders(&validators, &accounts);

        assert_eq!(result.get(&aaa.0), Some(&Coin::from(10)));
        assert_eq!(result.get(&bbb.0), Some(&Coin::from(50)));
    }

    #[test]
    fn validators_without_stake_are_excluded() {
        let aaa = ethjson::hash::Address(H160::from_str("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap());
        let bbb = ethjson::hash::Address(H160::from_str("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap());

        let validators = new_validator_set(
            ethjson::spec::ValidatorSet::List(vec![
                aaa.clone(),
                bbb.clone(),
            ]
        ));

        let mut ledger = BTreeMap::new();
        ledger.insert(aaa.clone(), account_with_balance(10));
        let accounts = ethjson::spec::State(ledger);

        let result = Ouroboros::stakeholders(&validators, &accounts);

        assert_eq!(result.get(&aaa.0), Some(&Coin::from(10)));
        assert_eq!(result.get(&bbb.0), None);
    }
}
