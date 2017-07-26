use pvss;

use ethjson;
use engines::validator_set::ValidatorSet;

pub struct PvssSecret {
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

        let escrow = pvss::simple::escrow(threshold as u32);
        let commitments = pvss::simple::commitments(&escrow);
        let shares = pvss::simple::create_shares(&escrow, public_keys);

        PvssSecret {
            escrow,
            commitments,
            shares,
        }
    }

    pub fn escrow(&self) -> &pvss::simple::Escrow {
        &self.escrow
    }

    pub fn commitments(&self) -> &[pvss::simple::Commitment] {
        &self.commitments
    }

    pub fn shares(&self) -> &[pvss::simple::EncryptedShare] {
        &self.shares
    }
}
