# Changelog

## Unreleased

No changes.

## [0.8.0 - 2024-09-17](https://github.com/stm32-rs/bxcan/releases/tag/v0.8.0)

### Fixes

* Mask out all reserved bits in `set_bit_timing` before writing the register.

### Other Changes

* Update the embedded hal dependency to use the new embedded-can crate instead.

## [0.7.0 - 2022-05-30](https://github.com/stm32-rs/bxcan/releases/tag/v0.7.0)

### New Features

* [*breaking change*] Add support for using the second RX FIFO.
  * The `Rx` type has been renamed to `Rx0`, and an `Rx1` type has been introduced that accesses the second FIFO.
  * `enable_bank` now takes the `Fifo` the filter should be assigned to as an additional argument.
* Implement the `embedded-hal` 0.2.7 CAN traits.

### Other Changes

* [*breaking change*] Removed the `embedded-can-03` feature as the `embedded-can` crate is deprecated.
* [*breaking change*] Use a new `OverrunError` type as the receive error instead of `()`.

## [0.6.2 - 2021-11-15](https://github.com/stm32-rs/bxcan/releases/tag/v0.6.2)

### Fixes

* Enter initialization mode when constructing a `CanBuilder` ([#49]).

[#49]: https://github.com/stm32-rs/bxcan/pull/49

## [0.6.1 - 2021-11-15](https://github.com/stm32-rs/bxcan/releases/tag/v0.6.1)

### New Features

* Updated to defmt 0.3.0 ([#47]).

[#47]: https://github.com/stm32-rs/bxcan/pull/47

## [0.6.0 - 2021-09-05](https://github.com/stm32-rs/bxcan/releases/tag/v0.6.0)

### New Features

* Add `CanConfig::set_automatic_retransmit` function to enable or disable automatic frame retransmission ([#42]).
* [*breaking change*] Remove `transmit_and_get_mailbox` in favor of an improved `transmit` method ([#40]).
  * `Can::transmit` now returns a `TransmitStatus` struct, which contains the dequeued frame and
    the mailbox the new frame was placed into.
* [*breaking change*] Make `CanConfig` harder to misuse ([#37]):
  * Methods now take `self` by value.
  * The `CanConfig` struct is now `#[must_use]`.
  * `CanConfig` leaves init mode on drop, and enables the peripheral when `.enable()` is called.
  * These changes make it very hard to forget to enable the peripheral after configuring, which was
    a common mistake in previous versions.
* [*breaking change*] Replace `Can::new` with `Can::builder`, which makes it harder to forget enabling the peripheral ([#46]).

### Other Changes

* [*breaking change*] Make `Can::clear_sleep_interrupt` and `Can::clear_wakeup_interrupt` take `&self` instead of `&mut self`.
* [*breaking change*] Gate `embedded_can` impls behind the `embedded-can-03` Cargo feature.
* [*breaking change*] Gate defmt support behind the `defmt` Cargo feature.
* [*breaking change*] Removed `Can::configure` in favor of `Can::modify_config` ([#36]).

[#36]: https://github.com/stm32-rs/bxcan/pull/36
[#37]: https://github.com/stm32-rs/bxcan/pull/37
[#40]: https://github.com/stm32-rs/bxcan/pull/40
[#42]: https://github.com/stm32-rs/bxcan/pull/42
[#46]: https://github.com/stm32-rs/bxcan/pull/46

## [0.5.1 - 2021-05-15](https://github.com/stm32-rs/bxcan/releases/tag/v0.5.1)

### New Features

* Add transmit function that returns the mailbox number, and transmit abort function ([#25]).
* Add more methods to acknowledge interrupts ([#30]).
* Add `Can::free`, a way to get back ownership of the raw peripheral ([#33]).

### Fixes

* The `Can::enable_interrupt` and `Can::disable_interrupt` functions now manipulate the correct bits in the interrupt
  enable register ([#29]).

### Misc

* Improve documentation of interrupts ([#30]).

[#25]: https://github.com/stm32-rs/bxcan/pull/25
[#29]: https://github.com/stm32-rs/bxcan/pull/29
[#30]: https://github.com/stm32-rs/bxcan/pull/30
[#33]: https://github.com/stm32-rs/bxcan/pull/33

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
