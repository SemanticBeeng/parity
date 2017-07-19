#[cfg(test)]
mod tests {
    #[test]
    fn fetches_commitments() {
        // Commitments is empty initially.
		let client = generate_dummy_client_with_spec_and_accounts(Spec::new_pvss_contract, None);
		let vc = Arc::new(PvssContract::new(Address::from_str("0000000000000000000000000000000000000005").unwrap()));
		vc.register_contract(Arc::downgrade(&client));

    }

    #[test]
    fn fetches_reveals() {
        // Reveals is empty initially.
    }

    #[test]
    fn write_commit_messages_and_read_from_chain() {

    }

    #[test]
    fn write_reveal_messages_and_read_from_chain() {

    }
}
