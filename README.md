# BlueR-Test - Code to test BlueR's functionality

## Overview
This is a temporary repository to test the functionality of [BlueR]. The repository will focus on system tests using code that controls Bluetooth (BR/EDR and BLE) adapters as opposed to a mocking interface. The tests will validate [BlueR] functionality over the air between multiple adapters. [Remoc] will be used to coordinate the multiple programs communicating through the adapters. 

 The first test will exercise the functionality of the advertisement capabilities of module bluer::adv by using code from le_advertise.rs to be the advertiser and scanner code pulled from gatt_client.rs to receive and verify the advertisements.

 These tests currently only run under Linux.

Please post issues of new tests and how to write more idiomatic Rust  .


## Test Cases

### Test the basic advertise feature 

This tests the [BlueR] Advertisement struct to perform basic advertisements. 
This test assumes two bluetooth controllers are connected to a single Linux VM.

First startup the scan software in one terminal that will look for advertisements (ex: 5C:F3:70:A1:71:0F) by scanning with a second controller (ex: 5C:F3:70:7B:F5:66).

`./target/debug/le_scan  -a 5C:F3:70:A1:71:0F -s 5C:F3:70:7B:F5:66 -d`

Now start the advertiser in a second terminal.

`./target/debug/le_advertise  -a 5C:F3:70:A1:71:0F -d`

The scanning software will exit when it detects the advertiser.

### Test the basic advertise feature with a service UUID

This tests the [BlueR] Advertisement struct to advertise with a service UUID.
This test assumes two bluetooth controllers are connected to a single Linux VM.

First startup the scan software in one terminal that will look for advertisements (ex: 5C:F3:70:A1:71:0F) with a service UUID (ex: 123e4567-e89b-12d3-a456-426614174000) by scanning with a second controller (ex: 5C:F3:70:7B:F5:66) 

`./target/debug/le_scan  -a 5C:F3:70:A1:71:0F -s 5C:F3:70:7B:F5:66 -u 123e4567-e89b-12d3-a456-426614174000 -d`

Now start the advertising the service UUID 123e4567-e89b-12d3-a456-426614174000

`./target/debug/le_advertise  -a 5C:F3:70:A1:71:0F -u 123e4567-e89b-12d3-a456-426614174000 -d`

The scanning software will exit when it detects the advertiser. It will print whether the correct service UUID was detected.

[BlueR]: https://github.com/bluez/bluer
[Remoc]: https://crates.io/crates/remoc