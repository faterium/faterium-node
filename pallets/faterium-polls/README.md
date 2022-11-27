# Faterium Polls implementation as Substrate Pallet

This is the official implementation of Faterium Crowdfunding Polls in Rust as Substrate FRAME-based pallet.

Read more about it on the official [grant application page](https://github.com/w3f/Grants-Program/blob/master/applications/faterium.md).

## Terminology

- **Faterium Polls:** A way for authors to decide what community wants and raise money for a project or idea.
- **Poll Rewards:** Any kind of reward that users can receive after winning the poll. It can be either a unique NFTs or any gifts personally from the author of the poll.
- **Pot:** Unspent funds accumulated by the treasury module.
- **Beneficiary:** An account who will receive the funds from a winner option of poll after its end.
- **Stake:** Funds that a voter lock when making a vote on a poll. The deposit can be returned to voter if poll's option lost or if author cancelled his poll, otherwise some percentage of funds will be given to author.

## Related Modules

- [Democracy](https://docs.rs/pallet-democracy/latest/pallet_democracy/)
- [Assets](https://docs.rs/pallet-assets/latest/pallet_assets/)
- [Treasury](https://docs.rs/pallet-treasury/latest/pallet_treasury/)
- [System](https://docs.rs/frame-system/latest/frame_system/)
- [Support](https://docs.rs/frame-support/latest/frame_support/)

## Legal

License: [Apache-2.0](../../LICENSE)
