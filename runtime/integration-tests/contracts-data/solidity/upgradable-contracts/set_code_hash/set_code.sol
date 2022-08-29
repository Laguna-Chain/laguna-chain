// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

interface set_code {
    /**
     * Contracts can call this method to change their code in-place in the runtime.
     * 
     * @dev It doesn't change any storage items. For more details refer to
     * https://docs.openzeppelin.com/upgrades-plugins/writing-upgradeable#modifying-your-contracts
     */
    function replace_code(bytes32 code_hash) external returns (bool);

    /**
     * Returns the number of times the contract has been updated
     */
    function code_version(address contract_addr) external view returns (uint32);
}