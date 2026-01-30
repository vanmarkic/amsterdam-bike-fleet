//! PostgreSQL Force Graph Tauri Commands
//!
//! Async versions of force graph commands for PostgreSQL backend.

use crate::database_pg::DatabaseError;
use crate::models::{
    Bike, Delivery, ForceGraphData, ForceLink, ForceNode, ForceNodeData, ForceNodeType, Issue,
};
use crate::AppState;
use fjadra::force::{Center, Collide, Link, ManyBody, Node, SimulationBuilder};
use std::f64::consts::PI;
use tauri::State;

// Constants (same as SQLite version)
const DELIVERER_RADIUS: f64 = 40.0;
const DELIVERY_RADIUS: f64 = 25.0;
const ISSUE_RADIUS: f64 = 18.0;
const DELIVERY_DISTANCE: f64 = 120.0;
const ISSUE_DISTANCE: f64 = 60.0;
const CENTER_STRENGTH: f64 = 0.05;
const REPULSION_STRENGTH: f64 = -300.0;
const LINK_STRENGTH: f64 = 0.7;

/// Get force graph layout for a specific deliverer (bike)
#[tauri::command]
pub async fn get_force_graph_layout(
    state: State<'_, AppState>,
    bike_id: String,
) -> Result<ForceGraphData, DatabaseError> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or(DatabaseError::NotInitialized)?;

    // Fetch data
    let bike = db
        .get_bike_by_id(&bike_id)
        .await?
        .ok_or_else(|| DatabaseError::InvalidData(format!("Bike not found: {}", bike_id)))?;
    let deliveries = db.get_deliveries_by_bike(&bike_id).await?;
    let issues = db.get_issues_by_bike(&bike_id).await?;

    // Build and compute the force graph
    compute_force_layout(&bike, &deliveries, &issues, None)
}

/// Update a node's position and recompute the layout
#[tauri::command]
pub async fn update_node_position(
    state: State<'_, AppState>,
    bike_id: String,
    node_id: String,
    x: f64,
    y: f64,
) -> Result<ForceGraphData, DatabaseError> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or(DatabaseError::NotInitialized)?;

    let bike = db
        .get_bike_by_id(&bike_id)
        .await?
        .ok_or_else(|| DatabaseError::InvalidData(format!("Bike not found: {}", bike_id)))?;
    let deliveries = db.get_deliveries_by_bike(&bike_id).await?;
    let issues = db.get_issues_by_bike(&bike_id).await?;

    compute_force_layout(&bike, &deliveries, &issues, Some((&node_id, x, y)))
}

// ============================================================================
// Layout Computation (same algorithm as SQLite version)
// ============================================================================

struct NodeInfo {
    id: String,
    node_type: ForceNodeType,
    label: String,
    radius: f64,
    data: ForceNodeData,
    initial_x: f64,
    initial_y: f64,
}

fn compute_force_layout(
    bike: &Bike,
    deliveries: &[Delivery],
    issues: &[Issue],
    fixed_node: Option<(&str, f64, f64)>,
) -> Result<ForceGraphData, DatabaseError> {
    let mut node_infos: Vec<NodeInfo> = Vec::new();
    let mut links: Vec<ForceLink> = Vec::new();
    let mut link_indices: Vec<(usize, usize)> = Vec::new();
    let mut radii: Vec<f64> = Vec::new();

    // 1. Create deliverer node at center
    node_infos.push(NodeInfo {
        id: bike.id.clone(),
        node_type: ForceNodeType::Deliverer,
        label: bike.name.clone(),
        radius: DELIVERER_RADIUS,
        data: ForceNodeData::Deliverer {
            name: bike.name.clone(),
            status: bike.status.clone(),
        },
        initial_x: 0.0,
        initial_y: 0.0,
    });
    radii.push(DELIVERER_RADIUS);

    // 2. Create delivery nodes
    let delivery_count = deliveries.len();
    for (i, delivery) in deliveries.iter().enumerate() {
        let angle = if delivery_count > 0 {
            (i as f64 / delivery_count as f64) * 2.0 * PI
        } else {
            0.0
        };
        let x = DELIVERY_DISTANCE * angle.cos();
        let y = DELIVERY_DISTANCE * angle.sin();

        let delivery_index = node_infos.len();
        node_infos.push(NodeInfo {
            id: delivery.id.clone(),
            node_type: ForceNodeType::Delivery,
            label: delivery.customer_name.clone(),
            radius: DELIVERY_RADIUS,
            data: ForceNodeData::Delivery {
                status: delivery.status.clone(),
                customer: delivery.customer_name.clone(),
                rating: delivery.rating,
            },
            initial_x: x,
            initial_y: y,
        });
        radii.push(DELIVERY_RADIUS);

        links.push(ForceLink {
            source: bike.id.clone(),
            target: delivery.id.clone(),
            strength: LINK_STRENGTH,
        });
        link_indices.push((0, delivery_index));
    }

    // 3. Create issue nodes
    let standalone_issues: Vec<_> = issues.iter().filter(|i| i.delivery_id.is_none()).collect();
    let linked_issues: Vec<_> = issues.iter().filter(|i| i.delivery_id.is_some()).collect();

    for issue in &linked_issues {
        let delivery_id = issue.delivery_id.as_ref().unwrap();

        let (delivery_idx, delivery_x, delivery_y) = node_infos
            .iter()
            .enumerate()
            .find(|(_, n)| &n.id == delivery_id)
            .map(|(idx, n)| (idx, n.initial_x, n.initial_y))
            .unwrap_or((1, DELIVERY_DISTANCE, 0.0));

        let angle_offset =
            (issues.iter().position(|i| i.id == issue.id).unwrap_or(0) as f64) * 0.8;
        let x = delivery_x + ISSUE_DISTANCE * angle_offset.cos();
        let y = delivery_y + ISSUE_DISTANCE * angle_offset.sin();

        let issue_index = node_infos.len();
        node_infos.push(NodeInfo {
            id: issue.id.clone(),
            node_type: ForceNodeType::Issue,
            label: issue.category.as_str().to_string(),
            radius: ISSUE_RADIUS,
            data: ForceNodeData::Issue {
                category: issue.category.clone(),
                resolved: issue.resolved,
                reporter: issue.reporter_type.clone(),
            },
            initial_x: x,
            initial_y: y,
        });
        radii.push(ISSUE_RADIUS);

        links.push(ForceLink {
            source: delivery_id.clone(),
            target: issue.id.clone(),
            strength: LINK_STRENGTH * 0.8,
        });
        link_indices.push((delivery_idx, issue_index));
    }

    let standalone_count = standalone_issues.len();
    for (i, issue) in standalone_issues.iter().enumerate() {
        let angle = if standalone_count > 0 {
            (i as f64 / standalone_count as f64) * 2.0 * PI + PI / 4.0
        } else {
            0.0
        };
        let x = (DELIVERY_DISTANCE + ISSUE_DISTANCE) * angle.cos();
        let y = (DELIVERY_DISTANCE + ISSUE_DISTANCE) * angle.sin();

        let issue_index = node_infos.len();
        node_infos.push(NodeInfo {
            id: issue.id.clone(),
            node_type: ForceNodeType::Issue,
            label: issue.category.as_str().to_string(),
            radius: ISSUE_RADIUS,
            data: ForceNodeData::Issue {
                category: issue.category.clone(),
                resolved: issue.resolved,
                reporter: issue.reporter_type.clone(),
            },
            initial_x: x,
            initial_y: y,
        });
        radii.push(ISSUE_RADIUS);

        links.push(ForceLink {
            source: bike.id.clone(),
            target: issue.id.clone(),
            strength: LINK_STRENGTH * 0.5,
        });
        link_indices.push((0, issue_index));
    }

    // 4. Create Fjädra nodes
    let fixed_node_index = fixed_node.and_then(|(id, _, _)| {
        node_infos.iter().position(|n| n.id == id)
    });

    let particles: Vec<Node> = node_infos
        .iter()
        .enumerate()
        .map(|(idx, info)| {
            if let Some((fixed_id, fx, fy)) = fixed_node {
                if info.id == fixed_id {
                    return Node::default().fixed_position(fx, fy);
                }
            }
            if idx == 0 && fixed_node_index != Some(0) {
                return Node::default().fixed_position(0.0, 0.0);
            }
            Node::default().position(info.initial_x, info.initial_y)
        })
        .collect();

    // 5. Build and run simulation
    let radii_clone = radii.clone();
    let mut simulation = SimulationBuilder::default()
        .build(particles)
        .add_force("center", Center::new().strength(CENTER_STRENGTH))
        .add_force(
            "charge",
            ManyBody::new().strength(|_node_idx, _count| REPULSION_STRENGTH),
        )
        .add_force(
            "collide",
            Collide::new()
                .radius(move |i| radii_clone[i] + 5.0)
                .iterations(2),
        )
        .add_force("links", Link::new(link_indices).iterations(3));

    simulation.step();

    // 6. Extract positions
    let positions: Vec<[f64; 2]> = simulation.positions().collect();

    let nodes: Vec<ForceNode> = node_infos
        .into_iter()
        .enumerate()
        .map(|(i, info)| {
            let [x, y] = positions.get(i).copied().unwrap_or([info.initial_x, info.initial_y]);
            ForceNode {
                id: info.id,
                node_type: info.node_type,
                label: info.label,
                x,
                y,
                radius: info.radius,
                data: info.data,
            }
        })
        .collect();

    let bounds = compute_bounds(&nodes);

    Ok(ForceGraphData {
        nodes,
        links,
        center_x: 0.0,
        center_y: 0.0,
        bounds,
    })
}

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

    let padding = 20.0;
    (
        min_x - padding,
        max_x + padding,
        min_y - padding,
        max_y + padding,
    )
}
