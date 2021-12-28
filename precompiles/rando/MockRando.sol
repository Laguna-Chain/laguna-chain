// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.0;

import "./IRando.sol";

// a concrete impl for the onchain precompile module that resides at the given address
//
// call order:
// 1. the precompile impl of target module resides at the address when compiled
// 2. when calling at the precompile evm address, evm try to intercept precompile address first
// 3. if matching precompile exist, precompile try to parse the input as function selector, with the same interface with IRando
// 4. contract develeper can use this predeploy contract to interact with onchain pallets
contract Rando is IRando {
    // the address of the precompile module
    address private constant precompile =
        address(0x0000000000000000000000000000000000000001);

    function call_rando() public view override {
        (bool success, bytes memory returnData) = precompile.staticcall(
            abi.encodeWithSignature("call_rando()")
        );
        assembly {
            if eq(success, 0) {
                revert(add(returnData, 0x20), returndatasize())
            }
        }
    }

    function get_count() public view override {
        (bool success, bytes memory returnData) = precompile.staticcall(
            abi.encodeWithSignature("get_count()")
        );
        assembly {
            if eq(success, 0) {
                revert(add(returnData, 0x20), returndatasize())
            }
        }
    }
}
