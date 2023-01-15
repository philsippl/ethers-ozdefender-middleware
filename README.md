# 🧙‍♂️ ethers-ozrelayer-middleware

Implements a custom middleware for [ethers-rs](https://github.com/gakonst/ethers-rs) to send transactions through [OpenZeppelin Relayer](https://docs.openzeppelin.com/defender/relay).
It's using [cognito-srp-auth](https://github.com/lucdew/cognito-srp-auth) for cognito authentication under the hood.

Access tokens are stored in memory and refreshed when expired.

```
Sending transaction 0 (id: cf95ef53-2cec-4777-888f-xxxxxxxxxxxx) took 5.930298041s
Sending transaction 1 (id: a3d51318-dc55-4d2f-9510-xxxxxxxxxxxx) took 2.024535333s
Sending transaction 2 (id: 15be1111-bae6-4fcf-aa7f-xxxxxxxxxxxx) took 1.065668958s
Sending transaction 3 (id: ca57da12-8463-4017-ae52-xxxxxxxxxxxx) took 923.026583ms
Sending transaction 4 (id: 1d2b177e-dbb6-4fd7-8800-xxxxxxxxxxxx) took 781.0245ms
```