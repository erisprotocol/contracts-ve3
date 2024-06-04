# ve(3,3)

## Strucuture

connector-alliance

- part 1 of Alliance Protocol Hub contracts
- contains logic for creating the VT coin to be staked and alliance delegation + claim mechanism

asset-staking

- part 2 of Alliance Protocol Hub contracts
- contains logic for staking and distributing rewards to "Asset (Token)" stakers.
- changes:
  - removed temp balance
  - disallow receiving funds
  - implement take rate from amp extractor

global-config

- Central place to store address information and access rights
- Structure based on Mars Address Provider, but simplified for usage of &str consts instead of enum.

## Mentions

This repository is based on multiple open source contracts available in Cosmos:

- Enterprise Protocol V1.1.0: <https://github.com/terra-money/enterprise-contracts/tree/version/1.1.0?tab=License-1-ov-file>

- Alliance Protocol: <https://github.com/terra-money/alliance-protocol/>
-
- White Whale Modifications on Alliance Protocol: <https://github.com/White-Whale-Defi-Platform/cw-alliance-hub>

- Global Config: https://github.com/mars-protocol/contracts/blob/master/contracts/address-provider/src/contract.rs

- ERIS Amp Extractor
