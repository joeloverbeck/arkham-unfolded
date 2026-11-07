//! Geometry is owned by unfolded-tabletop; game layout and visibility stay here.
pub(crate) use unfolded_tabletop::card::{CORNER_FRACTION, body, face, outline};
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues;
    use bevy::prelude::*;

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
