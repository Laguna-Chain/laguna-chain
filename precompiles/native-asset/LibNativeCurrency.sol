// SPDX-License-Identifier: MIT
// OpenZeppelin Contracts v4.4.1 (token/ERC20/IERC20.sol)

pragma solidity ^0.8.0;

library LibNativeCurrency {
    address private constant precompile =
        address(0x0000000000000000000000000000000000232a);

    function name() internal view returns (string memory) {
        (bool success, bytes memory returnData) = precompile.staticcall(
            abi.encodeWithSignature("name()")
        );
        assembly {
            if eq(success, 0) {
                revert(add(returnData, 0x20), returndatasize())
            }
        }

        return abi.decode(returnData, (string));
    }

    function balanceOf(address account) internal view returns (uint256) {
        (bool success, bytes memory returnData) = precompile.staticcall(
            abi.encodeWithSignature("balanceOf(address)", account)
        );
        assembly {
            if eq(success, 0) {
                revert(add(returnData, 0x20), returndatasize())
            }
        }

        return abi.decode(returnData, (uint256));
    }
}
