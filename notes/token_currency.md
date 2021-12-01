# Token and Currency

We will focus on the relationship between `pallet-balance`, `orml-tokens` and `orml-currencies`:

1. `pallet-balance` is the underlying assets native to the chain, it provides setting balance and transfering between accounts
2. `orml-tokens` is a pallet specific for defining distinquished token types, ranging from native to external, or even utility tokens such as liquidity-pool token
3. `orml-currencies` provides wrapper for assets so that `pallet-balances` and `orml-tokens` can all be exposed as currency therefore allowed to do transaction and other currency related behaviour
