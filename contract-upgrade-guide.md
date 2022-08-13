# Ways to upgrade solang-compiled solidity contracts on Laguna chain

There are primarily three mechanisms to have upgradable contract support, namely - forward-calls, delegate-calls, and set-code.

## Forward calls

- In this approach, There is a proxy contract through which the users interact. The proxy contract stores the address of the main contract and forwards any call request that doesn't match its selector. 

- State is stored in the storage of the contract to which calls are forwarded.

- Flow diagram: \<User> ---> \<ProxyContract> ---> \<MainContract (Both Storage & Logic layer)>

- Process to upgrade the contract using forward call mechanism:
    1. Upload & Instantiate the updated-contract
    2. Call the existing contract method which replaces the main_contract address and does the storage migration

## Delegate calls

Solang currently doesn't support the delegatecall method. So openzeppelin proxy contracts are not useable at the moment. The Laguna chain team is looking at ways to support delegate calls in the future...

## Set code (Recommended way)

- It replaces the contract's bytecode stored on-chain with the new one. It doesn't alter the existing storage.

- The contract simply needs to call an external function (named `replace_code(byte32 code_hash)`) to upgrade the contract.

- [Things to keep in mind when modifying the storage structure](https://docs.openzeppelin.com/upgrades-plugins/1.x/writing-upgradeable#modifying-your-contracts)

- Process to upgrade the contract:
    1. Upload the updated-contract code on-chain. (Instantiation is not required)
    2. Note down the `code-hash` of the uploaded contract.
    3. Call the existing contract method which calls the `replace_code` function.