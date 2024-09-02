# Phoenix Directive Treasury

This contract allows staking of a virtual token in Terra Governance. It supports the following features:

- Alliance Staking
- DCA swap (with max amount per execution, and cooldown period)
- OTC swap
- Milestone payments
- Vesting payments
- One-time Payments (with schedules)
- Veto anything at any time, payments delayed if veto mandatory
- Veto is possible both on per spend and per 30d of spendings.
- Basic on-chain price oracles (using USDC, Pool, or Router)

Queries:

- Query all actions per user - descending (user that is able to claim)
- Query all actions - ascending
- Query available balances (excludes reserved balances)
