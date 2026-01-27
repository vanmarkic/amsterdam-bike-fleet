# Design: Rust Backend API + Fjädra Force Graph

## Overview

Move data fetching from client-side mock interceptor to Rust/Tauri backend with encrypted IPC, and implement a force-directed graph visualization for deliverer→deliveries→issues relationships using Fjädra.

## Goals

1. **Maximum reverse-engineering resistance**: Encrypted IPC, compiled Rust, no algorithms in browser
2. **Unified data layer**: All data (bikes, deliveries, issues) served from Tauri commands
3. **Interactive force graph**: Visualize deliverer relationships with Fjädra-computed layouts

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  ANGULAR FRONTEND                                               │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  ForceGraphComponent (SVG/Canvas renderer)                  ││
│  │  - Receives computed (x,y) positions from Tauri            ││
│  │  - Handles drag events, sends to backend for recompute     ││
│  │  - Renders nodes/links with D3-selection (no d3-force)     ││
│  └─────────────────────────────────────────────────────────────┘│
│                              │                                   │
│                  invoke('get_force_graph_layout', {bikeId})     │
│                              ▼                                   │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  TauriService (encrypted IPC wrapper)                       ││
│  │  - Derives session key from license                         ││
│  │  - Encrypts/decrypts command payloads (ChaCha20-Poly1305)  ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
                              │
                    Tauri IPC (encrypted)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  TAURI BACKEND (Rust native binary)                             │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  commands/force_graph.rs                                    ││
│  │  - get_force_graph_layout(bike_id) → ForceGraphData        ││
│  │  - update_node_position(node_id, x, y) → ForceGraphData    ││
│  └─────────────────────────────────────────────────────────────┘│
│                              │                                   │
│  ┌───────────────────────────▼─────────────────────────────────┐│
│  │  force_graph.rs (Fjädra simulation)                         ││
│  │  - Builds graph: Deliverer(center) → Deliveries → Issues   ││
│  │  - Configures forces: Center, Link, ManyBody, Collision    ││
│  │  - Runs simulation ticks (100-300)                          ││
│  │  - Returns stable positions                                 ││
│  └─────────────────────────────────────────────────────────────┘│
│                              │                                   │
│  ┌───────────────────────────▼─────────────────────────────────┐│
│  │  database.rs (extended)                                      ││
│  │  - deliveries table                                          ││
│  │  - issues table                                              ││
│  │  - get_deliveries_by_bike(), get_issues_by_bike()           ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

---

## Data Model

### Force Graph Nodes

```rust
#[derive(Serialize, Deserialize)]
pub enum NodeType {
    Deliverer,   // Center node (the bike/courier)
    Delivery,    // Connected to deliverer
    Issue,       // Connected to delivery or directly to deliverer
}

#[derive(Serialize, Deserialize)]
pub struct ForceNode {
    pub id: String,
    pub node_type: NodeType,
    pub label: String,
    pub x: f64,          // Computed by Fjädra
    pub y: f64,          // Computed by Fjädra
    pub radius: f64,     // For collision detection
    pub data: NodeData,  // Type-specific payload
}

#[derive(Serialize, Deserialize)]
pub enum NodeData {
    Deliverer { name: String, status: BikeStatus },
    Delivery { status: DeliveryStatus, customer: String, rating: Option<u8> },
    Issue { category: IssueCategory, resolved: bool },
}
```

### Force Graph Links

```rust
#[derive(Serialize, Deserialize)]
pub struct ForceLink {
    pub source: String,  // Node ID
    pub target: String,  // Node ID
    pub strength: f64,   // Link strength (0.0 - 1.0)
}
```

### Complete Response

```rust
#[derive(Serialize, Deserialize)]
pub struct ForceGraphData {
    pub nodes: Vec<ForceNode>,
    pub links: Vec<ForceLink>,
    pub center_x: f64,
    pub center_y: f64,
    pub bounds: (f64, f64, f64, f64), // min_x, max_x, min_y, max_y
}
```

---

## Database Schema Changes

```sql
-- Add to initialize_schema()

CREATE TABLE IF NOT EXISTS deliveries (
    id TEXT PRIMARY KEY,
    bike_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'upcoming',
    customer_name TEXT NOT NULL,
    customer_address TEXT NOT NULL,
    restaurant_name TEXT NOT NULL,
    restaurant_address TEXT NOT NULL,
    rating INTEGER,
    complaint TEXT,
    created_at TEXT NOT NULL,
    completed_at TEXT,
    FOREIGN KEY (bike_id) REFERENCES bikes(id)
);

CREATE TABLE IF NOT EXISTS issues (
    id TEXT PRIMARY KEY,
    delivery_id TEXT,
    bike_id TEXT NOT NULL,
    reporter_type TEXT NOT NULL,
    category TEXT NOT NULL,
    description TEXT NOT NULL,
    resolved INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (delivery_id) REFERENCES deliveries(id),
    FOREIGN KEY (bike_id) REFERENCES bikes(id)
);

CREATE INDEX IF NOT EXISTS idx_deliveries_bike_id ON deliveries(bike_id);
CREATE INDEX IF NOT EXISTS idx_issues_bike_id ON issues(bike_id);
CREATE INDEX IF NOT EXISTS idx_issues_delivery_id ON issues(delivery_id);
```

---

## Encrypted IPC Implementation

### Key Derivation (session startup)

```rust
// In crypto.rs
use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;

pub struct SessionCrypto {
    cipher: ChaCha20Poly1305,
    nonce_counter: AtomicU64,
}

impl SessionCrypto {
    pub fn from_license(license_key: &str, session_nonce: &[u8; 16]) -> Self {
        // Derive 256-bit key using HKDF
        let ikm = license_key.as_bytes();
        let salt = session_nonce;
        let info = b"amsterdam-bike-fleet-ipc-v1";

        let hk = Hkdf::<Sha256>::new(Some(salt), ikm);
        let mut key = [0u8; 32];
        hk.expand(info, &mut key).unwrap();

        let cipher = ChaCha20Poly1305::new(&key.into());

        Self {
            cipher,
            nonce_counter: AtomicU64::new(0),
        }
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Vec<u8> { ... }
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> { ... }
}
```

### Command Wrapper Pattern

```rust
// Instead of exposing raw commands, wrap with encryption
#[tauri::command]
pub fn secure_invoke(
    state: State<'_, AppState>,
    encrypted_payload: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let crypto = state.crypto.lock().unwrap();

    // Decrypt request
    let payload = crypto.decrypt(&encrypted_payload)
        .map_err(|e| e.to_string())?;

    // Parse and route command
    let cmd: SecureCommand = bincode::deserialize(&payload)
        .map_err(|e| e.to_string())?;

    let result = match cmd {
        SecureCommand::GetForceGraphLayout { bike_id } => {
            bincode::serialize(&get_force_graph_layout_internal(&state, &bike_id)?)
        }
        // ... other commands
    }?;

    // Encrypt response
    Ok(crypto.encrypt(&result))
}
```

---

## Fjädra Force Simulation

```rust
// In force_graph.rs
use fjadra::{Center, Collide, Link, ManyBody, Simulation};

pub fn compute_force_layout(
    deliverer: &Bike,
    deliveries: &[Delivery],
    issues: &[Issue],
) -> ForceGraphData {
    // Build particles (nodes)
    let mut positions: Vec<[f64; 2]> = Vec::new();
    let mut nodes: Vec<ForceNode> = Vec::new();

    // 1. Add deliverer at center
    positions.push([0.0, 0.0]);
    nodes.push(ForceNode {
        id: deliverer.id.clone(),
        node_type: NodeType::Deliverer,
        label: deliverer.name.clone(),
        x: 0.0,
        y: 0.0,
        radius: 30.0,
        data: NodeData::Deliverer {
            name: deliverer.name.clone(),
            status: deliverer.status.clone(),
        },
    });

    // 2. Add deliveries in a ring around center
    for (i, delivery) in deliveries.iter().enumerate() {
        let angle = (i as f64 / deliveries.len() as f64) * 2.0 * PI;
        let x = 100.0 * angle.cos();
        let y = 100.0 * angle.sin();
        positions.push([x, y]);
        nodes.push(ForceNode {
            id: delivery.id.clone(),
            node_type: NodeType::Delivery,
            // ...
        });
    }

    // 3. Add issues (linked to deliveries or directly to deliverer)
    for issue in issues {
        // Position near linked delivery or random around deliverer
        // ...
    }

    // Build links
    let mut links: Vec<(usize, usize)> = Vec::new();
    // Deliverer → each delivery
    for i in 1..=deliveries.len() {
        links.push((0, i));
    }
    // Delivery → linked issues
    // ...

    // Configure simulation
    let mut simulation = Simulation::new(positions.len())
        .with_force(
            "center",
            Center::new().strength(0.1)
        )
        .with_force(
            "charge",
            ManyBody::new().strength(-300.0)
        )
        .with_force(
            "collision",
            Collide::new().radius(|i| nodes[i].radius)
        )
        .with_force(
            "links",
            Link::new(links).strength(0.7).distance(80.0)
        );

    // Initialize positions
    for (i, pos) in positions.iter().enumerate() {
        simulation.set_position(i, *pos);
    }

    // Run simulation until stable (alpha < 0.001)
    while simulation.alpha() > 0.001 {
        simulation.tick();
    }

    // Extract final positions
    for (i, node) in nodes.iter_mut().enumerate() {
        let pos = simulation.position(i);
        node.x = pos[0];
        node.y = pos[1];
    }

    ForceGraphData {
        nodes,
        links: /* convert to ForceLink */,
        center_x: 0.0,
        center_y: 0.0,
        bounds: compute_bounds(&nodes),
    }
}
```

---

## Angular Component (Rendering Only)

The frontend component does **not** run force simulation - it only renders:

```typescript
// force-graph.component.ts
@Component({
  selector: 'app-force-graph',
  template: `
    <svg #svgElement [attr.viewBox]="viewBox">
      <g class="links">
        <line *ngFor="let link of links"
              [attr.x1]="getNode(link.source).x"
              [attr.y1]="getNode(link.source).y"
              [attr.x2]="getNode(link.target).x"
              [attr.y2]="getNode(link.target).y"
              [class]="getLinkClass(link)" />
      </g>
      <g class="nodes">
        <g *ngFor="let node of nodes"
           [attr.transform]="'translate(' + node.x + ',' + node.y + ')'"
           (mousedown)="onDragStart($event, node)"
           [class]="getNodeClass(node)">
          <circle [attr.r]="node.radius" />
          <text>{{ node.label }}</text>
        </g>
      </g>
    </svg>
  `
})
export class ForceGraphComponent {
  @Input() bikeId!: string;

  nodes: ForceNode[] = [];
  links: ForceLink[] = [];

  async loadGraph() {
    const data = await this.tauriService.invokeSecure<ForceGraphData>(
      'get_force_graph_layout',
      { bikeId: this.bikeId }
    );
    this.nodes = data.nodes;
    this.links = data.links;
  }

  async onDragEnd(node: ForceNode, x: number, y: number) {
    // Send new position to backend for re-simulation
    const data = await this.tauriService.invokeSecure<ForceGraphData>(
      'update_node_position',
      { nodeId: node.id, x, y }
    );
    this.nodes = data.nodes;
    this.links = data.links;
  }
}
```

---

## New Dependencies

### Tauri (Cargo.toml)

```toml
[dependencies]
# Existing...

# Encryption
chacha20poly1305 = "0.10"
hkdf = "0.12"

# Binary serialization (smaller, faster, harder to inspect)
bincode = "1.3"

# Force simulation
fjadra = "0.1"  # Check latest version
```

### Angular (package.json)

```json
{
  "dependencies": {
    "d3-selection": "^3.0.0",  // For DOM manipulation only
    "d3-drag": "^3.0.0"        // For drag interaction
  }
}
```

Note: We do NOT need `d3-force` since Fjädra handles the simulation.

---

## File Changes Summary

### New Files

| File | Purpose |
|------|---------|
| `src-tauri/src/crypto.rs` | Session key derivation, ChaCha20 encrypt/decrypt |
| `src-tauri/src/force_graph.rs` | Fjädra simulation, graph building |
| `src-tauri/src/commands/force_graph.rs` | Tauri commands for force graph |
| `src-tauri/src/commands/deliveries.rs` | CRUD commands for deliveries |
| `src-tauri/src/commands/issues.rs` | CRUD commands for issues |
| `src/app/components/force-graph/` | Angular component for rendering |
| `src/app/services/tauri.service.ts` | Encrypted IPC wrapper |

### Modified Files

| File | Changes |
|------|---------|
| `src-tauri/Cargo.toml` | Add chacha20poly1305, hkdf, bincode, fjadra |
| `src-tauri/src/lib.rs` | Register new commands, init crypto |
| `src-tauri/src/database.rs` | Add deliveries/issues tables, queries |
| `src-tauri/src/models.rs` | Add Delivery, Issue, ForceNode, ForceLink |
| `src/app/app-routing.module.ts` | Add route for force graph view |

---

## Implementation Order

1. **Database layer**: Add deliveries/issues tables, migrate mock data
2. **Crypto module**: Implement encrypted IPC wrapper
3. **Tauri commands**: Add delivery/issue CRUD with encryption
4. **Fjädra integration**: Implement force graph computation
5. **Angular service**: Create TauriService with encrypted invoke
6. **Angular component**: Build ForceGraphComponent with drag support
7. **Integration**: Wire up bike selection → force graph view

---

## Verification

1. **Encrypted IPC**:
   - Intercept Tauri IPC with devtools → verify payload is encrypted binary
   - Attempt to parse as JSON → should fail

2. **Force graph layout**:
   - Select a deliverer with 5+ deliveries and 2+ issues
   - Verify nodes don't overlap (collision working)
   - Verify links connect correctly
   - Drag a node → verify re-layout maintains structure

3. **Data persistence**:
   - Create delivery via Tauri command
   - Restart app → verify delivery persists
   - Check SQLite file directly → data present

4. **Reverse engineering resistance**:
   - Inspect WASM binary → no Fjädra algorithms (runs in Tauri)
   - Inspect Angular bundle → only rendering code, no simulation
   - Inspect IPC traffic → encrypted blobs only
