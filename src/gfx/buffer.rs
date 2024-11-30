use std::sync::Arc;

use wgpu::Device;
use winit::dpi::PhysicalSize;

pub struct GfxBuffer {
    pub bytes_per_row: u32,
    pub buffer: wgpu::Buffer,
    pub extent: wgpu::Extent3d,
    pub texture: Arc<wgpu::Texture>,
}

impl GfxBuffer {
    pub fn new(device: &Device, size: PhysicalSize<u32>) -> Self {
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let bytes_per_row = 4 * size.width + (align - (4 * size.width) % align) % align;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(bytes_per_row * size.height),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let extent = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            label: None,
            view_formats: &[],
        });

        Self {
            bytes_per_row,
            buffer,
            extent,
            texture: Arc::new(texture),
        }
    }
}
