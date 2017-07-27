use bincode::{serialize, Infinite};

use pvss;

/// Which method of PVSS to use
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum PvssMethod {
    Simple,
    Scrape,
}

enum SecretType {
    Simple(PvssSimple),
    Scrape(PvssScrape),
}

pub struct PvssSecret {
    secret_type: SecretType,
}

unsafe impl Send for PvssSecret {}
unsafe impl Sync for PvssSecret {}

impl PvssSecret {
    pub fn new(pvss_method: PvssMethod, public_keys: &[Vec<u8>]) -> Self {
        match pvss_method {
            PvssMethod::Simple => PvssSecret {
                secret_type: SecretType::Simple(PvssSimple::new(public_keys))
            },
            PvssMethod::Scrape =>PvssSecret {
                secret_type: SecretType::Scrape(PvssScrape::new(public_keys))
            },
        }
    }

    pub fn secret_bytes(&self) -> Vec<u8> {
        match self.secret_type {
            SecretType::Simple(ref simple) => simple.secret_bytes(),
            SecretType::Scrape(ref scrape) => scrape.secret_bytes(),
        }
    }

    pub fn commitment_bytes(&self) -> Vec<u8> {
        match self.secret_type {
            SecretType::Simple(ref simple) => simple.commitment_bytes(),
            SecretType::Scrape(ref scrape) => scrape.commitment_bytes(),
        }
    }

    pub fn share_bytes(&self) -> Vec<u8> {
        match self.secret_type {
            SecretType::Simple(ref simple) => simple.share_bytes(),
            SecretType::Scrape(ref scrape) => scrape.share_bytes(),
        }
    }
}

struct PvssSimple {
    escrow: pvss::simple::Escrow,
    commitments: Vec<pvss::simple::Commitment>,
    shares: Vec<pvss::simple::EncryptedShare>,
}

impl PvssSimple {
    fn new(public_keys: &[Vec<u8>]) -> Self {
        // Calculate the threshold in the same way as cardano does https://github.com/input-output-hk/cardano-sl/blob/9d527fd/godtossing/Pos/Ssc/GodTossing/Functions.hs#L138-L141
        let num_stakeholders = public_keys.len();
        let threshold = num_stakeholders / 2 + num_stakeholders % 2;

        let public_keys: Vec<_> = public_keys.iter().map(|bytes| {
            pvss::crypto::PublicKey::from_bytes(bytes)
        }).collect();

        let escrow = pvss::simple::escrow(threshold as u32);
        let commitments = pvss::simple::commitments(&escrow);
        let shares = pvss::simple::create_shares(&escrow, &public_keys);

        PvssSimple {
            escrow,
            commitments,
            shares,
        }
    }

    fn secret_bytes(&self) -> Vec<u8> {
        serialize(&self.escrow.secret, Infinite).expect("could not serialize secret")
    }

    fn commitment_bytes(&self) -> Vec<u8> {
        serialize(&self.commitments, Infinite).expect("could not serialize commitments")
    }

    fn share_bytes(&self) -> Vec<u8> {
        serialize(&self.shares, Infinite).expect("could not serialize shares")
    }
}

pub struct PvssScrape {
    escrow: pvss::scrape::Escrow,
    public_shares: pvss::scrape::PublicShares,
}

impl PvssScrape {
    pub fn new(public_keys: &[Vec<u8>]) -> Self {
        // Calculate the threshold in the same way as cardano does https://github.com/input-output-hk/cardano-sl/blob/9d527fd/godtossing/Pos/Ssc/GodTossing/Functions.hs#L138-L141
        let num_stakeholders = public_keys.len();
        let threshold = num_stakeholders / 2 + num_stakeholders % 2;

        let public_keys: Vec<_> = public_keys.iter().map(|bytes| {
            pvss::crypto::PublicKey::from_bytes(bytes)
        }).collect();

        let escrow = pvss::scrape::escrow(threshold as u32);
        let public_shares = pvss::scrape::create_shares(&escrow, &public_keys);

        PvssScrape {
            escrow,
            public_shares,
        }
    }

    fn secret_bytes(&self) -> Vec<u8> {
        serialize(&self.escrow.secret, Infinite).expect("could not serialize secret")
    }

    fn commitment_bytes(&self) -> Vec<u8> {
        serialize(&self.public_shares.commitments, Infinite)
            .expect("could not serialize commitments")
    }

    fn share_bytes(&self) -> Vec<u8> {
        serialize(&self.public_shares.encrypted_shares, Infinite)
            .expect("could not serialize shares")
    }
}
