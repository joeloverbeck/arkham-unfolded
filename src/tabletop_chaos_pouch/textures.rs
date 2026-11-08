//! Honor the delivered PNG textures' trilinear minification sampler. Bevy's
//! glTF PNG decoder supplies only the base level; unfiltered fabric normals
//! visibly alias at the tabletop's viewing distance. Source GLB bytes stay intact.
use bevy::{prelude::*, render::render_resource::TextureFormat};

pub(super) fn prepare(
    mut events: MessageReader<AssetEvent<Image>>,
    server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    for event in events.read() {
        let (AssetEvent::Added { id } | AssetEvent::Modified { id }) = *event else {
            continue;
        };
        if !server
            .get_path(id)
            .is_some_and(|path| path.path() == std::path::Path::new(super::PATH))
        {
            continue;
        }
        // Concurrent labeled-subasset requests can replace a previously decoded
        // image. Prepare those replacements too, before extraction sees them.
        // The source event already schedules its upload; do not emit another
        // Modified event and trigger a redundant upload on the following frame.
        if let Some(image) = images.get_mut_untracked(id) {
            mipmaps(image);
        }
    }
}

fn mipmaps(image: &mut Image) {
    if super::texture_ready(image) {
        return;
    }
    let (channels, srgb) = match image.texture_descriptor.format {
        TextureFormat::Rgba8UnormSrgb => (4, true),
        TextureFormat::Rgba8Unorm => (4, false),
        TextureFormat::Rg8Unorm => (2, false),
        TextureFormat::R8Unorm => (1, false),
        _ => return,
    };
    let mut width = image.width() as usize;
    let mut height = image.height() as usize;
    let Some(data) = image.data.as_mut() else {
        return;
    };
    if data.len() != width * height * channels {
        return;
    }
    let linear: [f32; 256] = std::array::from_fn(|i| Srgba::gamma_function(i as f32 / 255.0));
    let mut offset = 0;
    while width > 1 || height > 1 {
        let next = data.len();
        let (w, h) = ((width / 2).max(1), (height / 2).max(1));
        for y in 0..h {
            for x in 0..w {
                for channel in 0..channels {
                    let mut sum = 0.0;
                    for dy in 0..2 {
                        for dx in 0..2 {
                            let value = data[offset
                                + (((y * 2 + dy).min(height - 1) * width
                                    + (x * 2 + dx).min(width - 1))
                                    * channels)
                                + channel];
                            sum += if srgb && channel < 3 {
                                linear[value as usize]
                            } else {
                                value as f32 / 255.0
                            };
                        }
                    }
                    let average = sum / 4.0;
                    let value = if srgb && channel < 3 {
                        Srgba::gamma_function_inverse(average)
                    } else {
                        average
                    };
                    data.push((value * 255.0).round().clamp(0.0, 255.0) as u8);
                }
            }
        }
        offset = next;
        width = w;
        height = h;
        image.texture_descriptor.mip_level_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mipmaps_preserve_base_bytes_and_average_color_in_linear_space() {
        use bevy::{
            asset::RenderAssetUsages,
            render::render_resource::{Extent3d, TextureDimension},
        };
        let bytes = [
            0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255,
        ];
        let mut image = Image::new(
            Extent3d {
                width: 2,
                height: 2,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            bytes.to_vec(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        mipmaps(&mut image);
        assert_eq!(image.texture_descriptor.mip_level_count, 2);
        let data = image.data.as_ref().unwrap();
        assert_eq!(&data[..16], &bytes);
        assert_eq!(&data[16..], &[188, 188, 188, 255]);
        mipmaps(&mut image);
        assert_eq!(image.data.as_ref().unwrap().len(), 20);
    }
}
