BlueR-Test - Code to test BlueR's functionality
===============================================
This is a temporary repository to test the functionality of [BlueR]. The repository will focus on system tests using code that controls Bluetooth (BR/EDR and BLE) adapters as opposed to a mocking interface. The tests will validate [BlueR] functionality over the air between multiple adapters. [Remoc] will be used to coordinate the multiple programs communicating through the adapters. 

 The first test will exercise the functionality of the advertisement capabilities of module bluer::adv by using code from le_advertise.rs to be the advertiser and scanner code pulled from gatt_client.rs to receive and verify the advertisements.

[BlueR]: https://github.com/bluez/bluer
[Remoc]: https://crates.io/crates/remoc