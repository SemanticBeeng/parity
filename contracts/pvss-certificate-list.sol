pragma solidity ^0.4.6;

contract PvssCertificateList {
    mapping (uint64 => mapping (address => bytes)) commitments;
    mapping (uint64 => mapping (address => bytes)) shares;
    mapping (uint64 => mapping (address => bytes)) secrets;

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

    function saveSecret(
        uint64 epochIndex,
        bytes secret_bytes
    ) external {
        secrets[epochIndex][msg.sender] = secret_bytes;
    }

    function getSecret(
        uint64 epochIndex,
        address sender
    ) external returns (bytes) {
        return secrets[epochIndex][sender];
    }
}
