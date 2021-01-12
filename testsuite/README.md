# On-Device bxCAN Testsuite

This is a small hardware-in-the-loop testsuite powered by [`defmt-test`]. It was
made for STM32F105 MCUs with 2 CAN peripherals.

Specifically, the testsuite was written to work with the "CAN filter" boards
described in [this article][can-filter-article], and it makes the following
assumptions:

* Both CAN1 and CAN2 are connected to the same CAN bus, with no interfering
  devices on the bus.
* CAN1 is connected to pins PA11 and PA12.
* CAN2 is connected to pins PB5 and PB6 (a non-default remapping).

[`defmt-test`]: https://crates.io/crates/defmt-test
[can-filter-article]: https://dangerouspayload.com/2020/03/10/hacking-a-mileage-manipulator-can-bus-filter-device/
