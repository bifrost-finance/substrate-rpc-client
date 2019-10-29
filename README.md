# substrate-rpc-client
is a library written in Rust for connecting to the substrate's RPC interface via WebSockets allowing to

* Compose extrinsics, send them and subscribe to updates.
* Watch events and execute code upon events.
* Parse and print the node metadata.
* Send sudo call.

## Prerequisites
In order to build the substrate-rpc-client, Rust is needed. For Linux/Mac:

    curl https://sh.rustup.rs -sSf | sh

For more information, please refer to the [substrate](https://github.com/paritytech/substrate) repository.

## Test with bifrost node

To run a bifrost node, check out this repo: [Bifrost](https://github.com/bifrost-codes/bifrost).
See the instructions to build and run a bifrost node, either native target or docker image.

After you run a bifrost successfully, run a test case.
```
cargo test test_balances_transfer -- --nocapture
```

You'll see Bob's balances is increased.

## Alternatives

Parity offers a Rust client with similar functionality: [substrate-subxt](https://github.com/paritytech/substrate-subxt)

## Tips

Only support V8 metadata now.
