# PostgreSQL Backup & Recovery Strategy

## Target: Zero Data Loss + Fast Recovery

This document covers the complete backup and disaster recovery strategy for achieving 99.99% availability with pgBackRest.

---

## Backup Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           BACKUP FLOW                                        │
│                                                                              │
│   PostgreSQL Primary                                                         │
│   ┌─────────────────┐                                                       │
│   │                 │──── WAL Streaming ────► Sync Replica (HA)             │
│   │   Write Data    │                                                       │
│   │                 │──── archive_command ──┐                               │
│   └─────────────────┘                       │                               │
│                                             ▼                               │
│                              ┌─────────────────────────────────┐            │
│                              │     pgBackRest Repository       │            │
│                              │                                 │            │
│                              │  ┌─────────────────────────┐   │            │
│                              │  │   WAL Archive           │   │            │
│                              │  │   (continuous)          │   │            │
│                              │  │   Retention: 30 days    │   │            │
│                              │  └─────────────────────────┘   │            │
│                              │                                 │            │
│                              │  ┌─────────────────────────┐   │            │
│                              │  │   Full Backups          │   │            │
│                              │  │   (weekly, Sunday 2AM)  │   │            │
│                              │  │   Retention: 4 weeks    │   │            │
│                              │  └─────────────────────────┘   │            │
│                              │                                 │            │
│                              │  ┌─────────────────────────┐   │            │
│                              │  │   Differential Backups  │   │            │
│                              │  │   (daily, 2AM)          │   │            │
│                              │  │   Retention: 14 days    │   │            │
│                              │  └─────────────────────────┘   │            │
│                              │                                 │            │
│                              └─────────────────────────────────┘            │
│                                             │                               │
│                                             │ (optional)                    │
│                                             ▼                               │
│                              ┌─────────────────────────────────┐            │
│                              │   Off-site Copy (DR Site)       │            │
│                              │   - S3-compatible storage       │            │
│                              │   - NFS share at remote site    │            │
│                              └─────────────────────────────────┘            │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Backup Types & Schedule

| Backup Type | Schedule | Retention | RPO* | Description |
|-------------|----------|-----------|------|-------------|
| **WAL Archive** | Continuous | 30 days | ~0 sec | Every WAL segment archived immediately |
| **Full Backup** | Weekly (Sun 2AM) | 4 weeks | N/A | Complete database snapshot |
| **Differential** | Daily (2AM) | 14 days | N/A | Changes since last full |
| **Incremental** | Optional hourly | 7 days | N/A | Changes since last backup (any type) |

*RPO = Recovery Point Objective (max data loss)

---

## pgBackRest Configuration

### Main Configuration (/etc/pgbackrest/pgbackrest.conf)

```ini
[global]
# Repository location
repo1-path=/backup/pgbackrest
repo1-retention-full=4
repo1-retention-diff=14
repo1-retention-archive=30

# Performance tuning
process-max=4
compress-type=zst
compress-level=6

# Encryption at rest (optional but recommended)
repo1-cipher-type=aes-256-cbc
repo1-cipher-pass=<your-encryption-key>

# Delta restore (faster restores)
delta=y

# Archive settings
archive-async=y
archive-push-queue-max=4GiB

# Logging
log-level-console=info
log-level-file=detail
log-path=/var/log/pgbackrest

# TLS for remote operations
tls-server-address=*
tls-server-cert-file=/etc/pgbackrest/server.crt
tls-server-key-file=/etc/pgbackrest/server.key

[bike-fleet]
# Stanza (database cluster) configuration
pg1-path=/var/lib/postgresql/16/main
pg1-port=5432
pg1-user=postgres

# For standby servers, add their paths
# pg2-host=pg-node-2
# pg2-path=/var/lib/postgresql/16/main

# Recovery settings
recovery-option=recovery_target_timeline=latest
```

### Off-site Repository Configuration (Optional)

```ini
[global]
# ... main config above ...

# Second repository for disaster recovery
repo2-type=s3
repo2-s3-endpoint=s3.your-internal-s3.local
repo2-s3-bucket=postgres-backups
repo2-s3-region=us-east-1
repo2-path=/bike-fleet
repo2-retention-full=8
repo2-retention-archive=60

# Copy backups to off-site asynchronously
repo2-bundle=y
repo2-block=y
```

---

## Backup Commands

### Initial Setup

```bash
# Create the stanza (run once after first PostgreSQL setup)
sudo -u postgres pgbackrest --stanza=bike-fleet stanza-create

# Verify the stanza
sudo -u postgres pgbackrest --stanza=bike-fleet check
```

### Manual Backup Commands

```bash
# Full backup
sudo -u postgres pgbackrest --stanza=bike-fleet --type=full backup

# Differential backup (changes since last full)
sudo -u postgres pgbackrest --stanza=bike-fleet --type=diff backup

# Incremental backup (changes since any last backup)
sudo -u postgres pgbackrest --stanza=bike-fleet --type=incr backup

# Backup with annotation
sudo -u postgres pgbackrest --stanza=bike-fleet --type=full \
    --annotation="pre-upgrade-backup" backup
```

### Verify Backup Integrity

```bash
# List all backups
sudo -u postgres pgbackrest --stanza=bike-fleet info

# Verify latest backup (checks checksums)
sudo -u postgres pgbackrest --stanza=bike-fleet verify

# Verify specific backup
sudo -u postgres pgbackrest --stanza=bike-fleet --set=20240115-020000F verify
```

---

## Automated Backup Schedule (Cron/Systemd)

### Option 1: Cron Jobs

```cron
# /etc/cron.d/pgbackrest

# Full backup every Sunday at 2:00 AM
0 2 * * 0 postgres pgbackrest --stanza=bike-fleet --type=full backup

# Differential backup Monday-Saturday at 2:00 AM
0 2 * * 1-6 postgres pgbackrest --stanza=bike-fleet --type=diff backup

# Verify backup integrity every day at 6:00 AM
0 6 * * * postgres pgbackrest --stanza=bike-fleet verify

# Clean up expired backups every Sunday at 4:00 AM
0 4 * * 0 postgres pgbackrest --stanza=bike-fleet expire
```

### Option 2: Systemd Timers (Recommended)

**Backup Service (/etc/systemd/system/pgbackrest-backup.service)**:
```ini
[Unit]
Description=pgBackRest backup
After=postgresql.service

[Service]
Type=oneshot
User=postgres
ExecStart=/usr/bin/pgbackrest --stanza=bike-fleet --type=diff backup
ExecStartPost=/usr/bin/pgbackrest --stanza=bike-fleet verify
```

**Backup Timer (/etc/systemd/system/pgbackrest-backup.timer)**:
```ini
[Unit]
Description=Daily pgBackRest backup

[Timer]
OnCalendar=*-*-* 02:00:00
RandomizedDelaySec=300
Persistent=true

[Install]
WantedBy=timers.target
```

**Full Backup Timer (/etc/systemd/system/pgbackrest-full.timer)**:
```ini
[Unit]
Description=Weekly pgBackRest full backup

[Timer]
OnCalendar=Sun *-*-* 02:00:00
Persistent=true

[Install]
WantedBy=timers.target
```

**Enable timers**:
```bash
sudo systemctl enable --now pgbackrest-backup.timer
sudo systemctl enable --now pgbackrest-full.timer
```

---

## Recovery Procedures

### Scenario 1: Point-in-Time Recovery (PITR)

Recover to a specific timestamp (e.g., before accidental data deletion):

```bash
# 1. Stop PostgreSQL
sudo systemctl stop patroni

# 2. Restore to specific point in time
sudo -u postgres pgbackrest --stanza=bike-fleet \
    --target="2024-01-15 14:30:00" \
    --target-action=promote \
    --type=time \
    --delta \
    restore

# 3. Start PostgreSQL
sudo systemctl start patroni

# 4. Verify recovery
psql -c "SELECT pg_is_in_recovery();"  # Should return 'f' (false)
```

### Scenario 2: Full Cluster Restore (Disaster Recovery)

Complete restore from backup (e.g., total data loss):

```bash
# 1. Stop PostgreSQL on all nodes
ssh pg-node-1 "sudo systemctl stop patroni"
ssh pg-node-2 "sudo systemctl stop patroni"
ssh pg-node-3 "sudo systemctl stop patroni"

# 2. Clear data directory on primary node
ssh pg-node-1 "sudo -u postgres rm -rf /var/lib/postgresql/16/main/*"

# 3. Restore latest backup
ssh pg-node-1 "sudo -u postgres pgbackrest --stanza=bike-fleet \
    --target-action=promote \
    --delta \
    restore"

# 4. Start primary
ssh pg-node-1 "sudo systemctl start patroni"

# 5. Wait for primary to be ready
sleep 30

# 6. Reinitialize replicas (Patroni handles this automatically)
ssh pg-node-2 "sudo systemctl start patroni"
ssh pg-node-3 "sudo systemctl start patroni"

# 7. Verify cluster health
ssh pg-node-1 "patronictl -c /etc/patroni/patroni.yml list"
```

### Scenario 3: Restore Specific Tables

Restore only specific tables (requires pg_restore):

```bash
# 1. Restore to a temporary directory
sudo -u postgres pgbackrest --stanza=bike-fleet \
    --target="2024-01-15 14:30:00" \
    --type=time \
    --db-include=bike_fleet \
    --target-action=pause \
    restore --pg1-path=/tmp/pg_restore_temp

# 2. Start temporary PostgreSQL on different port
pg_ctl -D /tmp/pg_restore_temp -o "-p 5433" start

# 3. Dump specific tables
pg_dump -h localhost -p 5433 -t bikes -t deliveries bike_fleet > /tmp/tables_backup.sql

# 4. Stop temporary PostgreSQL
pg_ctl -D /tmp/pg_restore_temp stop

# 5. Restore tables to production (be careful!)
psql -h localhost -p 5432 -d bike_fleet < /tmp/tables_backup.sql

# 6. Clean up
rm -rf /tmp/pg_restore_temp /tmp/tables_backup.sql
```

### Scenario 4: Restore to Different Server

Restore backup to a new server:

```bash
# On the new server:

# 1. Install PostgreSQL and pgBackRest
apt install postgresql-16 pgbackrest

# 2. Copy pgbackrest.conf (adjust paths)
scp pg-node-1:/etc/pgbackrest/pgbackrest.conf /etc/pgbackrest/

# 3. Mount or copy the backup repository
mount -t nfs backup-server:/backup/pgbackrest /backup/pgbackrest

# 4. Restore
sudo -u postgres pgbackrest --stanza=bike-fleet restore

# 5. Start PostgreSQL
sudo systemctl start postgresql
```

---

## Monitoring & Alerting

### Backup Monitoring Script

```bash
#!/bin/bash
# /usr/local/bin/check_pgbackrest.sh

STANZA="bike-fleet"
MAX_BACKUP_AGE_HOURS=26  # Alert if no backup in 26 hours
MAX_WAL_LAG_SECONDS=300  # Alert if WAL archiving is > 5 min behind

# Check backup age
LAST_BACKUP=$(pgbackrest --stanza=$STANZA info --output=json | \
    jq -r '.[0].backup[-1].timestamp.stop')
LAST_BACKUP_EPOCH=$(date -d "$LAST_BACKUP" +%s)
NOW_EPOCH=$(date +%s)
BACKUP_AGE_HOURS=$(( (NOW_EPOCH - LAST_BACKUP_EPOCH) / 3600 ))

if [ $BACKUP_AGE_HOURS -gt $MAX_BACKUP_AGE_HOURS ]; then
    echo "CRITICAL: Last backup is $BACKUP_AGE_HOURS hours old"
    exit 2
fi

# Check WAL archiving
ARCHIVE_STATUS=$(pgbackrest --stanza=$STANZA check 2>&1)
if echo "$ARCHIVE_STATUS" | grep -q "ERROR"; then
    echo "CRITICAL: WAL archiving error - $ARCHIVE_STATUS"
    exit 2
fi

# Check backup verification
VERIFY_STATUS=$(pgbackrest --stanza=$STANZA verify 2>&1)
if echo "$VERIFY_STATUS" | grep -q "ERROR"; then
    echo "WARNING: Backup verification failed - $VERIFY_STATUS"
    exit 1
fi

echo "OK: Backup healthy, last backup $BACKUP_AGE_HOURS hours ago"
exit 0
```

### Prometheus Metrics (with pgbackrest_exporter)

```yaml
# prometheus/alerts/pgbackrest.yml
groups:
  - name: pgbackrest
    rules:
      - alert: PostgresBackupMissing
        expr: time() - pgbackrest_backup_last_timestamp > 93600  # 26 hours
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "PostgreSQL backup is stale"
          description: "No successful backup in the last 26 hours"

      - alert: PostgresWALArchiveBehind
        expr: pgbackrest_wal_archive_status != 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "PostgreSQL WAL archiving is behind"
          description: "WAL archiving may be failing or lagging"

      - alert: PostgresBackupVerifyFailed
        expr: pgbackrest_backup_verify_status != 1
        for: 15m
        labels:
          severity: warning
        annotations:
          summary: "PostgreSQL backup verification failed"
          description: "Backup integrity check failed - backups may be corrupted"
```

---

## Recovery Time Objectives (RTO)

| Scenario | Expected RTO | Notes |
|----------|--------------|-------|
| Single node failure | < 30 seconds | Patroni automatic failover |
| Corrupted table | 5-15 minutes | PITR to before corruption |
| Full database restore (100GB) | 30-60 minutes | Depends on storage speed |
| Full cluster rebuild | 1-2 hours | Includes replica sync |
| Cross-site DR failover | 5-15 minutes | Requires manual intervention |

---

## Backup Storage Sizing

Formula: `Storage = (DB_Size × Full_Retention) + (Daily_Change × Diff_Retention) + (WAL_Rate × Archive_Retention)`

**Example for 50GB database:**
- Full backups: 50GB × 4 weeks = 200GB
- Differential (5% daily change): 2.5GB × 14 days = 35GB
- WAL archive (1GB/day): 1GB × 30 days = 30GB
- **Total: ~265GB + 20% overhead = ~320GB**

---

## Testing Procedures

### Monthly DR Test

```bash
#!/bin/bash
# /usr/local/bin/dr_test.sh

echo "=== Monthly DR Test Started ==="
echo "Date: $(date)"

# 1. Create test database
psql -c "CREATE DATABASE dr_test_$(date +%Y%m%d);"
psql -d dr_test_$(date +%Y%m%d) -c "CREATE TABLE test (id serial, data text);"
psql -d dr_test_$(date +%Y%m%d) -c "INSERT INTO test (data) SELECT md5(random()::text) FROM generate_series(1,10000);"

# 2. Take backup
pgbackrest --stanza=bike-fleet --type=full backup

# 3. Record current state
CHECKSUM=$(psql -d dr_test_$(date +%Y%m%d) -t -c "SELECT md5(string_agg(data, '')) FROM test;")
echo "Checksum before restore: $CHECKSUM"

# 4. Restore to temporary location (non-destructive test)
mkdir -p /tmp/dr_test
pgbackrest --stanza=bike-fleet --pg1-path=/tmp/dr_test --delta restore

# 5. Start temporary PostgreSQL and verify
pg_ctl -D /tmp/dr_test -o "-p 5433" start
RESTORED_CHECKSUM=$(psql -p 5433 -d dr_test_$(date +%Y%m%d) -t -c "SELECT md5(string_agg(data, '')) FROM test;")
echo "Checksum after restore: $RESTORED_CHECKSUM"

# 6. Compare
if [ "$CHECKSUM" = "$RESTORED_CHECKSUM" ]; then
    echo "SUCCESS: DR test passed - checksums match"
else
    echo "FAILURE: DR test failed - checksums do not match"
fi

# 7. Cleanup
pg_ctl -D /tmp/dr_test stop
rm -rf /tmp/dr_test
psql -c "DROP DATABASE dr_test_$(date +%Y%m%d);"

echo "=== Monthly DR Test Completed ==="
```

---

## Quick Reference Card

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     pgBackRest Quick Reference                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  BACKUP COMMANDS                                                             │
│  ───────────────                                                             │
│  Full backup:     pgbackrest --stanza=bike-fleet --type=full backup         │
│  Diff backup:     pgbackrest --stanza=bike-fleet --type=diff backup         │
│  List backups:    pgbackrest --stanza=bike-fleet info                       │
│  Verify:          pgbackrest --stanza=bike-fleet verify                     │
│                                                                              │
│  RESTORE COMMANDS                                                            │
│  ────────────────                                                            │
│  Latest:          pgbackrest --stanza=bike-fleet restore                    │
│  Point-in-time:   pgbackrest --stanza=bike-fleet --type=time \              │
│                     --target="2024-01-15 14:30:00" restore                  │
│  Specific backup: pgbackrest --stanza=bike-fleet --set=<backup-label> \     │
│                     restore                                                  │
│                                                                              │
│  EMERGENCY PROCEDURES                                                        │
│  ────────────────────                                                        │
│  1. Stop Patroni:   systemctl stop patroni                                  │
│  2. Restore:        pgbackrest --stanza=bike-fleet --delta restore          │
│  3. Start Patroni:  systemctl start patroni                                 │
│                                                                              │
│  RECOVERY TARGETS                                                            │
│  ────────────────                                                            │
│  RPO: < 1 minute (WAL archiving)                                            │
│  RTO: < 1 hour (full restore)                                               │
│  Failover: < 30 seconds (Patroni)                                           │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```
