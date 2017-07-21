pragma solidity ^0.4.6;

/*

In order to store and retrieve pvss values, we need to map:

epoch_number => {
    sender_address: {
        commitments: [commitment],
        shares: {recipient_address => share},
        secret: secret,
    }
}

If we always sort signer addresses lexicographically, and we never change
the set of signers, then we could use array indices for the signers. We could
also use array indices for the epochs:

[[
    {
        commitments: [commitment],
        shares: [share],
        secret: secret,
    }
]]

- Is this assumption valid? For now?
- Is making this assumption worth the tradeoffs?
- Would need to keep empty slots in the array held for each epoch, since
values might come in for different signers at different times
- Epochs should be strictly increasing

*/
pragma solidity ^0.4.6;

contract PvssCertificateList {
    struct PvssCommitInfo {
        bytes commitments;
        bytes shares;
    }

    struct PvssRevealInfo {
        bytes secret;
    }

    mapping (uint64 => mapping(address => PvssCommitInfo)) commit_by_epoch;
    mapping (uint64 => mapping(address => PvssRevealInfo)) reveal_by_epoch;

    function saveCommitmentsAndShares(
        uint64 epochIndex,
        bytes commitments,
        bytes shares) external {

        commit_by_epoch[epochIndex][msg.sender] = PvssCommitInfo(
            commitments, shares
        );
    }

    function getCommitmentsAndShares(
        uint64 epochIndex,
        address sender
    ) external returns (bytes, bytes) {
        PvssCommitInfo pvss_commit_info = commit_by_epoch[epochIndex][sender];
        return (pvss_commit_info.commitments, pvss_commit_info.shares);
    }

    function saveSecret(
        uint64 epochIndex,
        bytes secret) external {
        reveal_by_epoch[epochIndex][msg.sender] = PvssRevealInfo(
            secret
        );
    }

    function getSecret(
        uint64 epochIndex,
        address sender
    ) external returns (bytes) {
        PvssRevealInfo pvss_reveal_info = reveal_by_epoch[epochIndex][sender];
        return pvss_reveal_info.secret;
    }
}
