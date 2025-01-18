pub mod buffer;
pub mod surface;

use std::{num::NonZeroU32, sync::Arc};

use buffer::GfxBuffer;
use surface::GfxSurface;
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

pub struct GfxConfig {
    pub present_mode: wgpu::PresentMode,
    pub required_features: wgpu::Features,
    pub multisample_count: NonZeroU32,
}

impl Default for GfxConfig {
    fn default() -> Self {
        Self {
            present_mode: wgpu::PresentMode::default(),
            required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                | wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER,
            multisample_count: NonZeroU32::MIN,
        }
    }
}

pub struct Gfx {
    pub backing: GfxBacking,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub multisample_count: NonZeroU32,
    pub multisample_view: Option<wgpu::TextureView>,
}

impl Gfx {
    pub fn new_from_window(window: Window, config: &GfxConfig) -> Result<Self, GfxError> {
        pollster::block_on(async {
            let instance = Self::create_instance();
            let window = Arc::new(window);
            let surface = instance.create_surface(Arc::clone(&window))?;
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptionsBase {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: Some(&surface),
                })
                .await
                .ok_or(GfxError::RequestAdapterError)?;
            let (device, queue) = Self::request_device(&adapter, config).await?;
            let size = window.inner_size();
            let internal = GfxBacking::Surface(GfxSurface { window, surface });

            Ok(Self::setup(&adapter, device, queue, internal, size, config))
        })
    }

    pub fn new_from_buffer(size: PhysicalSize<u32>, config: &GfxConfig) -> Result<Self, GfxError> {
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
            let (device, queue) = Self::request_device(&adapter, config).await?;
            let internal = GfxBacking::Buffer(GfxBuffer::new(&device, size));
            Ok(Self::setup(&adapter, device, queue, internal, size, config))
        })
    }

    fn create_instance() -> wgpu::Instance {
        wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: Backends::VULKAN | Backends::METAL | Backends::DX12 | Backends::GL,
            ..Default::default()
        })
    }

    async fn request_device(
        adapter: &wgpu::Adapter,
        config: &GfxConfig,
    ) -> Result<(wgpu::Device, wgpu::Queue), GfxError> {
        Ok(adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: config.required_features,
                    required_limits: wgpu::Limits {
                        max_texture_dimension_1d: 8192,
                        max_texture_dimension_2d: 8192,
                        ..wgpu::Limits::downlevel_defaults()
                    },
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await?)
    }

    fn setup(
        adapter: &wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
        internal: GfxBacking,
        size: PhysicalSize<u32>,
        config: &GfxConfig,
    ) -> Self {
        let capabilities = match &internal {
            GfxBacking::Surface(GfxSurface { surface, .. }) => surface.get_capabilities(adapter),
            GfxBacking::Buffer(_) => wgpu::SurfaceCapabilities::default(),
        };
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

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: texture_format,
            width: size.width,
            height: size.height,
            present_mode: config.present_mode,
            alpha_mode,
            view_formats: vec![texture_format],
            desired_maximum_frame_latency: 2,
        };
        if let GfxBacking::Surface(GfxSurface { surface, .. }) = &internal {
            surface.configure(&device, &surface_config);
        };

        let multisample_view =
            Self::create_multisample_view(&device, config.multisample_count, &surface_config);

        Self {
            backing: internal,
            device,
            queue,
            config: surface_config,
            multisample_count: config.multisample_count,
            multisample_view,
        }
    }

    pub fn get_current_texture(&self) -> Result<RenderableTexture, GfxError> {
        match &self.backing {
            GfxBacking::Surface(GfxSurface { surface, .. }) => {
                Ok(RenderableTexture::Surface(surface.get_current_texture()?))
            }
            GfxBacking::Buffer(buffer) => {
                Ok(RenderableTexture::Texture(Arc::clone(&buffer.texture)))
            }
        }
    }

    pub fn color_attachments<'a>(
        &'a self,
        load: wgpu::LoadOp<wgpu::Color>,
        final_view: &'a wgpu::TextureView,
    ) -> Result<wgpu::RenderPassColorAttachment<'a>, GfxError> {
        if let Some(m) = &self.multisample_view {
            Ok(wgpu::RenderPassColorAttachment {
                view: m,
                resolve_target: Some(final_view),
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Discard,
                },
            })
        } else {
            Ok(wgpu::RenderPassColorAttachment {
                view: final_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })
        }
    }

    pub fn present(&self) -> Result<(), GfxError> {
        match &self.backing {
            GfxBacking::Surface(GfxSurface { surface, .. }) => {
                surface.get_current_texture()?.present();
                Ok(())
            }
            GfxBacking::Buffer(buffer) => {
                let mut encoder = self
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                encoder.copy_texture_to_buffer(
                    buffer.texture.as_image_copy(),
                    wgpu::TexelCopyBufferInfo {
                        buffer: &buffer.buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(buffer.bytes_per_row),
                            rows_per_image: None,
                        },
                    },
                    buffer.extent,
                );
                self.queue.submit(Some(encoder.finish()));
                Ok(())
            }
        }
    }

    pub fn window_resize(&mut self, size: &PhysicalSize<u32>) {
        let old_size = (self.config.width, self.config.height);
        self.config.width = size.width;
        self.config.height = size.height;
        let new_size = (self.config.width, self.config.height);
        log::trace!("window resize {old_size:?} -> {new_size:?}");
        if let GfxBacking::Surface(GfxSurface { surface, .. }) = &self.backing {
            surface.configure(&self.device, &self.config);
        };
        self.multisample_view =
            Self::create_multisample_view(&self.device, self.multisample_count, &self.config);
    }

    fn create_multisample_view(
        device: &wgpu::Device,
        multisample_count: NonZeroU32,
        config: &wgpu::SurfaceConfiguration,
    ) -> Option<wgpu::TextureView> {
        if multisample_count.get() > 1 {
            let view = device
                .create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: config.width,
                        height: config.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: multisample_count.get(),
                    dimension: wgpu::TextureDimension::D2,
                    format: config.format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default());
            Some(view)
        } else {
            None
        }
    }

    pub fn set_cursor_grab(&self, grab: bool) -> Result<(), GfxError> {
        let GfxBacking::Surface(GfxSurface { window, .. }) = &self.backing else {
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

    pub fn set_cursor_visible(&self, visible: bool) {
        if let GfxBacking::Surface(GfxSurface { window, .. }) = &self.backing {
            window.set_cursor_visible(visible);
        };
    }

    pub fn toggle_fullscreen(&self) {
        if let GfxBacking::Surface(GfxSurface { window, .. }) = &self.backing {
            window.set_fullscreen(fullscreen_mode(window.fullscreen().is_none()));
        }
    }

    #[cfg(feature = "capture")]
    pub fn create_png(&self, output: &std::path::Path) -> Result<(), GfxError> {
        use std::{fs::File, io::Write as _};

        let GfxBacking::Buffer(GfxBuffer {
            bytes_per_row,
            buffer,
            extent,
            ..
        }) = &self.backing
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
        self.device.poll(wgpu::Maintain::Wait);
        for chunk in buffer_slice
            .get_mapped_range()
            .chunks(*bytes_per_row as usize)
        {
            writer.write_all(&chunk[..extent.width as usize * 4])?;
        }
        writer.finish()?;
        Ok(())
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.config.width as f32 / self.config.height as f32
    }

    pub fn window(&self) -> Option<&Window> {
        let GfxBacking::Surface(GfxSurface { window, .. }) = &self.backing else {
            return None;
        };
        Some(window)
    }
}

pub enum GfxBacking {
    Surface(GfxSurface),
    Buffer(GfxBuffer),
}

fn fullscreen_mode(fullscreen: bool) -> Option<Fullscreen> {
    if fullscreen {
        Some(Fullscreen::Borderless(None))
    } else {
        None
    }
}

/// Wrapper that allows a surface or a buffer to be used
pub enum RenderableTexture {
    Surface(wgpu::SurfaceTexture),
    Texture(Arc<wgpu::Texture>),
}

impl RenderableTexture {
    pub fn texture(&self) -> &wgpu::Texture {
        match self {
            Self::Surface(surface) => &surface.texture,
            Self::Texture(texture) => texture.as_ref(),
        }
    }

    pub fn present(self) {
        match self {
            Self::Surface(surface) => surface.present(),
            Self::Texture(_) => (),
        }
    }
}
