// SPDX-License-Identifier: MIT
// OpenZeppelin Contracts v4.4.1 (token/ERC20/IERC20.sol)

pragma solidity ^0.8.0;

import "./LibNativeCurrency.sol";

contract Native {
    function name() external view returns (string memory) {
        return LibNativeCurrency.name();
    }

    function balanceOf(address account) external view returns (uint256) {
        return LibNativeCurrency.balanceOf(account);
    }
}
