use hyperQL::functions::geometric::distance::hyperbolic_distance;
use hyperQL::functions::geometric::operations::{k_nearest_neighbors, find_near_positions, hyperbolic_centroid};
use hyperQL::types::Position3D;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("HyperQL KNN Operations Demo");
    println!("===========================");
    
    // Create some test positions in hyperbolic space
    let positions = vec![
        Position3D { x: 0.1, y: 0.0, z: 0.0 },  // Close to origin
        Position3D { x: 0.0, y: 0.2, z: 0.0 },  // Moderate distance
        Position3D { x: 0.0, y: 0.0, z: 0.3 },  // Farther from origin
        Position3D { x: 0.15, y: 0.15, z: 0.0 }, // Diagonal close
        Position3D { x: 0.4, y: 0.4, z: 0.2 },  // More distant
    ];
    
    let reference = Position3D { x: 0.0, y: 0.0, z: 0.0 }; // Origin
    
    println!("\nTest Positions:");
    for (i, pos) in positions.iter().enumerate() {
        let dist = hyperbolic_distance(&reference, pos)?;
        println!("Position[{}]: ({:.2}, {:.2}, {:.2}) - Distance: {:.4}", 
                 i, pos.x, pos.y, pos.z, dist);
    }
    
    // Test k-nearest neighbors
    println!("\nK-Nearest Neighbors (k=3):");
    let knn_result = k_nearest_neighbors(&positions, &reference, 3)?;
    for (i, (index, distance)) in knn_result.iter().enumerate() {
        let pos = &positions[*index];
        println!("  {}. Position[{}]: ({:.2}, {:.2}, {:.2}) - Distance: {:.4}",
                 i + 1, index, pos.x, pos.y, pos.z, distance);
    }
    
    // Test finding positions within radius
    println!("\nPositions within radius 0.25:");
    let near_indices = find_near_positions(&positions, &reference, 0.25)?;
    for index in &near_indices {
        let pos = &positions[*index];
        let dist = hyperbolic_distance(&reference, pos)?;
        println!("  Position[{}]: ({:.2}, {:.2}, {:.2}) - Distance: {:.4}",
                 index, pos.x, pos.y, pos.z, dist);
    }
    
    // Test hyperbolic centroid
    println!("\nHyperbolic Centroid:");
    let centroid = hyperbolic_centroid(&positions)?;
    println!("  Centroid: ({:.4}, {:.4}, {:.4})", centroid.x, centroid.y, centroid.z);
    
    // Verify centroid is within Poincaré ball
    let centroid_norm = (centroid.x * centroid.x + centroid.y * centroid.y + centroid.z * centroid.z).sqrt();
    println!("  Centroid norm: {:.6} (< 1.0: {})", centroid_norm, centroid_norm < 1.0);
    
    println!("\nDemo completed successfully!");
    Ok(())
}
