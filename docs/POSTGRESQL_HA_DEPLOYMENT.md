# PostgreSQL High Availability Deployment Guide

## Target: 99.99% Uptime (52 minutes downtime/year)

This guide covers deploying a production-grade, self-managed PostgreSQL cluster with automatic failover.

---

## Architecture Overview

```
                            ┌─────────────────────────────────┐
                            │        APPLICATION LAYER        │
                            │   (Tauri Desktop App / APIs)    │
                            └───────────────┬─────────────────┘
                                            │
                                            ▼
                            ┌─────────────────────────────────┐
                            │          HAProxy VIP            │
                            │       10.0.0.100:5432          │
                            │   (keepalived for HA proxy)     │
                            └───────────────┬─────────────────┘
                                            │
              ┌─────────────────────────────┼─────────────────────────────┐
              │                             │                             │
              ▼                             ▼                             ▼
   ┌─────────────────────┐     ┌─────────────────────┐     ┌─────────────────────┐
   │   pg-node-1         │     │   pg-node-2         │     │   pg-node-3         │
   │   10.0.0.11         │     │   10.0.0.12         │     │   10.0.0.13         │
   │                     │     │                     │     │                     │
   │ ┌─────────────────┐ │     │ ┌─────────────────┐ │     │ ┌─────────────────┐ │
   │ │ PostgreSQL 16   │ │     │ │ PostgreSQL 16   │ │     │ │ PostgreSQL 16   │ │
   │ │ (PRIMARY)       │ │────►│ │ (SYNC STANDBY)  │ │     │ │ (ASYNC STANDBY) │ │
   │ └─────────────────┘ │ WAL │ └─────────────────┘ │     │ └─────────────────┘ │
   │ ┌─────────────────┐ │     │ ┌─────────────────┐ │     │ ┌─────────────────┐ │
   │ │ Patroni         │ │     │ │ Patroni         │ │     │ │ Patroni         │ │
   │ └─────────────────┘ │     │ └─────────────────┘ │     │ └─────────────────┘ │
   │ ┌─────────────────┐ │     │ ┌─────────────────┐ │     │ ┌─────────────────┐ │
   │ │ pgBackRest      │ │     │ │ pgBackRest      │ │     │ │ pgBackRest      │ │
   │ └─────────────────┘ │     │ └─────────────────┘ │     │ └─────────────────┘ │
   └─────────────────────┘     └─────────────────────┘     └─────────────────────┘
              │                             │                             │
              └─────────────────────────────┼─────────────────────────────┘
                                            │
                            ┌───────────────▼─────────────────┐
                            │        ETCD CLUSTER             │
                            │   (Distributed Consensus)       │
                            │                                 │
                            │  etcd-1: 10.0.0.21:2379        │
                            │  etcd-2: 10.0.0.22:2379        │
                            │  etcd-3: 10.0.0.23:2379        │
                            └───────────────┬─────────────────┘
                                            │
                            ┌───────────────▼─────────────────┐
                            │     BACKUP REPOSITORY           │
                            │   (NFS/S3-compatible/Local)     │
                            │                                 │
                            │   /backup/pgbackrest            │
                            │   - Full backups (weekly)       │
                            │   - Incremental (daily)         │
                            │   - WAL archiving (continuous)  │
                            └─────────────────────────────────┘
```

---

## Hardware Requirements

### Minimum for 99.99%

| Role | CPU | RAM | Storage | Quantity |
|------|-----|-----|---------|----------|
| PostgreSQL + Patroni | 8 cores | 32 GB | 500 GB NVMe SSD | 3 |
| etcd | 2 cores | 4 GB | 50 GB SSD | 3 (can colocate with PG) |
| HAProxy | 2 cores | 4 GB | 20 GB | 2 (active/standby) |
| Backup Storage | - | - | 2 TB+ | 1 NAS/SAN |

### Network Requirements

- 10 Gbps between all nodes (replication is I/O intensive)
- Redundant NICs on each server
- Separate VLAN for database traffic (recommended)

---

## Deployment Options

Choose one:
1. **Docker Compose** - For development/testing or small deployments
2. **Ansible** - For production bare-metal/VM deployments

---

## Option 1: Docker Compose Deployment

### Directory Structure

```
postgres-ha/
├── docker-compose.yml
├── .env
├── haproxy/
│   └── haproxy.cfg
├── patroni/
│   └── patroni.yml
├── pgbackrest/
│   └── pgbackrest.conf
└── scripts/
    ├── init-cluster.sh
    └── backup.sh
```

### Environment File (.env)

```bash
# Cluster Settings
CLUSTER_NAME=bike-fleet-cluster
POSTGRES_VERSION=16

# Network
PG_NODE1_IP=10.0.0.11
PG_NODE2_IP=10.0.0.12
PG_NODE3_IP=10.0.0.13
ETCD1_IP=10.0.0.21
ETCD2_IP=10.0.0.22
ETCD3_IP=10.0.0.23
HAPROXY_VIP=10.0.0.100

# Credentials (CHANGE THESE!)
POSTGRES_SUPERUSER_PASSWORD=<generate-strong-password>
POSTGRES_REPLICATION_PASSWORD=<generate-strong-password>
POSTGRES_APP_PASSWORD=<generate-strong-password>
PATRONI_RESTAPI_PASSWORD=<generate-strong-password>

# Backup
BACKUP_RETENTION_FULL=4
BACKUP_RETENTION_DIFF=14
PGBACKREST_REPO_PATH=/backup/pgbackrest
```

### Docker Compose File (docker-compose.yml)

```yaml
version: '3.8'

# This is the MAIN docker-compose.yml - deploy on each node with appropriate profile
# Usage: docker compose --profile node1 up -d  (on pg-node-1)
#        docker compose --profile node2 up -d  (on pg-node-2)
#        docker compose --profile node3 up -d  (on pg-node-3)
#        docker compose --profile etcd up -d   (on etcd nodes)
#        docker compose --profile haproxy up -d (on proxy nodes)

x-postgres-common: &postgres-common
  image: postgres:16-bookworm
  environment:
    POSTGRES_USER: postgres
    POSTGRES_PASSWORD: ${POSTGRES_SUPERUSER_PASSWORD}
  volumes:
    - ./patroni/patroni.yml:/etc/patroni/patroni.yml:ro
    - ./pgbackrest/pgbackrest.conf:/etc/pgbackrest/pgbackrest.conf:ro
  networks:
    - pg-cluster

services:
  # ============ ETCD CLUSTER ============
  etcd1:
    image: quay.io/coreos/etcd:v3.5.11
    profiles: ["etcd", "etcd1"]
    hostname: etcd1
    environment:
      ETCD_NAME: etcd1
      ETCD_INITIAL_ADVERTISE_PEER_URLS: http://${ETCD1_IP}:2380
      ETCD_LISTEN_PEER_URLS: http://0.0.0.0:2380
      ETCD_LISTEN_CLIENT_URLS: http://0.0.0.0:2379
      ETCD_ADVERTISE_CLIENT_URLS: http://${ETCD1_IP}:2379
      ETCD_INITIAL_CLUSTER: etcd1=http://${ETCD1_IP}:2380,etcd2=http://${ETCD2_IP}:2380,etcd3=http://${ETCD3_IP}:2380
      ETCD_INITIAL_CLUSTER_STATE: new
      ETCD_INITIAL_CLUSTER_TOKEN: ${CLUSTER_NAME}-etcd
    volumes:
      - etcd1-data:/etcd-data
    networks:
      - pg-cluster
    ports:
      - "2379:2379"
      - "2380:2380"
    restart: unless-stopped

  etcd2:
    image: quay.io/coreos/etcd:v3.5.11
    profiles: ["etcd", "etcd2"]
    hostname: etcd2
    environment:
      ETCD_NAME: etcd2
      ETCD_INITIAL_ADVERTISE_PEER_URLS: http://${ETCD2_IP}:2380
      ETCD_LISTEN_PEER_URLS: http://0.0.0.0:2380
      ETCD_LISTEN_CLIENT_URLS: http://0.0.0.0:2379
      ETCD_ADVERTISE_CLIENT_URLS: http://${ETCD2_IP}:2379
      ETCD_INITIAL_CLUSTER: etcd1=http://${ETCD1_IP}:2380,etcd2=http://${ETCD2_IP}:2380,etcd3=http://${ETCD3_IP}:2380
      ETCD_INITIAL_CLUSTER_STATE: new
      ETCD_INITIAL_CLUSTER_TOKEN: ${CLUSTER_NAME}-etcd
    volumes:
      - etcd2-data:/etcd-data
    networks:
      - pg-cluster
    ports:
      - "2379:2379"
      - "2380:2380"
    restart: unless-stopped

  etcd3:
    image: quay.io/coreos/etcd:v3.5.11
    profiles: ["etcd", "etcd3"]
    hostname: etcd3
    environment:
      ETCD_NAME: etcd3
      ETCD_INITIAL_ADVERTISE_PEER_URLS: http://${ETCD3_IP}:2380
      ETCD_LISTEN_PEER_URLS: http://0.0.0.0:2380
      ETCD_LISTEN_CLIENT_URLS: http://0.0.0.0:2379
      ETCD_ADVERTISE_CLIENT_URLS: http://${ETCD3_IP}:2379
      ETCD_INITIAL_CLUSTER: etcd1=http://${ETCD1_IP}:2380,etcd2=http://${ETCD2_IP}:2380,etcd3=http://${ETCD3_IP}:2380
      ETCD_INITIAL_CLUSTER_STATE: new
      ETCD_INITIAL_CLUSTER_TOKEN: ${CLUSTER_NAME}-etcd
    volumes:
      - etcd3-data:/etcd-data
    networks:
      - pg-cluster
    ports:
      - "2379:2379"
      - "2380:2380"
    restart: unless-stopped

  # ============ POSTGRESQL + PATRONI NODES ============
  pg-node-1:
    <<: *postgres-common
    profiles: ["postgres", "node1"]
    hostname: pg-node-1
    container_name: pg-node-1
    environment:
      PATRONI_NAME: pg-node-1
      PATRONI_POSTGRESQL_CONNECT_ADDRESS: ${PG_NODE1_IP}:5432
      PATRONI_RESTAPI_CONNECT_ADDRESS: ${PG_NODE1_IP}:8008
    volumes:
      - pg-node-1-data:/var/lib/postgresql/data
      - ./patroni/patroni-node1.yml:/etc/patroni/patroni.yml:ro
      - ./pgbackrest/pgbackrest.conf:/etc/pgbackrest/pgbackrest.conf:ro
      - backup-repo:/backup/pgbackrest
    ports:
      - "5432:5432"
      - "8008:8008"
    restart: unless-stopped

  pg-node-2:
    <<: *postgres-common
    profiles: ["postgres", "node2"]
    hostname: pg-node-2
    container_name: pg-node-2
    environment:
      PATRONI_NAME: pg-node-2
      PATRONI_POSTGRESQL_CONNECT_ADDRESS: ${PG_NODE2_IP}:5432
      PATRONI_RESTAPI_CONNECT_ADDRESS: ${PG_NODE2_IP}:8008
    volumes:
      - pg-node-2-data:/var/lib/postgresql/data
      - ./patroni/patroni-node2.yml:/etc/patroni/patroni.yml:ro
      - ./pgbackrest/pgbackrest.conf:/etc/pgbackrest/pgbackrest.conf:ro
      - backup-repo:/backup/pgbackrest
    ports:
      - "5432:5432"
      - "8008:8008"
    restart: unless-stopped

  pg-node-3:
    <<: *postgres-common
    profiles: ["postgres", "node3"]
    hostname: pg-node-3
    container_name: pg-node-3
    environment:
      PATRONI_NAME: pg-node-3
      PATRONI_POSTGRESQL_CONNECT_ADDRESS: ${PG_NODE3_IP}:5432
      PATRONI_RESTAPI_CONNECT_ADDRESS: ${PG_NODE3_IP}:8008
    volumes:
      - pg-node-3-data:/var/lib/postgresql/data
      - ./patroni/patroni-node3.yml:/etc/patroni/patroni.yml:ro
      - ./pgbackrest/pgbackrest.conf:/etc/pgbackrest/pgbackrest.conf:ro
      - backup-repo:/backup/pgbackrest
    ports:
      - "5432:5432"
      - "8008:8008"
    restart: unless-stopped

  # ============ HAPROXY LOAD BALANCER ============
  haproxy:
    image: haproxy:2.9-bookworm
    profiles: ["haproxy"]
    hostname: haproxy
    volumes:
      - ./haproxy/haproxy.cfg:/usr/local/etc/haproxy/haproxy.cfg:ro
    ports:
      - "5432:5432"   # PostgreSQL (write)
      - "5433:5433"   # PostgreSQL (read replicas)
      - "8404:8404"   # HAProxy stats
    networks:
      - pg-cluster
    restart: unless-stopped

networks:
  pg-cluster:
    driver: bridge
    ipam:
      config:
        - subnet: 10.0.0.0/24

volumes:
  etcd1-data:
  etcd2-data:
  etcd3-data:
  pg-node-1-data:
  pg-node-2-data:
  pg-node-3-data:
  backup-repo:
```

### Patroni Configuration (patroni/patroni-node1.yml)

```yaml
scope: bike-fleet-cluster
name: pg-node-1

restapi:
  listen: 0.0.0.0:8008
  connect_address: 10.0.0.11:8008
  authentication:
    username: patroni
    password: ${PATRONI_RESTAPI_PASSWORD}

etcd3:
  hosts:
    - 10.0.0.21:2379
    - 10.0.0.22:2379
    - 10.0.0.23:2379

bootstrap:
  dcs:
    ttl: 30
    loop_wait: 10
    retry_timeout: 10
    maximum_lag_on_failover: 1048576  # 1MB
    synchronous_mode: true
    synchronous_mode_strict: false
    postgresql:
      use_pg_rewind: true
      use_slots: true
      parameters:
        # Connection
        max_connections: 200
        superuser_reserved_connections: 5

        # Memory (adjust based on your RAM)
        shared_buffers: 8GB
        effective_cache_size: 24GB
        work_mem: 64MB
        maintenance_work_mem: 2GB

        # WAL & Replication
        wal_level: replica
        hot_standby: 'on'
        max_wal_senders: 10
        max_replication_slots: 10
        wal_keep_size: 1GB
        archive_mode: 'on'
        archive_command: 'pgbackrest --stanza=bike-fleet archive-push %p'

        # Checkpoints
        checkpoint_completion_target: 0.9
        max_wal_size: 4GB
        min_wal_size: 1GB

        # Logging
        log_destination: 'stderr'
        logging_collector: 'on'
        log_directory: 'log'
        log_filename: 'postgresql-%Y-%m-%d_%H%M%S.log'
        log_min_duration_statement: 1000  # Log queries > 1 second
        log_checkpoints: 'on'
        log_connections: 'on'
        log_disconnections: 'on'
        log_lock_waits: 'on'

        # Performance
        random_page_cost: 1.1  # SSD
        effective_io_concurrency: 200  # SSD

  initdb:
    - encoding: UTF8
    - data-checksums
    - locale: en_US.UTF-8

  pg_hba:
    - host replication replicator 10.0.0.0/24 md5
    - host all all 10.0.0.0/24 md5
    - host all all 0.0.0.0/0 md5

  users:
    admin:
      password: ${POSTGRES_SUPERUSER_PASSWORD}
      options:
        - createrole
        - createdb
    replicator:
      password: ${POSTGRES_REPLICATION_PASSWORD}
      options:
        - replication
    fleet_app:
      password: ${POSTGRES_APP_PASSWORD}
      options: []

postgresql:
  listen: 0.0.0.0:5432
  connect_address: 10.0.0.11:5432
  data_dir: /var/lib/postgresql/data
  pgpass: /tmp/pgpass
  authentication:
    superuser:
      username: postgres
      password: ${POSTGRES_SUPERUSER_PASSWORD}
    replication:
      username: replicator
      password: ${POSTGRES_REPLICATION_PASSWORD}
  parameters:
    unix_socket_directories: '/var/run/postgresql'
  pg_hba:
    - local all all trust
    - host replication replicator 127.0.0.1/32 trust
    - host replication replicator 10.0.0.0/24 md5
    - host all all 0.0.0.0/0 md5

tags:
  nofailover: false
  noloadbalance: false
  clonefrom: false
  nosync: false
```

### HAProxy Configuration (haproxy/haproxy.cfg)

```haproxy
global
    maxconn 1000
    log stdout format raw local0

defaults
    log global
    mode tcp
    retries 3
    timeout connect 10s
    timeout client 30m
    timeout server 30m
    timeout check 5s

# Stats page - accessible at http://<haproxy-ip>:8404/stats
listen stats
    mode http
    bind *:8404
    stats enable
    stats uri /stats
    stats refresh 10s
    stats admin if LOCALHOST

# PostgreSQL Primary (read-write)
listen postgresql-primary
    bind *:5432
    mode tcp
    option httpchk GET /primary
    http-check expect status 200
    default-server inter 3s fall 3 rise 2 on-marked-down shutdown-sessions
    server pg-node-1 10.0.0.11:5432 check port 8008
    server pg-node-2 10.0.0.12:5432 check port 8008
    server pg-node-3 10.0.0.13:5432 check port 8008

# PostgreSQL Replicas (read-only)
listen postgresql-replicas
    bind *:5433
    mode tcp
    balance roundrobin
    option httpchk GET /replica
    http-check expect status 200
    default-server inter 3s fall 3 rise 2 on-marked-down shutdown-sessions
    server pg-node-1 10.0.0.11:5432 check port 8008
    server pg-node-2 10.0.0.12:5432 check port 8008
    server pg-node-3 10.0.0.13:5432 check port 8008
```

---

## Option 2: Ansible Deployment (Production)

For production bare-metal/VM deployments, use Ansible.

### Directory Structure

```
ansible-postgres-ha/
├── inventory/
│   └── hosts.yml
├── group_vars/
│   ├── all.yml
│   └── vault.yml (encrypted)
├── roles/
│   ├── common/
│   ├── etcd/
│   ├── postgresql/
│   ├── patroni/
│   ├── pgbackrest/
│   ├── haproxy/
│   └── monitoring/
├── playbooks/
│   ├── site.yml
│   ├── deploy-cluster.yml
│   ├── backup.yml
│   └── restore.yml
└── ansible.cfg
```

### Inventory (inventory/hosts.yml)

```yaml
all:
  children:
    etcd:
      hosts:
        etcd-1:
          ansible_host: 10.0.0.21
          etcd_name: etcd1
        etcd-2:
          ansible_host: 10.0.0.22
          etcd_name: etcd2
        etcd-3:
          ansible_host: 10.0.0.23
          etcd_name: etcd3

    postgresql:
      hosts:
        pg-node-1:
          ansible_host: 10.0.0.11
          patroni_name: pg-node-1
          patroni_tags:
            nofailover: false
            noloadbalance: false
        pg-node-2:
          ansible_host: 10.0.0.12
          patroni_name: pg-node-2
          patroni_tags:
            nofailover: false
            noloadbalance: false
        pg-node-3:
          ansible_host: 10.0.0.13
          patroni_name: pg-node-3
          patroni_tags:
            nofailover: false
            noloadbalance: false

    haproxy:
      hosts:
        haproxy-1:
          ansible_host: 10.0.0.101
          keepalived_priority: 100
          keepalived_state: MASTER
        haproxy-2:
          ansible_host: 10.0.0.102
          keepalived_priority: 99
          keepalived_state: BACKUP

  vars:
    ansible_user: admin
    ansible_become: true
    cluster_name: bike-fleet-cluster
    haproxy_vip: 10.0.0.100
```

### Group Variables (group_vars/all.yml)

```yaml
---
# Cluster Configuration
cluster_name: bike-fleet-cluster
postgres_version: 16

# Network
postgres_port: 5432
patroni_restapi_port: 8008
etcd_client_port: 2379
etcd_peer_port: 2380

# PostgreSQL Tuning (adjust based on hardware)
postgresql_shared_buffers: "8GB"
postgresql_effective_cache_size: "24GB"
postgresql_work_mem: "64MB"
postgresql_maintenance_work_mem: "2GB"
postgresql_max_connections: 200

# Replication
postgresql_synchronous_mode: true
postgresql_max_lag_on_failover: 1048576

# Backup Configuration
pgbackrest_repo_path: /backup/pgbackrest
pgbackrest_retention_full: 4
pgbackrest_retention_diff: 14
pgbackrest_repo_retention_archive: 30
pgbackrest_compress_level: 6
pgbackrest_process_max: 4

# Monitoring
prometheus_enabled: true
grafana_enabled: true
```

### Main Playbook (playbooks/deploy-cluster.yml)

```yaml
---
- name: Deploy PostgreSQL HA Cluster
  hosts: all
  become: true

  pre_tasks:
    - name: Validate inventory
      assert:
        that:
          - groups['etcd'] | length >= 3
          - groups['postgresql'] | length >= 3
          - groups['haproxy'] | length >= 2
        fail_msg: "Minimum 3 etcd, 3 PostgreSQL, and 2 HAProxy nodes required for HA"

- name: Configure common settings
  hosts: all
  roles:
    - common

- name: Deploy etcd cluster
  hosts: etcd
  roles:
    - etcd

- name: Deploy PostgreSQL with Patroni
  hosts: postgresql
  serial: 1  # Deploy one at a time for safe initialization
  roles:
    - postgresql
    - patroni
    - pgbackrest

- name: Deploy HAProxy load balancers
  hosts: haproxy
  roles:
    - haproxy

- name: Deploy monitoring stack
  hosts: postgresql[0]
  roles:
    - monitoring
  when: prometheus_enabled | default(true)

- name: Verify cluster health
  hosts: postgresql[0]
  tasks:
    - name: Check Patroni cluster status
      command: patronictl -c /etc/patroni/patroni.yml list
      register: patroni_status
      changed_when: false

    - name: Display cluster status
      debug:
        var: patroni_status.stdout_lines

    - name: Verify cluster has a leader
      assert:
        that:
          - "'Leader' in patroni_status.stdout"
        fail_msg: "No leader elected - cluster may be unhealthy"
```

### Patroni Role (roles/patroni/tasks/main.yml)

```yaml
---
- name: Install Patroni dependencies
  apt:
    name:
      - python3
      - python3-pip
      - python3-psycopg2
    state: present

- name: Install Patroni
  pip:
    name:
      - patroni[etcd3]
    state: present

- name: Create Patroni configuration directory
  file:
    path: /etc/patroni
    state: directory
    mode: '0755'

- name: Deploy Patroni configuration
  template:
    src: patroni.yml.j2
    dest: /etc/patroni/patroni.yml
    mode: '0640'
    owner: postgres
    group: postgres
  notify: Restart Patroni

- name: Create Patroni systemd service
  template:
    src: patroni.service.j2
    dest: /etc/systemd/system/patroni.service
    mode: '0644'
  notify:
    - Reload systemd
    - Restart Patroni

- name: Enable and start Patroni
  systemd:
    name: patroni
    enabled: true
    state: started
```

### Patroni Configuration Template (roles/patroni/templates/patroni.yml.j2)

```yaml
scope: {{ cluster_name }}
name: {{ patroni_name }}

restapi:
  listen: 0.0.0.0:{{ patroni_restapi_port }}
  connect_address: {{ ansible_host }}:{{ patroni_restapi_port }}
  authentication:
    username: patroni
    password: {{ patroni_restapi_password }}

etcd3:
  hosts:
{% for host in groups['etcd'] %}
    - {{ hostvars[host]['ansible_host'] }}:{{ etcd_client_port }}
{% endfor %}

bootstrap:
  dcs:
    ttl: 30
    loop_wait: 10
    retry_timeout: 10
    maximum_lag_on_failover: {{ postgresql_max_lag_on_failover }}
    synchronous_mode: {{ postgresql_synchronous_mode | lower }}
    postgresql:
      use_pg_rewind: true
      use_slots: true
      parameters:
        max_connections: {{ postgresql_max_connections }}
        shared_buffers: {{ postgresql_shared_buffers }}
        effective_cache_size: {{ postgresql_effective_cache_size }}
        work_mem: {{ postgresql_work_mem }}
        maintenance_work_mem: {{ postgresql_maintenance_work_mem }}
        wal_level: replica
        hot_standby: 'on'
        max_wal_senders: 10
        max_replication_slots: 10
        wal_keep_size: 1GB
        archive_mode: 'on'
        archive_command: 'pgbackrest --stanza={{ cluster_name }} archive-push %p'

  initdb:
    - encoding: UTF8
    - data-checksums
    - locale: en_US.UTF-8

  pg_hba:
    - host replication {{ postgres_replication_user }} 10.0.0.0/24 md5
    - host all all 10.0.0.0/24 md5
    - host all all 0.0.0.0/0 md5

  users:
    {{ postgres_admin_user }}:
      password: {{ postgres_admin_password }}
      options:
        - createrole
        - createdb
    {{ postgres_replication_user }}:
      password: {{ postgres_replication_password }}
      options:
        - replication
    {{ postgres_app_user }}:
      password: {{ postgres_app_password }}
      options: []

postgresql:
  listen: 0.0.0.0:{{ postgres_port }}
  connect_address: {{ ansible_host }}:{{ postgres_port }}
  data_dir: /var/lib/postgresql/{{ postgres_version }}/main
  bin_dir: /usr/lib/postgresql/{{ postgres_version }}/bin
  authentication:
    superuser:
      username: postgres
      password: {{ postgres_superuser_password }}
    replication:
      username: {{ postgres_replication_user }}
      password: {{ postgres_replication_password }}

tags:
{% for key, value in patroni_tags.items() %}
  {{ key }}: {{ value | lower }}
{% endfor %}
```

---

## Deployment Steps

### Docker Compose (Development/Testing)

```bash
# 1. Clone and configure
git clone <your-repo>
cd postgres-ha
cp .env.example .env
# Edit .env with your passwords and IPs

# 2. Start etcd cluster first (run on each etcd node)
docker compose --profile etcd1 up -d  # on etcd node 1
docker compose --profile etcd2 up -d  # on etcd node 2
docker compose --profile etcd3 up -d  # on etcd node 3

# 3. Verify etcd health
docker exec etcd1 etcdctl endpoint health --cluster

# 4. Start PostgreSQL nodes (run on each PG node)
docker compose --profile node1 up -d  # on pg-node-1
docker compose --profile node2 up -d  # on pg-node-2
docker compose --profile node3 up -d  # on pg-node-3

# 5. Start HAProxy
docker compose --profile haproxy up -d

# 6. Verify cluster
docker exec pg-node-1 patronictl list
```

### Ansible (Production)

```bash
# 1. Install Ansible
pip install ansible

# 2. Configure inventory and variables
cd ansible-postgres-ha
# Edit inventory/hosts.yml with your IPs
# Edit group_vars/all.yml with your settings

# 3. Create encrypted vault for secrets
ansible-vault create group_vars/vault.yml
# Add passwords:
# postgres_superuser_password: <password>
# postgres_replication_password: <password>
# postgres_app_password: <password>
# patroni_restapi_password: <password>

# 4. Test connectivity
ansible all -m ping

# 5. Deploy cluster
ansible-playbook playbooks/deploy-cluster.yml --ask-vault-pass

# 6. Verify cluster
ansible postgresql -m command -a "patronictl -c /etc/patroni/patroni.yml list"
```

---

## Verification Commands

```bash
# Check Patroni cluster status
patronictl -c /etc/patroni/patroni.yml list

# Expected output:
# + Cluster: bike-fleet-cluster ----+---------+---------+----+-----------+
# | Member    | Host       | Role    | State   | TL | Lag in MB |
# +-----------+------------+---------+---------+----+-----------+
# | pg-node-1 | 10.0.0.11  | Leader  | running |  1 |           |
# | pg-node-2 | 10.0.0.12  | Replica | running |  1 |         0 |
# | pg-node-3 | 10.0.0.13  | Replica | running |  1 |         0 |
# +-----------+------------+---------+---------+----+-----------+

# Check etcd cluster health
etcdctl endpoint health --cluster

# Check HAProxy stats
curl http://10.0.0.100:8404/stats

# Test database connection through HAProxy
psql -h 10.0.0.100 -p 5432 -U fleet_app -d bike_fleet -c "SELECT 1"
```

---

## Failover Testing

```bash
# Manual failover to specific node
patronictl -c /etc/patroni/patroni.yml switchover --master pg-node-1 --candidate pg-node-2

# Simulate node failure
docker stop pg-node-1  # or: systemctl stop patroni

# Watch failover (should complete in < 30 seconds)
watch -n 1 "patronictl -c /etc/patroni/patroni.yml list"

# Rejoin recovered node
docker start pg-node-1  # or: systemctl start patroni
```

---

## Next Steps

1. Set up monitoring (see MONITORING.md)
2. Configure backups (see BACKUP_RECOVERY.md)
3. Update application connection strings
4. Test failover scenarios
5. Document runbooks for operations team
