# Union devnet.nix Memory Optimization Analysis

## Scope

This analyzes `/Users/notjoon/union/networks/devnet.nix` for a minimal-memory local E2E setup covering:

- `devnet-union`
- `devnet-eth` (`geth`, `forge`, `lodestar`, optional Blockscout stack)
- `postgres`

The current Docker/Arion service definitions do not set cgroup memory limits such as `mem_limit` or `deploy.resources`. Values below are configuration-based estimates because the Union devnet containers were not running when inspected.

## Current Configuration

`full-dev-setup` is composed from:

- `services.devnet-eth`
- `services.devnet-union`
- `services.postgres`

Other Cosmos devnets in the same file (`stargaze`, `osmosis`, `simd`, `union-v1`) are not part of `full-dev-setup`.

### Component Breakdown

| Component | Current setting | Estimated memory |
| --- | ---: | ---: |
| Union validators | `validatorCount = 4` | 1.6-3.2GB |
| geth | full sync, archive GC, debug APIs, WS/Auth RPC | 0.7-1.5GB |
| lodestar | `genesisValidators = 128`, `startValidators = 0..127` | 1.0-2.5GB |
| forge | one-shot contract deploy container | peak 0.5-1.0GB |
| postgres | `shared_buffers=1024MB`, `effective_cache_size=2048MB` | 1.1-1.5GB |
| Blockscout stack | enabled on x86_64 unless `NO_BLOCKSCOUT` is set | 1.5-3.5GB |

Estimated current total:

- Steady state: 5.9-12.2GB
- With forge deploy peak: 6.4-13.2GB

NixOS E2E wrappers add VM-level ceilings:

- `devnetEthNode`: 8GB
- `unionNode`: 4GB
- `voyagerNode`: 8GB

Local `nix run .#full-dev-setup` uses host Docker and does not get those VM ceilings.

## Optimization Options

### 1. Reduce Union Validators

Change:

```nix
devnet-union = mkCosmosDevnet {
  # ...
  validatorCount = 1;
};
```

Expected saving:

- `4 -> 2`: about 0.8-1.6GB
- `4 -> 1`: about 1.2-2.4GB

For basic IBC packet flow, one validator is enough. It still produces blocks and proofs. Keep four validators only for validator-set churn, jailing, threshold, or rotation behavior.

### 2. Disable Blockscout

Use the existing gate:

```bash
NO_BLOCKSCOUT=true nix run .#full-dev-setup
```

This removes:

- `blockscout-backend`
- `blockscout-frontend`
- `blockscout-sc-verifier`
- `blockscout-db`
- `blockscout-redis`
- `blockscout-sig-provider`
- `blockscout-stats-db`
- `blockscout-stats`
- `blockscout-visualizer`
- `blockscout-proxy`

Expected saving: 1.5-3.5GB.

Trade-off: no explorer UI, no Blockscout API, no contract verification. The forge deploy script already omits `--verify --verifier blockscout` when `NO_BLOCKSCOUT` is set.

### 3. Shrink Postgres

Current:

```nix
command = "postgres -c shared_buffers=1024MB -c effective_cache_size=2048MB";
```

Minimal local E2E setting:

```nix
command = "postgres -c shared_buffers=128MB -c effective_cache_size=256MB";
```

Expected saving: about 0.8-0.9GB.

Trade-off: worse query/cache performance for Voyager queue workloads. This is fine for small local packet tests.

### 4. Reduce Lodestar Validator Count

Current source:

```nix
devnetConfig = {
  validatorCount = 4;
  ethereum = {
    beacon = {
      validatorCount = 128;
    };
  };
};
```

Minimal setting:

```nix
devnetConfig.ethereum.beacon.validatorCount = 1;
```

If Lodestar dev mode or tests become flaky with one validator, use `8` as the practical fallback.

Expected saving: about 0.2-0.8GB.

Trade-off: less realistic beacon validator topology. This should not matter for local ZKGM packet flow as long as the beacon API advances slots and geth finalization paths remain usable.

### 5. Forge

`forge` is required only to deploy EVM contracts into the devnet. It is not a steady-state chain component.

Minimum options:

- Keep it for automatic setup, accept the short deploy-time memory peak.
- Remove it only if contracts are pre-deployed and tests are pointed at existing addresses.

Expected saving:

- Steady state: usually none after it exits.
- Startup peak: about 0.5-1.0GB.

### 6. Geth and Lodestar

For Union EVM E2E, keep both:

- `geth` provides execution RPC, WS events, engine/auth RPC, and contract execution.
- `lodestar` provides beacon REST API and slot/head progression used by tests and EVM light-client/state flows.

Removing Lodestar is not a safe default because E2E code waits on `http://...:9596/eth/v2/beacon/blocks/head`.

## Minimal E2E Configuration

Minimum component list:

- `union-0`
- `geth`
- `lodestar`
- `forge` for deployment, or pre-deployed contracts
- `voyager`
- small `postgres`

Recommended minimal settings:

```nix
# networks/devnet.nix
devnet-union = mkCosmosDevnet {
  # ...
  validatorCount = 1;
};

# flake.nix
devnetConfig = {
  validatorCount = 4;
  ethereum = {
    beacon = {
      validatorCount = 1; # use 8 if Lodestar dev mode is flaky
    };
  };
};

# networks/services/postgres.nix
command = "postgres -c shared_buffers=128MB -c effective_cache_size=256MB";
```

Run with:

```bash
NO_BLOCKSCOUT=true nix run .#full-dev-setup
```

Estimated minimal total:

| Component | Estimated memory |
| --- | ---: |
| Union validator x1 | 0.4-0.8GB |
| geth | 0.7-1.5GB |
| lodestar, reduced validators | 0.8-1.7GB |
| postgres, reduced buffers | 0.2-0.5GB |
| forge deploy peak | 0.5-1.0GB |
| Voyager + queue, if included | 0.8-2.0GB |

Estimated total:

- Without Voyager steady state: 2.1-4.5GB
- With Voyager steady state: 2.9-6.5GB
- Startup peak with forge: add 0.5-1.0GB

## Recommendations

### 16GB Machine

Use:

- `NO_BLOCKSCOUT=true`
- Union `validatorCount = 1`
- Lodestar beacon validator count `1`, fallback `8`
- Postgres `shared_buffers=128MB`

Expected usable range:

- About 3-7.5GB for the E2E stack depending on Voyager and deploy peak.

### 32GB Machine

Use:

- `NO_BLOCKSCOUT=true`
- Keep Union `validatorCount = 4` if validator behavior matters
- Otherwise use `validatorCount = 1`
- Postgres can stay large unless the machine is shared with other stacks

Expected usable range:

- About 5-9GB without Blockscout.
- About 7-12GB with Blockscout.

## Trade-offs

| Change | Lost capability |
| --- | --- |
| Union validators `4 -> 1` | Validator-set rotation, jailing/threshold realism, multi-validator failure tests |
| `NO_BLOCKSCOUT=true` | Explorer UI, Blockscout API, contract verification |
| Postgres buffers reduced | Lower queue/query throughput |
| Lodestar validators reduced | Less realistic beacon topology |
| Forge removed | No automatic EVM contract deployment unless addresses are pre-provided |

## Short Recommendation

The first configuration to try is:

```bash
NO_BLOCKSCOUT=true nix run .#full-dev-setup
```

Then reduce `devnet-union.validatorCount` from `4` to `1`. If memory is still tight, shrink Postgres buffers and reduce `devnetConfig.ethereum.beacon.validatorCount`.
