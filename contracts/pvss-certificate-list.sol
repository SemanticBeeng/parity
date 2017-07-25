pragma solidity ^0.4.6;

contract PvssCertificateList {
    mapping (uint64 => uint64) theValue;

    function saveCommitment(
        uint64 epochIndex,
        uint64 commitment
    ) external {
        theValue[epochIndex] = commitment;
    }

    function getCommitment(
        uint64 epochIndex,
        address sender
    ) external returns (uint64) {
        return theValue[epochIndex];
    }
}

