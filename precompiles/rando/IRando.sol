// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.0;

// compatible interface for the precompile to parse and sent calls to the underlying extrinsics
interface IRando {
    function call_rando() external;

    function get_count() external returns (uint256);
}
