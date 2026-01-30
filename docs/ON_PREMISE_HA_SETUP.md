# On-Premise High Availability Setup Guide

## Overview

This guide covers deploying the Amsterdam Bike Fleet application with **99.99% uptime** (52 minutes downtime/year) on your own infrastructure.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     COMPLETE ARCHITECTURE                                    │
│                                                                              │
│   ┌───────────────────────────────────────────────────────────────────┐     │
│   │                    TAURI DESKTOP APP                              │     │
│   │               (Built with --features postgres)                    │     │
│   └────────────────────────────┬──────────────────────────────────────┘     │
│                                │                                            │
│                                ▼                                            │
│   ┌───────────────────────────────────────────────────────────────────┐     │
│   │                    HAProxy VIP (10.0.0.100)                       │     │
│   │                    Port 5432 (write) / 5433 (read)                │     │
│   └────────────────────────────┬──────────────────────────────────────┘     │
│                                │                                            │
│        ┌───────────────────────┼───────────────────────┐                    │
│        │                       │                       │                    │
│        ▼                       ▼                       ▼                    │
│   ┌─────────────┐        ┌─────────────┐        ┌─────────────┐            │
│   │ PostgreSQL  │        │ PostgreSQL  │        │ PostgreSQL  │            │
│   │   Node 1    │◄──────►│   Node 2    │◄──────►│   Node 3    │            │
│   │  (PRIMARY)  │  sync  │  (REPLICA)  │  async │  (REPLICA)  │            │
│   │             │        │             │        │             │            │
│   │ + Patroni   │        │ + Patroni   │        │ + Patroni   │            │
│   │ + pgBackRest│        │ + pgBackRest│        │ + pgBackRest│            │
│   └──────┬──────┘        └──────┬──────┘        └──────┬──────┘            │
│          │                      │                      │                    │
│          └──────────────────────┼──────────────────────┘                    │
│                                 │                                           │
│                                 ▼                                           │
│   ┌───────────────────────────────────────────────────────────────────┐     │
│   │                      ETCD Cluster (3 nodes)                       │     │
│   │                    (Leader Election & Config)                     │     │
│   └───────────────────────────────────────────────────────────────────┘     │
│                                 │                                           │
│                                 ▼                                           │
│   ┌───────────────────────────────────────────────────────────────────┐     │
│   │                    pgBackRest Repository                          │     │
│   │                  (NFS/SAN - Backup Storage)                       │     │
│   │                                                                   │     │
│   │    • Full backups (weekly)                                        │     │
│   │    • Differential backups (daily)                                 │     │
│   │    • WAL archive (continuous, ~0 RPO)                             │     │
│   └───────────────────────────────────────────────────────────────────┘     │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Quick Start

### Step 1: Deploy PostgreSQL HA Cluster

Follow [POSTGRESQL_HA_DEPLOYMENT.md](./POSTGRESQL_HA_DEPLOYMENT.md) to deploy:
- 3-node PostgreSQL cluster with Patroni
- 3-node etcd cluster for consensus
- HAProxy for load balancing and automatic failover
- pgBackRest for backups

**Minimum hardware:**
| Component | CPU | RAM | Storage | Quantity |
|-----------|-----|-----|---------|----------|
| PostgreSQL nodes | 8 cores | 32 GB | 500 GB NVMe | 3 |
| etcd nodes | 2 cores | 4 GB | 50 GB SSD | 3 (can colocate) |
| HAProxy | 2 cores | 4 GB | 20 GB | 2 |
| Backup storage | - | - | 2 TB+ | 1 |

### Step 2: Configure Backups

Follow [BACKUP_RECOVERY.md](./BACKUP_RECOVERY.md) to configure:
- Automated backup schedule (full weekly, differential daily)
- WAL archiving for point-in-time recovery
- Backup verification and monitoring

**Recovery targets:**
| Metric | Target |
|--------|--------|
| RPO (data loss) | < 1 minute |
| RTO (recovery time) | < 1 hour |
| Failover time | < 30 seconds |

### Step 3: Build Tauri App with PostgreSQL

```bash
# Build for PostgreSQL (instead of default SQLite)
cd src-tauri
cargo build --release --no-default-features --features postgres
```

### Step 4: Configure App Environment

Set environment variables before running the app:

```bash
# Point to HAProxy VIP (not individual PostgreSQL nodes)
export PG_HOST=10.0.0.100
export PG_PORT=5432
export PG_USER=fleet_app
export PG_PASSWORD=your_secure_password
export PG_DATABASE=bike_fleet
export PG_POOL_SIZE=16

# Run the app
./amsterdam-bike-fleet
```

---

## Database Connection Flow

```
App Start
    │
    ▼
init_database() called
    │
    ▼
Read PG_HOST, PG_PORT, PG_USER, PG_PASSWORD from env
    │
    ▼
Create connection pool (deadpool-postgres)
    │
    ▼
Connect to HAProxy VIP (10.0.0.100:5432)
    │
    ▼
HAProxy routes to current PRIMARY
    │
    ▼
Initialize schema if needed (CREATE TABLE IF NOT EXISTS)
    │
    ▼
Seed mock data if empty
    │
    ▼
App ready!
```

---

## Failover Behavior

### What happens when a PostgreSQL node fails?

1. **Patroni detects failure** (~10 seconds)
2. **Patroni promotes sync replica to primary** (~5 seconds)
3. **HAProxy health check detects new primary** (~5 seconds)
4. **HAProxy routes traffic to new primary**
5. **App reconnects automatically** (connection pool handles this)

**Total failover time: < 30 seconds**

### What the app sees:

```
Normal operation
    │
    ▼
Primary fails
    │
    ▼
Next query fails (connection error)
    │
    ▼
Connection pool retries with new connection
    │
    ▼
HAProxy routes to new primary
    │
    ▼
Query succeeds
    │
    ▼
Normal operation resumes
```

The app doesn't need special failover logic — the infrastructure handles it.

---

## Monitoring

### Check cluster health:

```bash
# On any PostgreSQL node
patronictl -c /etc/patroni/patroni.yml list

# Expected output:
# + Cluster: bike-fleet-cluster ----+---------+---------+----+-----------+
# | Member    | Host       | Role    | State   | TL | Lag in MB |
# +-----------+------------+---------+---------+----+-----------+
# | pg-node-1 | 10.0.0.11  | Leader  | running |  1 |           |
# | pg-node-2 | 10.0.0.12  | Replica | running |  1 |         0 |
# | pg-node-3 | 10.0.0.13  | Replica | running |  1 |         0 |
# +-----------+------------+---------+---------+----+-----------+
```

### Check backup status:

```bash
pgbackrest --stanza=bike-fleet info
```

### HAProxy stats:

```
http://10.0.0.100:8404/stats
```

---

## File Summary

| File | Purpose |
|------|---------|
| [POSTGRESQL_HA_DEPLOYMENT.md](./POSTGRESQL_HA_DEPLOYMENT.md) | Full cluster deployment guide (Docker + Ansible) |
| [BACKUP_RECOVERY.md](./BACKUP_RECOVERY.md) | Backup strategy and recovery procedures |
| [src-tauri/src/database_pg.rs](../src-tauri/src/database_pg.rs) | PostgreSQL database module |
| [src-tauri/src/commands/*_pg.rs](../src-tauri/src/commands/) | PostgreSQL Tauri commands |

---

## Building for Different Backends

```bash
# SQLite (default) - for standalone desktop use
cargo build --release

# PostgreSQL - for on-premise HA
cargo build --release --no-default-features --features postgres

# Both features cannot be enabled simultaneously
```

---

## Uptime Calculation

For 99.99% uptime (52 minutes/year downtime):

| Component | Expected Uptime | Downtime/Year |
|-----------|-----------------|---------------|
| Single PostgreSQL | 99.9% | 8.7 hours |
| 3-node Patroni cluster | 99.99% | 52 minutes |
| + Multi-site DR | 99.999% | 5 minutes |

**Key requirements for 99.99%:**
- ✅ 3+ PostgreSQL nodes
- ✅ Automatic failover (Patroni)
- ✅ Redundant network paths
- ✅ RAID storage
- ✅ UPS + backup power
- ✅ 24/7 monitoring
- ✅ Tested recovery procedures

---

## Support

For issues with:
- **PostgreSQL cluster**: Check Patroni logs (`journalctl -u patroni`)
- **Backups**: Check pgBackRest logs (`/var/log/pgbackrest`)
- **App connection**: Check PG_* environment variables
- **HAProxy**: Check stats page and logs
