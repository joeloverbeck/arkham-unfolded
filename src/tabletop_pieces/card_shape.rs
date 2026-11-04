//! Shared physical-card silhouette. UVs retain the entire original image scale.
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

pub(crate) const CORNER_FRACTION: f32 = 0.045;

fn perimeter(width: f32, length: f32, radius: f32) -> Vec<Vec2> {
    let half = Vec2::new(width, length) * 0.5;
    let mut points = Vec::with_capacity(36);
    for (corner, center) in [
        Vec2::new(half.x - radius, half.y - radius),
        Vec2::new(-half.x + radius, half.y - radius),
        Vec2::new(-half.x + radius, -half.y + radius),
        Vec2::new(half.x - radius, -half.y + radius),
    ]
    .into_iter()
    .enumerate()
    {
        for step in 0..=8 {
            let angle = (corner as f32 + step as f32 / 8.) * std::f32::consts::FRAC_PI_2;
            points.push(center + Vec2::new(angle.cos(), angle.sin()) * radius);
        }
    }
    points
}

pub(crate) fn face(width: f32, length: f32) -> Mesh {
    let outline = perimeter(width, length, width.min(length) * CORNER_FRACTION);
    let mut positions = vec![[0., 0., 0.]];
    let mut uv = vec![[0.5, 0.5]];
    let mut indices = Vec::new();
    for point in &outline {
        positions.push([point.x, 0., point.y]);
        uv.push([point.x / width + 0.5, point.y / length + 0.5]);
    }
    for i in 0..outline.len() as u32 {
        indices.extend([0, (i + 1) % outline.len() as u32 + 1, i + 1]);
    }
    let count = positions.len();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; count])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
    .with_inserted_indices(Indices::U32(indices))
}

pub(crate) fn body(width: f32, height: f32, length: f32) -> Mesh {
    let outline = perimeter(width, length, width.min(length) * CORNER_FRACTION);
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uv = Vec::new();
    let mut indices = Vec::new();
    for (i, from) in outline.iter().enumerate() {
        let to = outline[(i + 1) % outline.len()];
        let edge = to - *from;
        let normal = Vec3::new(edge.y, 0., -edge.x)
            .normalize_or_zero()
            .to_array();
        let start = positions.len() as u32;
        positions.extend([
            [from.x, -height / 2., from.y],
            [to.x, -height / 2., to.y],
            [to.x, height / 2., to.y],
            [from.x, height / 2., from.y],
        ]);
        normals.extend([normal; 4]);
        uv.extend([[0., 0.], [1., 0.], [1., 1.], [0., 1.]]);
        indices.extend([start, start + 2, start + 1, start, start + 3, start + 2]);
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
    .with_inserted_indices(Indices::U32(indices));
    mesh.merge(&face(width, length).translated_by(Vec3::Y * height / 2.))
        .expect("Matching card attributes");
    mesh.merge(
        &face(width, length)
            .rotated_by(Quat::from_rotation_x(std::f32::consts::PI))
            .translated_by(-Vec3::Y * height / 2.),
    )
    .expect("Matching card attributes");
    mesh
}

pub(crate) fn outline(width: f32, length: f32) -> Mesh {
    let radius = width.min(length) * CORNER_FRACTION;
    let inner = perimeter(width + 0.06, length + 0.06, radius + 0.03);
    let outer = perimeter(width + 0.11, length + 0.11, radius + 0.055);
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for (a, b) in inner.iter().zip(&outer) {
        positions.extend([[a.x, 0., a.y], [b.x, 0., b.y]]);
    }
    for i in 0..inner.len() as u32 {
        let a = i * 2;
        let b = ((i + 1) % inner.len() as u32) * 2;
        indices.extend([a, b, a + 1, a + 1, b, b + 1]);
    }
    let count = positions.len();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; count])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; count])
    .with_inserted_indices(Indices::U32(indices))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues;

    #[test]
    fn card_silhouettes_keep_image_scale_and_face_outward() {
        for mesh in [face(2., 2.8), body(2., 0.06, 2.8), outline(2., 2.8)] {
            let Some(VertexAttributeValues::Float32x3(positions)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!("card positions")
            };
            let Some(VertexAttributeValues::Float32x3(normals)) =
                mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
            else {
                panic!("card normals")
            };
            let indices: Vec<_> = mesh.indices().unwrap().iter().collect();
            for triangle in indices.chunks_exact(3) {
                let [a, b, c] = [triangle[0], triangle[1], triangle[2]];
                let cross = (Vec3::from(positions[b]) - Vec3::from(positions[a]))
                    .cross(Vec3::from(positions[c]) - Vec3::from(positions[a]));
                assert!(
                    cross.dot(Vec3::from(normals[a])) > 0.,
                    "back-face culling must retain the visible card surface"
                );
            }
        }
        let mesh = face(2., 2.8);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            unreachable!()
        };
        let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            unreachable!()
        };
        for (point, uv) in positions.iter().zip(uvs) {
            assert!(
                !(point[0].abs() > 0.99 && point[2].abs() > 1.39),
                "the rectangular corner is cut away"
            );
            assert!((uv[0] - (point[0] / 2. + 0.5)).abs() < 0.00001);
            assert!((uv[1] - (point[2] / 2.8 + 0.5)).abs() < 0.00001);
        }
    }
}
