# BlueR-Test - Code to test BlueR's functionality

## Overview
This is a temporary repository to test the functionality of [BlueR]. The repository will focus on system tests using code that controls Bluetooth (BR/EDR and BLE) adapters as opposed to a mocking interface. The tests will validate [BlueR] functionality over the air between multiple adapters. [Remoc] will be used to coordinate the multiple programs communicating through the adapters. The testing framework is based on a client server architecture. A server is started which listens for clients to request tests. Clients then connect to the server using [Remoc] and perform individual tests.

All tests assumes two bluetooth controllers are connected to a single Linux host. Additionally all tests require the server to be started before the clients. 

To use the framework, first start the server software in a terminal. This will look for clients requesting tests. The server manages the adapters. The server can be started with:
`cargo run -- -v server`

Next in another terminal start a client. A client will perform one test of [BlueR]'s functionality and exit.

When done with the tests the server is stopped from the client using the command:
`cargo run -- -v client kill-server`

## Setup
Setting up the system may require restarting the bluetooth driver. To do that first stop the bluetooth driver with:
`sudo systemctl stop bluetooth`
and then restart it with
 `sudo systemctl restart bluetooth`

To setup monitoring of the dbus run: 
`sudo dbus-monitor --system "sender='org.bluez'"`
and to capture the host interface traffic to the bluetooth device run
`sudo btmon -w filename_of_capture.log`

## Test Cases
For all tests the name of the device is randomly generated. These tests currently only run under Linux. The current suite of tests are:

### Test the ability to detect a server advertising
In this test the client receives the server address over [Remoc] and then looks for an advertisement from that address
To test receiving a server address run:
`cargo run -- -v client server-address`

### Exercise the functionality of the advertisement capabilities of module bluer::adv 
These tests verify that the server BLE advertisements are populated with the correct information.
To test sending service uuids or data in the advertisement
`cargo run -- -v client advertising-service-data`
`cargo run -- -v client advertising-service-uuids128`
`cargo run -- -v client advertising-service-uuids16`

### Exercise the functionality of the local and remote GATT services capabilities of module bluer::gatt
This test verifies that the GATT server is populated correctly and can be read correctly.
`cargo run -- -v client gatt-server`

### Logging of tests
Test report generation can be genreated from the event driven diagnostics captured using Tokio's [tracing] library.
`export RUST_LOG=info`
The log file name is in the format: `prefix.log.yyyy-mm-dd-hh` where yyyy is the current year, mm is the current month, and hh the current day.
The log path can be specified with `-l <PATH_FOR_LOG_FILES>

### The general usage is
USAGE:
    bluer-test client [OPTIONS] <SUBCOMMAND>

OPTIONS:
    -h, --help               Print help information
    -v, --verbose-mode
    -l, --log-path <PATH_FOR_LOG_FILES>
    -s, --server <SERVER>    Server hostname or IP address [default: localhost]

SUBCOMMANDS:
    gatt-server                     Sets up a GATT server and reads data from it
    advertising-service-data        Performs the advertising test with service data
    advertising-service-uuids128    Performs the advertising test with 128 bit UUIDs
    advertising-service-uuids16     Performs the advertising test with 16 bit SIG UUIDs 
    server-address                  Prints the server's Bluetooth address  
    help                            Print this message or the help of the given subcomm
    kill-server                     Kills the server side software

[BlueR]: https://github.com/bluez/bluer
[Remoc]: https://crates.io/crates/remoc
[tracing]: https://crates.io/crates/tracing