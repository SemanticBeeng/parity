pragma solidity ^0.4.6;

contract PvssCertificateList {
    mapping (uint64 => bytes) theValue;

    function saveCommitment(
        uint64 epochIndex,
        bytes commitment
    ) external {
        theValue[epochIndex] = commitment;
    }

    function getCommitment(
        uint64 epochIndex,
        address sender
    ) external returns (bytes) {
        return theValue[epochIndex];
    }
}
