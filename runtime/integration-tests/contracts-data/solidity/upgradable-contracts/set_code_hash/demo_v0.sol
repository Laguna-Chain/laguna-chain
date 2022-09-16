// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./set_code.sol";

contract demo_v0 {
    address admin;
    uint public value;
    address public constant SET_CODE = address(0x01);

    modifier onlyAdmin() {
        require(msg.sender == admin, "Not Authorised");
        _;
    }

    constructor() {
        admin = msg.sender;
    }

    function set_value(uint new_val) external {
        value = new_val;
    }

    function add_a_number() external view returns (uint) {
        return value + 5;
    }

    function upgrade_contract(bytes32 code_hash) external onlyAdmin returns(bool) {
        return set_code(SET_CODE).replace_code(code_hash);
    }
}