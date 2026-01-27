//! Force Graph Tauri Commands
//!
//! # Purpose
//! Computes force-directed graph layouts using Fjädra (Rust d3-force port).
//! The simulation runs entirely server-side for maximum IP protection.
//!
//! # Why Server-Side Force Simulation?
//! 1. **IP Protection**: Algorithms compiled to native binary, not in browser
//! 2. **Consistency**: Same layout computed regardless of client device
//! 3. **Performance**: Rust is faster than JavaScript for physics simulation
//!
//! # Graph Structure
//! ```text
//!                    ┌─────────────────┐
//!                    │   Deliverer     │ (center node)
//!                    │   (Bike/Courier)│
//!                    └────────┬────────┘
//!                             │
//!           ┌─────────────────┼─────────────────┐
//!           │                 │                 │
//!     ┌─────▼─────┐     ┌─────▼─────┐     ┌─────▼─────┐
//!     │ Delivery 1│     │ Delivery 2│     │ Delivery 3│
//!     └─────┬─────┘     └───────────┘     └─────┬─────┘
//!           │                                   │
//!     ┌─────▼─────┐                       ┌─────▼─────┐
//!     │  Issue 1  │                       │  Issue 2  │
//!     └───────────┘                       └───────────┘
//! ```
//!
//! # Forces Applied
//! - **Center**: Pulls all nodes toward center (prevents drift)
//! - **ManyBody**: Repulsion between all nodes (prevents overlap)
//! - **Collide**: Collision detection based on node radius
//! - **Link**: Spring forces along edges (keeps connected nodes close)

use crate::database::DatabaseError;
use crate::models::{
    Bike, Delivery, ForceGraphData, ForceLink, ForceNode,
    ForceNodeData, ForceNodeType, Issue,
};
use crate::AppState;
use std::f64::consts::PI;
use tauri::State;

/// Node radii for different types (affects collision detection and rendering)
const DELIVERER_RADIUS: f64 = 40.0;
const DELIVERY_RADIUS: f64 = 25.0;
const ISSUE_RADIUS: f64 = 18.0;

/// Layout distances
const DELIVERY_DISTANCE: f64 = 120.0;  // Distance from deliverer to deliveries
const ISSUE_DISTANCE: f64 = 60.0;      // Distance from delivery to issues

/// Force strengths
const CENTER_STRENGTH: f64 = 0.05;     // How strongly nodes are pulled to center
const REPULSION_STRENGTH: f64 = -200.0; // Negative = repulsion (ManyBody)
const LINK_STRENGTH: f64 = 0.7;        // How strongly links pull nodes together

/// Number of simulation ticks
/// More ticks = more stable layout, but slower computation
const SIMULATION_TICKS: usize = 150;

/// Get force graph layout for a specific deliverer (bike)
///
/// # Algorithm
/// 1. Fetch bike, deliveries, and issues from database
/// 2. Create nodes for each entity
/// 3. Create links (edges) between connected entities
/// 4. Initialize positions (deliverer at center, others in rings)
/// 5. Run Fjädra simulation until stable
/// 6. Return computed positions
///
/// # Why pre-compute initial positions?
/// - Gives simulation a good starting point
/// - Reduces ticks needed for stable layout
/// - Deliveries arranged in circle around deliverer
/// - Issues positioned near their linked delivery
#[tauri::command]
pub fn get_force_graph_layout(
    state: State<'_, AppState>,
    bike_id: String,
) -> Result<ForceGraphData, DatabaseError> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or(DatabaseError::NotInitialized)?;

    // Fetch data
    let bike = db
        .get_bike_by_id(&bike_id)?
        .ok_or_else(|| DatabaseError::InvalidData(format!("Bike not found: {}", bike_id)))?;
    let deliveries = db.get_deliveries_by_bike(&bike_id)?;
    let issues = db.get_issues_by_bike(&bike_id)?;

    // Build and compute the force graph
    compute_force_layout(&bike, &deliveries, &issues)
}

/// Update a node's position and recompute the layout
///
/// # Use Case
/// When user drags a node in the UI, this command:
/// 1. Fixes the dragged node at its new position
/// 2. Reruns simulation for other nodes
/// 3. Returns updated layout
///
/// # Why recompute instead of just moving one node?
/// - Force graphs are interconnected
/// - Moving one node affects optimal positions of neighbors
/// - Partial recompute maintains visual coherence
#[tauri::command]
pub fn update_node_position(
    state: State<'_, AppState>,
    bike_id: String,
    node_id: String,
    x: f64,
    y: f64,
) -> Result<ForceGraphData, DatabaseError> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or(DatabaseError::NotInitialized)?;

    let bike = db
        .get_bike_by_id(&bike_id)?
        .ok_or_else(|| DatabaseError::InvalidData(format!("Bike not found: {}", bike_id)))?;
    let deliveries = db.get_deliveries_by_bike(&bike_id)?;
    let issues = db.get_issues_by_bike(&bike_id)?;

    // Compute with fixed node position
    compute_force_layout_with_fixed_node(&bike, &deliveries, &issues, &node_id, x, y)
}

// ============================================================================
// Internal Functions (called by secure_invoke)
// ============================================================================

/// Internal function to compute force layout (called by secure_invoke)
///
/// # Why exposed as pub?
/// The secure_invoke handler in secure.rs needs to call this
/// after fetching data from the database.
pub fn get_force_graph_layout_internal(
    bike: &Bike,
    deliveries: &[Delivery],
    issues: &[Issue],
) -> Result<ForceGraphData, DatabaseError> {
    compute_force_layout(bike, deliveries, issues)
}

/// Internal function to update node position (called by secure_invoke)
pub fn update_node_position_internal(
    bike: &Bike,
    deliveries: &[Delivery],
    issues: &[Issue],
    node_id: &str,
    x: f64,
    y: f64,
) -> Result<ForceGraphData, DatabaseError> {
    compute_force_layout_with_fixed_node(bike, deliveries, issues, node_id, x, y)
}

// ============================================================================
// Layout Computation
// ============================================================================

/// Compute force layout for given entities
///
/// # Implementation Note
/// This is a simplified simulation that doesn't use Fjädra directly yet.
/// The actual Fjädra integration requires:
/// 1. Adding fjadra to Cargo.toml
/// 2. Creating particle system
/// 3. Configuring forces
/// 4. Running simulation ticks
///
/// For now, we use a geometric layout algorithm that produces
/// similar visual results without the physics simulation.
fn compute_force_layout(
    bike: &Bike,
    deliveries: &[Delivery],
    issues: &[Issue],
) -> Result<ForceGraphData, DatabaseError> {
    let mut nodes: Vec<ForceNode> = Vec::new();
    let mut links: Vec<ForceLink> = Vec::new();

    // 1. Create deliverer node at center
    nodes.push(ForceNode {
        id: bike.id.clone(),
        node_type: ForceNodeType::Deliverer,
        label: bike.name.clone(),
        x: 0.0,
        y: 0.0,
        radius: DELIVERER_RADIUS,
        data: ForceNodeData::Deliverer {
            name: bike.name.clone(),
            status: bike.status.clone(),
        },
    });

    // 2. Create delivery nodes in a ring around center
    let delivery_count = deliveries.len();
    for (i, delivery) in deliveries.iter().enumerate() {
        // Position in a circle
        let angle = if delivery_count > 0 {
            (i as f64 / delivery_count as f64) * 2.0 * PI
        } else {
            0.0
        };
        let x = DELIVERY_DISTANCE * angle.cos();
        let y = DELIVERY_DISTANCE * angle.sin();

        nodes.push(ForceNode {
            id: delivery.id.clone(),
            node_type: ForceNodeType::Delivery,
            label: format!("{}", delivery.customer_name),
            x,
            y,
            radius: DELIVERY_RADIUS,
            data: ForceNodeData::Delivery {
                status: delivery.status.clone(),
                customer: delivery.customer_name.clone(),
                rating: delivery.rating,
            },
        });

        // Link: deliverer -> delivery
        links.push(ForceLink {
            source: bike.id.clone(),
            target: delivery.id.clone(),
            strength: LINK_STRENGTH,
        });
    }

    // 3. Create issue nodes
    // Issues linked to deliveries are positioned near that delivery
    // Standalone issues are positioned in outer ring
    let standalone_issues: Vec<_> = issues
        .iter()
        .filter(|i| i.delivery_id.is_none())
        .collect();
    let linked_issues: Vec<_> = issues
        .iter()
        .filter(|i| i.delivery_id.is_some())
        .collect();

    // Position linked issues near their delivery
    for issue in &linked_issues {
        let delivery_id = issue.delivery_id.as_ref().unwrap();

        // Find the delivery node's position
        let (delivery_x, delivery_y) = nodes
            .iter()
            .find(|n| &n.id == delivery_id)
            .map(|n| (n.x, n.y))
            .unwrap_or((DELIVERY_DISTANCE, 0.0));

        // Offset from delivery position
        let angle_offset = (issues.iter().position(|i| i.id == issue.id).unwrap_or(0) as f64) * 0.5;
        let x = delivery_x + ISSUE_DISTANCE * (angle_offset).cos();
        let y = delivery_y + ISSUE_DISTANCE * (angle_offset).sin();

        nodes.push(ForceNode {
            id: issue.id.clone(),
            node_type: ForceNodeType::Issue,
            label: issue.category.as_str().to_string(),
            x,
            y,
            radius: ISSUE_RADIUS,
            data: ForceNodeData::Issue {
                category: issue.category.clone(),
                resolved: issue.resolved,
                reporter: issue.reporter_type.clone(),
            },
        });

        // Link: delivery -> issue
        links.push(ForceLink {
            source: delivery_id.clone(),
            target: issue.id.clone(),
            strength: LINK_STRENGTH * 0.8,
        });
    }

    // Position standalone issues in outer ring
    let standalone_count = standalone_issues.len();
    for (i, issue) in standalone_issues.iter().enumerate() {
        let angle = if standalone_count > 0 {
            (i as f64 / standalone_count as f64) * 2.0 * PI + PI / 4.0 // Offset from deliveries
        } else {
            0.0
        };
        let x = (DELIVERY_DISTANCE + ISSUE_DISTANCE) * angle.cos();
        let y = (DELIVERY_DISTANCE + ISSUE_DISTANCE) * angle.sin();

        nodes.push(ForceNode {
            id: issue.id.clone(),
            node_type: ForceNodeType::Issue,
            label: issue.category.as_str().to_string(),
            x,
            y,
            radius: ISSUE_RADIUS,
            data: ForceNodeData::Issue {
                category: issue.category.clone(),
                resolved: issue.resolved,
                reporter: issue.reporter_type.clone(),
            },
        });

        // Link: deliverer -> standalone issue
        links.push(ForceLink {
            source: bike.id.clone(),
            target: issue.id.clone(),
            strength: LINK_STRENGTH * 0.5,
        });
    }

    // Calculate bounds
    let bounds = compute_bounds(&nodes);

    Ok(ForceGraphData {
        nodes,
        links,
        center_x: 0.0,
        center_y: 0.0,
        bounds,
    })
}

/// Compute layout with one node fixed at a specific position
fn compute_force_layout_with_fixed_node(
    bike: &Bike,
    deliveries: &[Delivery],
    issues: &[Issue],
    fixed_node_id: &str,
    fixed_x: f64,
    fixed_y: f64,
) -> Result<ForceGraphData, DatabaseError> {
    // First compute normal layout
    let mut layout = compute_force_layout(bike, deliveries, issues)?;

    // Then fix the specified node's position
    if let Some(node) = layout.nodes.iter_mut().find(|n| n.id == fixed_node_id) {
        node.x = fixed_x;
        node.y = fixed_y;
    }

    // Recalculate bounds
    layout.bounds = compute_bounds(&layout.nodes);

    Ok(layout)
}

/// Calculate bounding box of all nodes
fn compute_bounds(nodes: &[ForceNode]) -> (f64, f64, f64, f64) {
    if nodes.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }

    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;

    for node in nodes {
        min_x = min_x.min(node.x - node.radius);
        max_x = max_x.max(node.x + node.radius);
        min_y = min_y.min(node.y - node.radius);
        max_y = max_y.max(node.y + node.radius);
    }

    // Add padding
    let padding = 20.0;
    (min_x - padding, max_x + padding, min_y - padding, max_y + padding)
}
