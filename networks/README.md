Our networks configuration is structured like this:

## Networks

We define the following **networks**:

- **devnet**: what a developer runs _locally on their machine_ to simulate a full end-to-end network setup.
- **testnet**: what Union runs on their nodes in order to _test a mainnet-like environment_.
- **mainnet**: what runs on the public Union mainnet. _("the production environment")_

## Genesis

`genesis/` contains the genesis configurations for each _network_.

## Services

`services/` contains all of the **service-generating functions**. They are defined as Nix functions so that dependencies and network-specific configuration can be injected as needed for the network in which they are used. These functions are then included in [arion](https://docs.hercules-ci.com/arion/) specs.

## Arion

In `devnet.nix` we combine _Genesis configuration_ and _service-generating functions_ so that they are injected in an [arion](https://docs.hercules-ci.com/arion/) spec. Arion is a Nix wrapper around `docker-compose`. This allows us to create reproducible networks.

## Running from macOS through Linux Docker

The devnet Arion outputs are Linux-oriented. On macOS, run the devnet from a
Linux Nix container and mount the host Docker socket:

```sh
NO_BLOCKSCOUT=true ./networks/run-linux-devnet.sh
```

Apple Silicon defaults to `aarch64-linux`. To force the x86_64 path:

```sh
DEVNET_LINUX_SYSTEM=x86_64-linux NO_BLOCKSCOUT=true ./networks/run-linux-devnet.sh
```

The script starts `.#full-dev-setup` by default. Override with
`DEVNET_TARGET=devnet-eth` or another devnet app when needed.
