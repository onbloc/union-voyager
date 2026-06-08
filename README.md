# union-voyager

A fork of [Union](https://github.com/unionlabs/union)'s Voyager IBC relayer.

Voyager is a high-performance IBC relayer that operates as a PostgreSQL-backed state machine. All state is persisted in the database, allowing the relayer to resume exactly where it left off after a crash or restart.

> **Upstream**: [unionlabs/union](https://github.com/unionlabs/union)
> **Environment**: Instructions and scripts are written for **Amazon Linux**.

---

## Prerequisites

### 1. Nix

Builds are managed through Nix. Supported systems: **Linux (x86_64, aarch64)** and **macOS M-series (aarch64-darwin)**.

```bash
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install
```

**Configure Nix cache** (speeds up builds significantly)

```bash
echo "extra-substituters = https://cache.garnix.io" | sudo tee -a /etc/nix/nix.conf
echo "extra-trusted-public-keys = cache.garnix.io:CTFPyKSLcx5RMJKfLo5EEPUObbA78b0YQ2DTCJXqr9g=" | sudo tee -a /etc/nix/nix.conf
```

Verify:

```bash
nix show-config | grep substituters
```

### 2. PostgreSQL (Amazon Linux)

Voyager uses PostgreSQL as its work queue backend.

**Install and initialize**

```bash
sudo dnf install -y postgresql15 postgresql15-server
sudo postgresql-setup --initdb
sudo systemctl enable --now postgresql
```

**Configure authentication** (`/var/lib/pgsql/data/pg_hba.conf`)

Add or update the following line to enable password-based authentication:

```
host    all    all    127.0.0.1/32    md5
```

**Set password and restart**

```bash
sudo systemctl restart postgresql
sudo -u postgres psql -c "ALTER USER postgres WITH PASSWORD 'postgres';"
```

---

## Build

```bash
make build
```

Runs two nix builds in parallel.

| Target | Log |
|--------|-----|
| `voyager` | `voyager.log` |
| `voyager-modules-plugins` | `voyager-modules-plugins.log` |

Build artifacts are symlinked under `./result/`. To use `voyager` directly in your shell, add the binary path to your `PATH`:

```bash
export PATH=$PATH:$(pwd)/result/bin
```

---

## Configuration

Use `voyager/config.jsonc` as a reference. The minimum required fields are:

```jsonc
{
  "voyager": {
    "num_workers": 50,
    "queue": {
      "type": "pg-queue",
      "database_url": "postgres://postgres:postgres@127.0.0.1:5432/voyager"
    }
  },
  "modules": { ... },
  "plugins": [ ... ]
}
```

To view the full config schema:

```bash
voyager config schema
```

---

## Run

```bash
make run
```

Starts voyager in the background via `nohup`. Runtime logs are written to `voyager-run.log`.

```bash
tail -f voyager-run.log
```

---

## Indexing

Run the following commands to index each chain. Update the chain IDs to match your config.

```bash
voyager --config-file-path voyager/config.jsonc index <chain-id> -e
```

Example:

```bash
voyager --config-file-path voyager/config.jsonc index union-testnet-10 -e
voyager --config-file-path voyager/config.jsonc index 11155111 -e
voyager --config-file-path voyager/config.jsonc index dev.ibc -e
```

---

## Make Targets

```
make build   — start nix builds in the background (nohup, logs to *.log)
make init    — symlink nix store binaries to target/debug/ after build completes
make run     — start voyager in the background via nohup (logs to voyager-run.log)
make run-reset — truncate the voyager queue then start voyager
make stop      — kill the running voyager process
make help    — list available targets
```
