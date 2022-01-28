// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

library NativeCurrency {
    address public constant precompile =
        address(0x000000000000000000000000000000000000232a);

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

    function transfer(address recipient, uint256 amount)
        internal
        view
        returns (bool)
    {
        (bool success, bytes memory returnData) = precompile.staticcall(
            abi.encodeWithSignature(
                "transfer(address,address,uint256)",
                msg.sender,
                recipient,
                amount
            )
        );
        assembly {
            if eq(success, 0) {
                revert(add(returnData, 0x20), returndatasize())
            }
        }

        return abi.decode(returnData, (bool));
    }

    function totalSupply() internal view returns (uint256) {
        (bool success, bytes memory returnData) = precompile.staticcall(
            abi.encodeWithSignature("totalSupply()")
        );
        assembly {
            if eq(success, 0) {
                revert(add(returnData, 0x20), returndatasize())
            }
        }

        return abi.decode(returnData, (uint256));
    }
}
