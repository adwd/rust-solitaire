//! Rounded paper geometry; the card face and edge share the same silhouette.
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use klondike_bevy::{CARD_H, CARD_W};

fn outline() -> Vec<Vec2> {
    let radius = 0.09;
    let half = Vec2::new(CARD_W, CARD_H) * 0.5 - Vec2::splat(radius);
    [
        (Vec2::new(half.x, -half.y), -1.0),
        (Vec2::new(half.x, half.y), 0.0),
        (Vec2::new(-half.x, half.y), 1.0),
        (Vec2::new(-half.x, -half.y), 2.0),
    ]
    .into_iter()
    .flat_map(|(center, quarter)| {
        (0..=8).map(move |i| {
            let angle = (quarter + i as f32 / 8.0) * std::f32::consts::FRAC_PI_2;
            center + Vec2::new(angle.cos(), angle.sin()) * radius
        })
    })
    .collect()
}

pub fn face() -> Mesh {
    let outline = outline();
    let mut positions = vec![[0.0, 0.0, 0.0]];
    positions.extend(outline.iter().map(|p| [p.x, 0.0, p.y]));
    let uvs: Vec<_> = positions
        .iter()
        .map(|p| [p[0] / CARD_W + 0.5, p[2] / CARD_H + 0.5])
        .collect();
    let indices: Vec<_> = (0..outline.len() as u32)
        .flat_map(|i| [0, (i + 1) % outline.len() as u32 + 1, i + 1])
        .collect();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![[0.0, 1.0, 0.0]; positions.len()],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

pub fn body() -> Mesh {
    let outline = outline();
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    for (i, p) in outline.iter().enumerate() {
        let next = outline[(i + 1) % outline.len()];
        let delta = next - p;
        let normal = Vec3::new(delta.y, 0.0, -delta.x).normalize();
        let base = positions.len() as u32;
        positions.extend([
            [p.x, -0.014, p.y],
            [p.x, 0.014, p.y],
            [next.x, -0.014, next.y],
            [next.x, 0.014, next.y],
        ]);
        normals.extend([normal.to_array(); 4]);
        indices.extend([base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    }
    // Closing faces make the card cast a solid shadow and retain its thickness.
    for (height, normal, reverse) in [(0.014, 1.0, false), (-0.014, -1.0, true)] {
        let base = positions.len() as u32;
        positions.push([0.0, height, 0.0]);
        positions.extend(outline.iter().map(|p| [p.x, height, p.y]));
        normals.extend(vec![[0.0, normal, 0.0]; outline.len() + 1]);
        for i in 0..outline.len() as u32 {
            let a = base + i + 1;
            let b = base + (i + 1) % outline.len() as u32 + 1;
            indices.extend(if reverse { [base, a, b] } else { [base, b, a] });
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; positions.len()])
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices))
}
