use std::sync::Arc;

use thiserror::Error;
use wgpu::{Backends, CreateSurfaceError, RequestDeviceError, SurfaceError, TextureFormat};
use winit::{
    dpi::PhysicalSize,
    error::ExternalError,
    window::{Fullscreen, Window},
};

#[derive(Debug, Error)]
pub enum GfxError {
    #[error("io error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("winit error: {0}")]
    WinitError(#[from] ExternalError),
    #[error("surface error: {0}")]
    SurfaceError(#[from] SurfaceError),
    #[error("create surface error: {0}")]
    CreateSurfaceError(#[from] CreateSurfaceError),
    #[error("pngs can only be capture from buffers")]
    CannotCapturePngFromSurface,
    #[error("request adapter error")]
    RequestAdapterError,
    #[cfg(feature = "capture")]
    #[error("encoding error: {0}")]
    EncodingError(#[from] png::EncodingError),
    #[error("request device error: {0}")]
    RequestDeviceError(#[from] RequestDeviceError),
}

#[derive(Default)]
pub struct GfxConfig {
    pub present_mode: wgpu::PresentMode,
}

pub struct Gfx {
    internal: GfxInternal,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
}

impl Gfx {
    pub fn new_from_window(window: Window, config: GfxConfig) -> Result<Self, GfxError> {
        pollster::block_on(async {
            let instance = Self::create_instance();
            let window = Arc::new(window);
            let surface = instance.create_surface(window.clone())?;
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptionsBase {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: Some(&surface),
                })
                .await
                .ok_or(GfxError::RequestAdapterError)?;
            let (device, queue) = Self::request_device(&adapter).await?;
            let size = window.inner_size();
            let internal = GfxInternal::Surface { surface, window };

            Self::setup(adapter, device, queue, internal, size, config).await
        })
    }

    pub fn new_from_buffer(size: PhysicalSize<u32>, config: GfxConfig) -> Result<Self, GfxError> {
        pollster::block_on(async {
            let instance = Self::create_instance();
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptionsBase {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: None,
                })
                .await
                .ok_or(GfxError::RequestAdapterError)?;
            let (device, queue) = Self::request_device(&adapter).await?;
            let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
            let bytes_per_row = 4 * size.width + (align - (4 * size.width) % align) % align;
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: (bytes_per_row * size.height) as u64,
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
            let internal = GfxInternal::Buffer {
                bytes_per_row,
                buffer,
                extent,
                texture: Box::leak(Box::new(texture)),
            };
            Self::setup(adapter, device, queue, internal, size, config).await
        })
    }

    fn create_instance() -> wgpu::Instance {
        wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: Backends::VULKAN | Backends::METAL | Backends::DX12 | Backends::GL,
            // This can be swapped out for a faster and more modern compiler which requires extra
            // dlls to be shipped https://docs.rs/wgpu/latest/wgpu/enum.Dx12Compiler.html
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
            ..Default::default()
        })
    }

    async fn request_device(
        adapter: &wgpu::Adapter,
    ) -> Result<(wgpu::Device, wgpu::Queue), GfxError> {
        Ok(adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                        | wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER,
                    required_limits: wgpu::Limits {
                        max_texture_dimension_1d: 8192,
                        max_texture_dimension_2d: 8192,
                        ..wgpu::Limits::downlevel_defaults()
                    },
                },
                None,
            )
            .await?)
    }

    async fn setup(
        adapter: wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
        internal: GfxInternal,
        size: PhysicalSize<u32>,
        config: GfxConfig,
    ) -> Result<Self, GfxError> {
        let capabilities = internal.get_capabilities(&adapter);
        log::debug!("Found texture formats: {:?}", capabilities.formats);
        let texture_format = capabilities
            .formats
            .into_iter()
            .next()
            .unwrap_or(TextureFormat::Rgba8UnormSrgb);
        let alpha_mode = capabilities
            .alpha_modes
            .into_iter()
            .next()
            .unwrap_or_default();
        log::info!("Chosen texture format: {texture_format:?} and alpha mode: {alpha_mode:?}");
        if !texture_format.is_srgb() {
            log::warn!("Texture format is not srgb");
        }

        let sample_flags = adapter.get_texture_format_features(texture_format).flags;
        log::debug!("Sample flags {sample_flags:#?}");

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: texture_format,
            width: size.width,
            height: size.height,
            present_mode: config.present_mode,
            alpha_mode,
            view_formats: vec![texture_format],
            desired_maximum_frame_latency: 2,
        };
        internal.configure(&device, &config);

        Ok(Self {
            device,
            queue,
            internal,
            config,
        })
    }

    pub fn get_current_texture(&self) -> Result<RenderableTexture, GfxError> {
        self.internal.get_current_texture()
    }

    pub fn present(&self, renderable_texture: RenderableTexture) {
        self.internal
            .present(&self.device, &self.queue, renderable_texture)
    }

    pub fn window_resize(&mut self, size: &PhysicalSize<u32>) {
        self.config.width = size.width;
        self.config.height = size.height;
        self.internal.configure(&self.device, &self.config);
    }

    pub fn set_cursor_grab(&self, grab: bool) -> Result<(), GfxError> {
        self.internal.set_cursor_grab(grab)
    }

    pub fn set_cursor_visible(&self, visible: bool) {
        self.internal.set_cursor_visible(visible)
    }

    pub fn toggle_fullscreen(&self) {
        self.internal.toggle_fullscreen()
    }

    #[cfg(feature = "capture")]
    pub fn create_png(&self, output: &std::path::Path) -> Result<(), GfxError> {
        self.internal.create_png(&self.device, output)
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.config.width as f32 / self.config.height as f32
    }
}

enum GfxInternal {
    Surface {
        window: Arc<Window>,
        surface: wgpu::Surface<'static>,
    },
    Buffer {
        bytes_per_row: u32,
        buffer: wgpu::Buffer,
        extent: wgpu::Extent3d,
        texture: &'static wgpu::Texture,
    },
}

fn fullscreen_mode(fullscreen: bool) -> Option<Fullscreen> {
    if fullscreen {
        Some(Fullscreen::Borderless(None))
    } else {
        None
    }
}

impl GfxInternal {
    fn get_capabilities(&self, adapter: &wgpu::Adapter) -> wgpu::SurfaceCapabilities {
        if let Self::Surface { surface, .. } = self {
            surface.get_capabilities(adapter)
        } else {
            wgpu::SurfaceCapabilities::default()
        }
    }

    fn configure(&self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        if let Self::Surface { surface, .. } = self {
            surface.configure(device, config);
        }
    }

    fn get_current_texture(&self) -> Result<RenderableTexture, GfxError> {
        match self {
            Self::Surface { surface, .. } => {
                Ok(RenderableTexture::Surface(surface.get_current_texture()?))
            }
            Self::Buffer { texture, .. } => Ok(RenderableTexture::Texture(texture)),
        }
    }

    fn present(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        renderable_texture: RenderableTexture,
    ) {
        match (self, renderable_texture) {
            (Self::Surface { .. }, RenderableTexture::Surface(surface)) => surface.present(),
            (
                Self::Buffer {
                    bytes_per_row,
                    buffer,
                    extent,
                    ..
                },
                RenderableTexture::Texture(texture),
            ) => {
                log::info!("Copying texture to buffer");
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                encoder.copy_texture_to_buffer(
                    texture.as_image_copy(),
                    wgpu::ImageCopyBuffer {
                        buffer,
                        layout: wgpu::ImageDataLayout {
                            offset: 0,
                            bytes_per_row: Some(*bytes_per_row),
                            rows_per_image: None,
                        },
                    },
                    *extent,
                );
                queue.submit(Some(encoder.finish()));
            }
            _ => panic!("invalid internal gfx present"),
        }
    }

    fn set_cursor_grab(&self, grab: bool) -> Result<(), GfxError> {
        let Self::Surface { window, .. } = self else {
            return Ok(());
        };
        if grab {
            // Try locked then try confined
            if let Err(err) = window.set_cursor_grab(winit::window::CursorGrabMode::Locked) {
                log::error!("Failed to set cursor confined: {err}");
                window.set_cursor_grab(winit::window::CursorGrabMode::Confined)?;
            }
        } else {
            window.set_cursor_grab(winit::window::CursorGrabMode::None)?;
        }
        Ok(())
    }

    fn set_cursor_visible(&self, visible: bool) {
        if let Self::Surface { window, .. } = self {
            window.set_cursor_visible(visible);
        }
    }

    fn toggle_fullscreen(&self) {
        let Self::Surface { window, .. } = self else {
            return;
        };
        window.set_fullscreen(fullscreen_mode(window.fullscreen().is_none()))
    }

    #[cfg(feature = "capture")]
    fn create_png(&self, device: &wgpu::Device, output: &std::path::Path) -> Result<(), GfxError> {
        use std::{fs::File, io::Write};

        let Self::Buffer {
            bytes_per_row,
            buffer,
            extent,
            ..
        } = self
        else {
            return Err(GfxError::CannotCapturePngFromSurface);
        };
        let mut encoder = png::Encoder::new(File::create(output)?, extent.width, extent.height);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_color(png::ColorType::Rgba);
        let mut writer = encoder
            .write_header()?
            .into_stream_writer_with_size(extent.width as usize * 4)?;
        let buffer_slice = buffer.slice(..);

        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::Maintain::Wait);
        for chunk in buffer_slice
            .get_mapped_range()
            .chunks(*bytes_per_row as usize)
        {
            writer.write_all(&chunk[..extent.width as usize * 4])?;
        }
        writer.finish()?;
        Ok(())
    }
}

/// Wrapper that allows a surface or a buffer to be used
pub enum RenderableTexture {
    Surface(wgpu::SurfaceTexture),
    Texture(&'static wgpu::Texture),
}

impl RenderableTexture {
    pub fn texture(&self) -> &wgpu::Texture {
        match self {
            Self::Surface(surface) => &surface.texture,
            Self::Texture(texture) => texture,
        }
    }

    pub fn present(self) {
        match self {
            Self::Surface(surface) => surface.present(),
            Self::Texture(_) => (),
        }
    }
}
