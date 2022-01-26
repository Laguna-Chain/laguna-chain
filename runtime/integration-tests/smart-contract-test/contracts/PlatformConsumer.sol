// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

import "./LibPlatform.sol";

contract Native {
    function name() public view returns (string memory) {
        return NativeCurrency.name();
    }

    function balanceOf(address account) public view returns (uint256) {
        return NativeCurrency.balanceOf(account);
    }
}
