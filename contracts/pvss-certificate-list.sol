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

contract PvssCertificateList {
    struct PvssInfo {
        bytes commitments;
        bytes shares;
        bytes secret;
    }

    mapping (uint64 => mapping(address => PvssInfo)) by_epoch;


    function saveCommitmentsAndShares(
        uint64 epochIndex,
        bytes commitments,
        bytes shares) external {

        bytes storage b;

        by_epoch[epochIndex][msg.sender] = PvssInfo(
            commitments, shares, b
        );
    }

    function getCommitmentsAndShares(
        uint64 epochIndex,
        address sender
    ) external returns (bytes, bytes) {
        PvssInfo pvss_info = by_epoch[epochIndex][sender];
        return (pvss_info.commitments, pvss_info.shares);
    }
}
