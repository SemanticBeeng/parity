pragma solidity ^0.4.6;

contract PvssCertificateList {
    mapping (uint64 => mapping (address => bytes)) commitments;

    function saveCommitment(
        uint64 epochIndex,
        bytes commitment
    ) external {
        commitments[epochIndex][msg.sender] = commitment;
    }

    function getCommitment(
        uint64 epochIndex,
        address sender
    ) external returns (bytes) {
        return commitments[epochIndex][sender];
    }
}
