use std::sync::Arc;

use winit::window::Window;

pub struct GfxSurface {
    pub window: Arc<Window>,
    pub surface: wgpu::Surface<'static>,
}
