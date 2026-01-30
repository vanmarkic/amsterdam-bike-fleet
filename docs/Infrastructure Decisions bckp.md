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

## Fair Arguments FOR Kubernetes

Despite our decision against K8s, these are legitimate reasons teams choose it:

### 1. True Self-Healing Across Infrastructure

Kubernetes provides **automatic rescheduling** when nodes fail—not just container restarts. If a physical server dies, pods automatically start on healthy nodes without manual intervention. Docker Compose restart policies only handle container crashes on a running host; they cannot survive Docker runtime or hardware failures.

### 2. Declarative GitOps and Drift Prevention

Kubernetes' desired-state model means the system **continuously reconciles** any configuration drift. Tools like ArgoCD and FluxCD enable pull-request-based infrastructure changes with automatic rollback. With VMs/Compose, you rely on discipline—nothing stops manual SSH changes from creating "snowflake" servers.

### 3. Ecosystem Critical Mass

With **88% adoption rate** and 75% of organizations using Helm, Kubernetes has achieved ecosystem dominance. Pre-built Helm charts exist for nearly every application. Kubernetes Operators encode domain-specific logic (backups, failover, upgrades) that would otherwise require custom scripting. The job market also reflects this—K8s appears in ~28% of DevOps job listings (2024).

> *"Once a tool reaches critical mass, the ecosystem around it becomes a primary motivation to adopt it."*

---

## Fair Arguments AGAINST Kubernetes

The other side of the coin—legitimate reasons teams avoid or abandon K8s:

### 4. Operational Complexity and Debugging Nightmares

**76% of users cite Kubernetes complexity as a barrier** to wider adoption (2024 Spectro Cloud report). Teams lose an average of **34 workdays annually on troubleshooting** K8s incidents alone (Komodor 2025). Debugging distributed systems is exponentially harder than `grep /var/log/syslog` on a traditional server.

**Real incident:** A junior developer's ConfigMap tweak (INFO→DEBUG) flooded node disks in 8 minutes, crashed kubelets cluster-wide, and required 45 minutes of manual SSH recovery.

### 5. Hidden Costs (Personnel, Training, Infrastructure)

Your cloud bill is just the tip of the iceberg:

| Cost Factor | Impact |
|-------------|--------|
| Operational overhead | ~35% of total K8s spending |
| Cloud waste from over-provisioned clusters | $200B annually industry-wide |
| Microservices on K8s vs monolith | 2.5-3.75x higher total cost |
| K8s expert salaries | Premium due to scarcity |

**Cost comparison:**
| Approach | Monthly Cost | Ops Hours/Month |
|----------|-------------|-----------------|
| EKS (Kubernetes) | $300-700 | 10-20 hours |
| Single VPS ("boring stack") | $30 | 1-2 hours |

### 6. Resume-Driven Development and Over-Engineering

**61% of organizations plan to shrink their Kubernetes footprint** over the next 12 months. **42% are consolidating microservices back to monoliths.** Teams "plan to scale to 100 microservices eventually," but "eventually" never comes—meanwhile they've spent months learning K8s instead of shipping product.

> *"When you choose boring technology, you spend less time configuring and more time shipping. You'll be shipping products while others are still configuring their YAML files."*

---

## Fair Arguments FOR Simple VM/Bare Metal

Why "boring" infrastructure often wins:

### 4. Dramatic Cost Savings

The evidence is overwhelming—bare metal can save **50-97%** compared to managed cloud:

- **$12.9M annual savings case study:** Company spending $1.06M/month on AWS reduced to $50k/month on bare metal—95% reduction
- **$12k/month savings:** Team migrated back from K8s to dedicated servers, saving $144k annually
- **Cloud egress fees eliminated:** "Cloud egress fees were eating our margins alive. Bare metal solved that day 1."

### 5. Simplicity, Debuggability, and "Boring Technology" Advantage

Dan McKinley's "Choose Boring Technology" principle: **well-understood systems have well-understood failure modes**.

| Tool | Why It Works |
|------|--------------|
| **Ansible** | Agentless (SSH), YAML syntax, no agents to manage |
| **systemd** | Every Linux admin knows it; `journalctl` just works |
| **Terraform** | Declarative, works with any provider, massive ecosystem |

> *"Every company gets about three innovation tokens. Spending them on container orchestration when your business doesn't require it wastes capacity for where innovation actually matters."* — Dan McKinley

### 6. Security, Compliance, and Hardware Isolation

Regulated industries favor bare metal for simpler security posture:

- **Single-tenant eliminates multi-tenant risks:** "When you're the only tenant on a physical server, entire categories of risks disappear."
- **Container compliance is harder:** Traditional infra uses periodic scans; containers need continuous, automated controls
- **Shared kernel risk:** All containers share the host kernel—a compromise in one can affect others
- **Simpler audit trail:** Single box, single purpose, direct hardware control (BIOS, NUMA, passthrough)

---

## Fair Arguments AGAINST Simple VM/Docker Compose

Our chosen approach has real limitations worth acknowledging:

### 1. Configuration Drift Risk

Managing multiple VMs individually leads to **human error and divergence** over time. Without Kubernetes' continuous reconciliation, environments can become inconsistent. Auditing "what's actually deployed vs. what should be" requires external tooling.

### 2. Manual Recovery Burden

Docker Compose health checks and restart policies provide **container-level recovery only**. A host outage, network partition, or Docker daemon failure requires manual intervention. Updates often require `docker-compose build && docker-compose up -d` with potential downtime.

### 3. No Industry Standardization

Kubernetes is the **de facto enterprise standard**. Companies selling to Global 2000 enterprises report customers expect Kubernetes deployments. Our simpler approach works but may create friction with enterprise compliance requirements or cloud migration paths.

---

## Middle-Ground: Container Orchestration Alternatives

If you want more than Docker Compose but less than full Kubernetes, consider these options:

### Docker Swarm

**Status:** Actively maintained by Mirantis; 100+ customers running 10,000+ nodes in production.

| Aspect | Details |
|--------|---------|
| **Setup** | Built into Docker Engine—zero additional software |
| **Learning curve** | Minimal; uses native Docker CLI |
| **HA** | Built-in multi-node clustering |
| **Migration** | Direct path from Docker Compose (`docker stack deploy`) |

```yaml
# docker-compose.yml for Swarm
version: '3.8'
services:
  web:
    image: nginx:alpine
    deploy:
      replicas: 3
      update_config:
        parallelism: 1
        delay: 10s
      restart_policy:
        condition: on-failure
```

**Verdict:** Excellent fit for our use case if we want container orchestration without K8s complexity.

### HashiCorp Nomad

**Status:** Production-ready; used at Cloudflare, Pandora; tested with 2M containers.

| Aspect | Details |
|--------|---------|
| **Setup** | Single binary, no external dependencies |
| **Learning curve** | Moderate (HCL syntax) |
| **Unique strength** | Orchestrates containers, VMs, Java apps, batch jobs |
| **Integration** | Native Vault, Consul, Terraform support |

**Verdict:** Strong fit if you have mixed workloads (containers + legacy VMs) or already use HashiCorp tools.

### K3s (Lightweight Kubernetes)

**Status:** CNCF certified; <70MB binary; maintained by SUSE.

| Aspect | Details |
|--------|---------|
| **Setup** | Single binary, installs in <30 seconds |
| **Resources** | Minimum 512MB RAM, 1 CPU |
| **Ecosystem** | Full Kubernetes API—all Helm charts work |
| **HA** | Embedded etcd or external datastore |

**Verdict:** Best choice if you anticipate future Kubernetes migration or need K8s ecosystem access today.

### Podman + Quadlet (systemd)

**Status:** Mature; daemonless; Red Hat supported.

| Aspect | Details |
|--------|---------|
| **Setup** | Native Linux; systemd manages containers |
| **Security** | Rootless by default |
| **Multi-node** | Manual (pair with Ansible) |
| **K8s compat** | `podman play kube` runs K8s manifests |

**Verdict:** Good for single-node reliability with maximum security; not a multi-node orchestrator.

### Comparison Matrix

| Criteria | Docker Swarm | Nomad | K3s | Podman+systemd |
|----------|-------------|-------|-----|----------------|
| **Setup complexity** | Very Low | Low | Medium | Low |
| **Learning curve** | Minimal | Moderate | Higher | Low |
| **Multi-node HA** | Built-in | Built-in | Built-in | Manual |
| **Resource overhead** | Minimal | Minimal | Low | Minimal |
| **Ecosystem** | Docker | HashiCorp | Kubernetes | Linux native |
| **Future-proofing** | Limited | Good | Excellent | Limited |

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
| Docker Compose per server | Low-Medium | ✅ | Container preference, single-host |
| **Docker Swarm** | Low | ✅ | Multi-host containers without K8s |
| Podman + Quadlet | Low | ⚠️ Manual | Rootless security, single-host |
| HashiCorp Nomad | Medium | ✅ | Mixed workloads (containers + VMs) |
| K3s | Medium-High | ✅ | K8s ecosystem access, edge/IoT |
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

---

## References

### Kubernetes vs Docker Compose
- [Spacelift: Docker Compose vs Kubernetes](https://spacelift.io/blog/docker-compose-vs-kubernetes)
- [DataCamp: Docker Compose vs Kubernetes Comparison](https://www.datacamp.com/blog/docker-compose-vs-kubernetes)
- [Devtron: Top 5 Reasons to Migrate to Kubernetes](https://devtron.ai/blog/top-5-reasons-to-migration-from-docker-compose-to-kubernetes/)
- [Better Stack: Docker Compose vs Kubernetes](https://betterstack.com/community/guides/scaling-docker/docker-compose-vs-kubernetes/)

### Kubernetes Ecosystem
- [Syntasso: Kubernetes Operators in 2025](https://www.syntasso.io/post/what-are-kubernetes-operators-and-do-you-still-need-them-in-2025)
- [IBM: What is Helm in Kubernetes?](https://www.ibm.com/think/topics/helm)
- [Komodor: 14 Kubernetes Best Practices 2025](https://komodor.com/learn/14-kubernetes-best-practices-you-must-know-in-2025/)
- [Ardan Labs: Kubernetes in Today's Job Market](https://www.ardanlabs.com/news/2025/how-important-is-knowing-kubernetes-in-todays-job-market/)

### Alternative Orchestrators
- [Docker Swarm in 2025](https://medium.com/@niksa.makitan/docker-swarm-in-2025-0d2f2bc5d929)
- [Mirantis: Swarm Commitment](https://www.mirantis.com/blog/swarm-is-here-to-stay-and-keeps-getting-better-in-security-and-ease-of-operations/)
- [Nomad Documentation](https://developer.hashicorp.com/nomad/docs/what-is-nomad)
- [K3s Documentation](https://docs.k3s.io)
- [Podman Quadlet - DEV Community](https://dev.to/mcheremnov/podman-quadlet-modern-systemd-integration-2i7g)
