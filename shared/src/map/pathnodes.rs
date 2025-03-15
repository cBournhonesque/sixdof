use bevy::{prelude::*, scene::SceneInstance};
use avian3d::prelude::*;

#[derive(Resource)]
pub struct PathfindingGraph {
    pub nodes: Vec<Vec3>,
    pub connections: Vec<Vec<usize>>,
    pub node_spacing: f32,
}

impl Default for PathfindingGraph {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            connections: Vec::new(),
            node_spacing: 5.0,
        }
    }
}

pub fn generate_pathfinding_nodes_system(
    mut commands: Commands,
    spatial_query: SpatialQuery,
    world_loaded: Query<(), Changed<SceneInstance>>,
) {
    if !world_loaded.is_empty() {
        let mut graph = PathfindingGraph::default();
        
        let min_bounds = Vec3::new(-1000.0, -1000.0, -1000.0);
        let max_bounds = Vec3::new(1000.0, 1000.0, 1000.0);
        
        let step = graph.node_spacing;
        
        // Generate a grid of potential nodes
        for x in (min_bounds.x as i32..=max_bounds.x as i32).step_by(step as usize) {
            for y in (min_bounds.y as i32..=max_bounds.y as i32).step_by(step as usize) {
                for z in (min_bounds.z as i32..=max_bounds.z as i32).step_by(step as usize) {
                    let position = Vec3::new(x as f32, y as f32, z as f32);
                    if !position_collides(&spatial_query, position) {
                        graph.nodes.push(position);
                        graph.connections.push(Vec::new());
                    } else {
                        info!("Pruning node at: {:?}", position);
                    }
                }
            }
        }
        
        // // Connect nodes that have line of sight
        // connect_nodes(&mut graph, &spatial_query);
    
        // Store as a resource
        commands.insert_resource(graph);
    }
}

fn position_collides(spatial_query: &SpatialQuery, position: Vec3) -> bool {
    spatial_query.point_intersections(
        position, 
        // @todo-brian: filter out certain layers
        &SpatialQueryFilter::default()
    ).len() > 0
}

fn connect_nodes(graph: &mut PathfindingGraph, spatial_query: &SpatialQuery) {
    let max_connection_dist = graph.node_spacing * 1.5;
    
    for i in 0..graph.nodes.len() {
        for j in (i+1)..graph.nodes.len() {
            let a_pos = graph.nodes[i];
            let b_pos = graph.nodes[j];
            
            let dist = a_pos.distance(b_pos);
            
            // Only connect reasonably close nodes
            // Raycast to check line of sight
            if dist <= max_connection_dist {
                let dir = Dir3::new((b_pos - a_pos).normalize()).unwrap_or(Dir3::Z);
                let filter = SpatialQueryFilter::default();
                
                if spatial_query.cast_ray(a_pos, dir, dist, true, &filter).is_none() {
                    // No obstacles between nodes, connect them
                    graph.connections[i].push(j);
                    graph.connections[j].push(i);
                }
            }
        }
    }
}

pub fn draw_pathfinding_graph_system(
    graph: Res<PathfindingGraph>,
    mut gizmos: Gizmos,
) {
    // Draw each node
    // for (i, pos) in graph.nodes.iter().enumerate() {
    //     // Draw the node as a small sphere
    //     gizmos.sphere(*pos, 0.2, Color::srgb(1.0, 0.0, 0.0));
        
    //     // Draw connections
    //     for &conn_idx in &graph.connections[i] {
    //         let other_pos = graph.nodes[conn_idx];
    //         gizmos.line(*pos, other_pos, Color::srgb(0.0, 1.0, 0.0));
    //     }
    // }
}
