# Changelog

This changelog is based on [Keep A
Changelog](https://keepachangelog.com/en/1.1.0).

## v2.0.0

This version brings partial Conway support: all previously supported features
work with a Conway node, but no new Conway feature is added. These are planned
to be rolled out in a later minor version bump.

### Added

### Changed

- Updated plutus-ledger-api 3.0.1 (now shipping with it's own
  cardano-serialization-lib version)
- Added reference script fee calculation
- Breaking change: allowing multiple collaterals
  - allowing multiple UTxO in `CollateralStrategy::Explicit`
  - `CollateralStrategy::Automatic` will pick multiple UTxOs if necessary, `max_utxo_count`
    parameter added
  - `amount` renamed to `min_amount`

### Removed

- Plutip is discontinued and not compatible with Conway, we removed the client library

## v1.0.0

Babbage compatible first release
