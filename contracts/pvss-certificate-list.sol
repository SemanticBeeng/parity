pragma solidity ^0.4.6;

contract PvssCertificateList {
    mapping (uint64 => mapping (address => bytes)) commitments;
    mapping (uint64 => mapping (address => bytes)) shares;

    function saveCommitmentsAndShares(
        uint64 epochIndex,
        bytes commitment_bytes,
        bytes share_bytes
    ) external {
        commitments[epochIndex][msg.sender] = commitment_bytes;
        shares[epochIndex][msg.sender] = share_bytes;
    }

    function getCommitmentsAndShares(
        uint64 epochIndex,
        address sender
    ) external returns (bytes, bytes) {
        return (commitments[epochIndex][sender], shares[epochIndex][sender]);
    }
}
