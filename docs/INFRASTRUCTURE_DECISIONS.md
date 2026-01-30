# Infrastructure Decisions

## Why Not Kubernetes?

This document explains our infrastructure choices for on-premise deployments with high availability requirements but low load (2-3 client computers).

---

## TL;DR

**Kubernetes solves scaling and multi-team deployment problems. We have neither.**

With 2-3 clients and low load, K8s adds massive operational complexity for zero benefit. The HA problem is already solved by Patroni + HAProxy at the database level.

---

## Decision Matrix

| Factor | Our Situation | K8s Sweet Spot |
|--------|----------------|----------------|
| **Load** | 2-3 clients | 100+ pods, dynamic scaling |
| **Team size** | Small | Multiple teams deploying independently |
| **Apps** | ~5 (Fleet, ELK, Jira, PG, etc.) | 50+ microservices |
| **Ops expertise needed** | Medium | High (dedicated K8s admin) |
| **Recovery complexity** | Simple VMs | StatefulSets, PVCs, operators |

---

## The Kubernetes Tax

Running K8s/K3s for our use case would require:

- etcd cluster (3 nodes) just for K8s itself
- Control plane HA (3 masters recommended)
- Persistent volume management
- Ingress controllers
- Certificate management
- Operators for stateful apps (PostgreSQL, Elasticsearch)
- ~20-30% resource overhead
- Dedicated K8s expertise for troubleshooting

**None of this provides value for 2-3 clients.**

---

## Our Recommended Architecture

### Simple VM/Bare-Metal Setup

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        INFRASTRUCTURE OVERVIEW                           │
│                                                                          │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │                     SERVER 1 (Primary)                          │   │
│   │                                                                 │   │
│   │   • PostgreSQL (Patroni primary)                                │   │
│   │   • Elasticsearch (master + data)                               │   │
│   │   • Jira                                                        │   │
│   │   • HAProxy                                                     │   │
│   │   • etcd node 1                                                 │   │
│   └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │                     SERVER 2 (Secondary)                        │   │
│   │                                                                 │   │
│   │   • PostgreSQL (Patroni replica)                                │   │
│   │   • Elasticsearch (data)                                        │   │
│   │   • Kibana                                                      │   │
│   │   • HAProxy (standby)                                           │   │
│   │   • etcd node 2                                                 │   │
│   └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │                     SERVER 3 (Tertiary)                         │   │
│   │                                                                 │   │
│   │   • PostgreSQL (Patroni replica)                                │   │
│   │   • Elasticsearch (data)                                        │   │
│   │   • Logstash                                                    │   │
│   │   • pgBackRest repository                                       │   │
│   │   • etcd node 3                                                 │   │
│   └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │                     SHARED STORAGE (NAS)                        │   │
│   │                                                                 │   │
│   │   • Backups (pgBackRest repo)                                   │   │
│   │   • Elasticsearch snapshots                                     │   │
│   │   • Jira attachments                                            │   │
│   └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Technology Stack

### Management Tools

| Tool | Purpose | Why This Choice |
|------|---------|-----------------|
| **Ansible** | Configuration management | Reproducible setup, easy updates, agentless |
| **systemd** | Service management | Built-in, reliable, well-understood |
| **Patroni** | PostgreSQL HA | Automatic failover, battle-tested |
| **keepalived** | VIP failover for HAProxy | Simple, mature, single purpose |
| **Prometheus + Grafana** | Monitoring | Lightweight, powerful, standard |

### Application Stack

| Application | HA Mechanism | Notes |
|-------------|--------------|-------|
| **PostgreSQL** | Patroni (3 nodes) | Automatic failover < 30s |
| **Elasticsearch** | Native clustering | 3 data nodes minimum |
| **Jira** | Active-passive | Use shared DB and storage |
| **HAProxy** | keepalived VIP | Floating IP between 2 proxies |

---

## If You Want Containers (Without K8s)

Use **Docker Compose** on each server instead of Kubernetes:

```yaml
# /opt/stack/docker-compose.yml (per server)
version: '3.8'

services:
  postgresql:
    image: postgres:16
    volumes:
      - pg_data:/var/lib/postgresql/data
      - ./patroni/patroni.yml:/etc/patroni/patroni.yml
    network_mode: host
    restart: unless-stopped

  elasticsearch:
    image: elasticsearch:8.11.0
    environment:
      - node.name=${HOSTNAME}
      - cluster.name=bike-fleet-logs
      - discovery.seed_hosts=server1,server2,server3
      - cluster.initial_master_nodes=server1,server2,server3
      - bootstrap.memory_lock=true
      - "ES_JAVA_OPTS=-Xms4g -Xmx4g"
    ulimits:
      memlock:
        soft: -1
        hard: -1
    volumes:
      - es_data:/usr/share/elasticsearch/data
    network_mode: host
    restart: unless-stopped

  jira:
    image: atlassian/jira-software:latest
    environment:
      - ATL_JDBC_URL=jdbc:postgresql://localhost:5432/jira
      - ATL_JDBC_USER=jira
      - ATL_JDBC_PASSWORD=${JIRA_DB_PASSWORD}
      - ATL_DB_DRIVER=org.postgresql.Driver
      - ATL_DB_TYPE=postgres72
    volumes:
      - jira_data:/var/atlassian/application-data/jira
    ports:
      - "8080:8080"
    restart: unless-stopped
    profiles:
      - primary  # Only run on primary server

volumes:
  pg_data:
  es_data:
  jira_data:
```

### Docker Compose Benefits Over K8s

- Same container benefits (isolation, reproducibility)
- Much simpler operations
- No control plane overhead
- Easy to understand and debug
- No PVC/StorageClass complexity
- Familiar tools (docker logs, docker exec)

---

## When TO Consider Kubernetes

Move to Kubernetes when ANY of these become true:

| Trigger | Threshold |
|---------|-----------|
| Number of applications | 10+ different apps |
| Need auto-scaling | Load varies 10x+ |
| Team size | Multiple teams deploying independently |
| Architecture | Stateless microservices |
| DevOps capacity | Dedicated platform team |

**K3s** is lighter than K8s but still brings complexity you don't need for 2-3 clients.

---

## Comparison Summary

| Approach | Complexity | HA Capable | Best For |
|----------|------------|------------|----------|
| **3 VMs + Ansible + systemd** | Low | ✅ | **Our use case** |
| Docker Compose per server | Medium | ✅ | Container preference |
| K3s | High | ✅ | Edge/IoT with K8s familiarity |
| Full K8s | Very High | ✅ | Enterprise scale (50+ services) |

---

## Hardware Recommendations

### Minimum for 99.99% Uptime

| Server | CPU | RAM | Storage | Role |
|--------|-----|-----|---------|------|
| Server 1 | 8 cores | 32 GB | 500 GB NVMe | Primary services |
| Server 2 | 8 cores | 32 GB | 500 GB NVMe | Secondary/failover |
| Server 3 | 8 cores | 32 GB | 500 GB NVMe | Tertiary + backup |
| NAS | - | - | 4 TB+ | Shared backup storage |

### Network Requirements

- 10 Gbps between servers (replication traffic)
- Redundant NICs (bonding/teaming)
- Separate VLAN for database traffic (recommended)
- UPS on all servers

---

## Failure Scenarios

| Component Failure | Impact | Recovery |
|-------------------|--------|----------|
| Server 1 (primary PG) | < 30s failover | Patroni promotes replica |
| Server 2 | Reduced redundancy | Cluster continues on 2 nodes |
| Server 3 | No backups | Restore backup capability first |
| Network partition | Split-brain prevention | etcd quorum decides primary |
| NAS failure | No new backups | Local backups continue |

---

## Decision Record

**Date:** 2024-01
**Decision:** Use VM/bare-metal with Ansible, not Kubernetes
**Status:** Accepted

**Context:**
- 2-3 client computers
- High availability required (99.99%)
- Low load (< 100 requests/minute)
- Small ops team
- ~5 applications (Fleet app, PostgreSQL, ELK, Jira)

**Decision:**
Deploy on 3 VMs/bare-metal servers using:
- Ansible for configuration management
- systemd for service management
- Patroni for PostgreSQL HA
- Native clustering for Elasticsearch
- keepalived for HAProxy VIP

**Consequences:**
- ✅ Simple operations
- ✅ Easy troubleshooting
- ✅ Low resource overhead
- ✅ Team can understand entire stack
- ✅ Standard Linux skills sufficient
- ❌ Manual scaling (acceptable for our load)
- ❌ No GitOps/declarative deployments (acceptable)

**Revisit when:**
- Application count exceeds 10
- Team grows to need independent deployments
- Load requires auto-scaling
