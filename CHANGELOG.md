# Changelog

## Unreleased

### New Features

* Add transmit function that returns the mailbox number, and transmit abort function ([#25]).

### Fixes

* The `Can::enable_interrupt` and `Can::disable_interrupt` functions now manipulate the correct bits in the interrupt
  enable register ([#28]).

[#25]: https://github.com/stm32-rs/bxcan/pull/25
[#28]: https://github.com/stm32-rs/bxcan/pull/28

## [0.5.0 - 2021-03-15](https://github.com/stm32-rs/bxcan/releases/tag/v0.5.0)

### Breaking Changes

* Update to defmt 0.2.0 ([#17]).

[#17]: https://github.com/stm32-rs/bxcan/pull/17

## [0.4.0 - 2021-01-23](https://github.com/stm32-rs/bxcan/releases/tag/v0.4.0)

### Breaking Changes

* Revamp filter and configuration API to allow method chaining ([#10] [#12]).

### Bugfixes

* Wait for SLAK and INAK bits when changing mode ([#8]).

[#8]: https://github.com/stm32-rs/bxcan/pull/8
[#10]: https://github.com/stm32-rs/bxcan/pull/10
[#12]: https://github.com/stm32-rs/bxcan/pull/12

### Misc

* Clarify comments for the `transmit()` method ([#9]).

[#9]: https://github.com/stm32-rs/bxcan/pull/9

## [0.3.0 - 2020-12-28](https://github.com/stm32-rs/bxcan/releases/tag/v0.3.0)

### New Features

* Configurable mask for masked filters.
* Implement the `embedded-can` traits.

### Breaking Changes

* Changes to masked filters required some breaking API changes.

## [0.2.3 - 2020-12-09](https://github.com/stm32-rs/bxcan/releases/tag/v0.2.3)

### Fixes

* Fix a panic when aborting transmission of a lower-priority frame.
* Fix comparison when checking for a lower-priority mailbox.

## [0.2.2 - 2020-12-05](https://github.com/stm32-rs/bxcan/releases/tag/v0.2.2)

### New Features

* Add `Can::is_transmitter_idle`.

## [0.2.1 - 2020-12-05](https://github.com/stm32-rs/bxcan/releases/tag/v0.2.1)

### Breaking Changes

* Update `SlaveFilters::enable_bank` to also take `impl Into<T>`.

## [0.2.0 - 2020-12-05](https://github.com/stm32-rs/bxcan/releases/tag/v0.2.0)

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

## [0.1.0 - 2020-12-01](https://github.com/stm32-rs/bxcan/releases/tag/v0.1.0)

Initial release.
