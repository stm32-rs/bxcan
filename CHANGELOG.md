# Changelog

## Unreleased

No changes.

## [0.2.3 - 2020-12-09](https://github.com/jonas-schievink/bxcan/releases/tag/v0.2.3)

### Fixes

* Fix a panic when aborting transmission of a lower-priority frame.
* Fix comparison when checking for a lower-priority mailbox.

## [0.2.2 - 2020-12-05](https://github.com/jonas-schievink/bxcan/releases/tag/v0.2.2)

### New Features

* Add `Can::is_transmitter_idle`.

## [0.2.1 - 2020-12-05](https://github.com/jonas-schievink/bxcan/releases/tag/v0.2.1)

### Breaking Changes

* Update `SlaveFilters::enable_bank` to also take `impl Into<T>`.

## [0.2.0 - 2020-12-05](https://github.com/jonas-schievink/bxcan/releases/tag/v0.2.0)

### New Features

* Add associated constants for highest/lowest CAN IDs.

### Fixes

* Update bank count when changing filter bank split.
* Fix filter bank logic and document the expected behavior.
* Fix filter accesses for the slave peripheral.
* Fix DLC range check in `Frame::new_remote`.
* Fix `PartialEq` implementation of `Frame`.

### Breaking Changes

* Change some APIs to accept `impl Into<T>` arguments to improve ergonomics.
* Rename some filter methods to clarify their meaning.
* Remove `MasterInstance::Slave` associated type.

## [0.1.0 - 2020-12-01](https://github.com/jonas-schievink/bxcan/releases/tag/v0.1.0)

Initial release.
