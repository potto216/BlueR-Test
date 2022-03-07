# BlueR-Test - Code to test BlueR's functionality

## Overview
This is a temporary repository to test the functionality of [BlueR]. The repository will focus on system tests using code that controls Bluetooth (BR/EDR and BLE) adapters as opposed to a mocking interface. The tests will validate [BlueR] functionality over the air between multiple adapters. [Remoc] will be used to coordinate the multiple programs communicating through the adapters. The current suite of tests are:

The name is randomly generated

 1. Check the address functionality of the client
 2. Exercise the functionality of the advertisement capabilities of module bluer::adv 
 
 These tests currently only run under Linux.

## Test Cases

### Test the basic advertise feature 

This tests the [BlueR] Advertisement struct to perform basic advertisements. 
This test assumes two bluetooth controllers are connected to a single Linux VM.


First startup the server software in one terminal that will look for clients requesting tests. The server manages the adapters. The server can be started with:
`cargo run -- -d server`

To test receiving a server address run:
`cargo run -- -d client server-address`

To test 
`cargo run -- -d client advertising-service-data`
`cargo run -- -d client advertising-service-uuids128`

USAGE:
    bluer-test client [OPTIONS] <SUBCOMMAND>

OPTIONS:
    -h, --help               Print help information
    -s, --server <SERVER>    Server hostname or IP address [default: localhost]

SUBCOMMANDS:
    advertising-service-data
    advertising-service-uuids128    Performs the advertising test
    advertising-service-uuids16
    help                            Print this message or the help of the given subcommand(s)
    server-address                  Prints the server's Bluetooth address


[BlueR]: https://github.com/bluez/bluer
[Remoc]: https://crates.io/crates/remoc