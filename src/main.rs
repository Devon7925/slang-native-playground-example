use slang_playground_compiler::CompilationResult;
use slang_renderer::Renderer;
use wgpu::Features;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};
use slang_shader_macros::compile_shader;

#[cfg(target_family = "wasm")]
use std::panic;
#[cfg(debug_assertions)]
#[cfg(target_family = "wasm")]
extern crate console_error_panic_hook;

#[cfg(not(target_arch = "wasm32"))]
use slang_debug_app::DebugAppState;

struct RenderData {
    pub state: Renderer,
    pub window: Arc<Window>,
    pub surface: wgpu::Surface<'static>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

impl RenderData {
    async fn new(window: Arc<Window>, compilation: CompilationResult) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();

        let info = adapter.get_info();

        let info_logging = format!("Running on backend: {}\n", info.backend);
        #[cfg(not(target_arch = "wasm32"))]
        print!("{}", info_logging);
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&info_logging.into());

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                ..Default::default()
            })
            .await
            .unwrap();

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let state = Renderer::new(
                compilation,
                window.inner_size(),
                device.clone(),
                queue.clone(),
            ).await;

        configure_surface(&surface, &device, window.inner_size(), wgpu::TextureFormat::Rgba8Unorm);

        RenderData {
            state: state,
            window: window,
            surface: surface,
            device: device,
            queue: queue,
        }
    }
}

// Surface configuration is now handled externally if needed
fn configure_surface(surface: &wgpu::Surface, device: &wgpu::Device, size: PhysicalSize<u32>, surface_format: wgpu::TextureFormat) {
    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        view_formats: vec![surface_format],
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        width: size.width,
        height: size.height,
        desired_maximum_frame_latency: 2,
        present_mode: if cfg!(target_arch = "wasm32") {
            wgpu::PresentMode::Fifo
        } else {
            wgpu::PresentMode::Immediate
        },
    };
    surface
        .configure(device, &surface_config);
}

struct App {
    render_data: Option<RenderData>,
    #[cfg(target_arch = "wasm32")]
    state_receiver: Option<futures::channel::oneshot::Receiver<RenderData>>,
    #[cfg(not(target_arch = "wasm32"))]
    debug_app: Option<DebugAppState>,
    compilation: Option<CompilationResult>,
    surface_format: wgpu::TextureFormat,
}
impl App {
    fn new(compilation: CompilationResult) -> Self {
        Self {
            render_data: None,
            #[cfg(target_arch = "wasm32")]
            state_receiver: None,
            #[cfg(not(target_arch = "wasm32"))]
            debug_app: None,
            compilation: Some(compilation),
            surface_format: wgpu::TextureFormat::Rgba8Unorm,
        }
    }

    fn render_frame(&mut self) {
        let Some(render_data) = self.render_data.as_mut() else {
            return;
        };
        render_data.state.begin_frame();
        let surface_texture = render_data.surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface_format),
                ..Default::default()
            });
        let mut encoder = render_data
            .device
            .create_command_encoder(&Default::default());
        render_data.state.run_compute_passes(&mut encoder);
        render_data.state.run_draw_passes(&mut encoder, &texture_view);
        render_data.queue.submit([encoder.finish()]);
        render_data.state.handle_print_output();
        surface_texture.present();
    }

    #[cfg(target_arch = "wasm32")]
    fn ensure_state_is_loaded(&mut self) -> bool {
        if self.render_data.is_some() {
            return true;
        }

        if let Some(receiver) = self.state_receiver.as_mut() {
            if let Ok(Some(state)) = receiver.try_recv() {
                self.render_data = Some(state);
                self.state_receiver = None;
            }
        }
        self.render_data.is_some()
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let compilation = self.compilation.take().expect("Compilation result is missing");
        #[allow(unused_mut)]
        let mut builder = Window::default_attributes().with_title("Slang Native Playground");

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            let canvas = web_sys::window()
                .expect("error window")
                .document()
                .expect("error document")
                .get_element_by_id("canvas")
                .expect("could not find id canvas")
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .expect("error HtmlCanvasElement");
            builder = builder.with_decorations(false).with_canvas(Some(canvas));
        }

        let window = Arc::new(event_loop.create_window(builder).unwrap());
        let future = RenderData::new(window, compilation);
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.render_data = Some(pollster::block_on(future));
        }

        #[cfg(target_arch = "wasm32")]
        {
            let (sender, receiver) = futures::channel::oneshot::channel();
            self.state_receiver = Some(receiver);
            wasm_bindgen_futures::spawn_local(async move {
                let state = future.await;
                if sender.send(state).is_err() {
                    panic!("Failed to create and send renderer!");
                }
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        if cfg!(debug_assertions) {
            let debug_state = pollster::block_on(DebugAppState::new(
                event_loop,
                (1360, 768),
                self.render_data.as_ref().unwrap().state.uniform_components.clone(),
            ));
            self.debug_app = Some(debug_state);
            self.render_data.as_ref().unwrap().window.focus_window();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
                return;
            }
            _ => (),
        }
        #[cfg(target_arch = "wasm32")]
        if !self.ensure_state_is_loaded() {
            return; // Still loading, skip event
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(debug_app) = self.debug_app.as_mut()
            && debug_app.get_window_id() == _window_id
        {
            debug_app.handle_input(&event);
            return;
        }

        #[cfg(target_arch = "wasm32")]
        match event {
            WindowEvent::RedrawRequested => {
                self.render_frame();
            }
            _ => (),
        }

        let Some(render_data) = self.render_data.as_mut() else {
            return;
        };
        match event {
            WindowEvent::Resized(size) => {
                configure_surface(&render_data.surface, &render_data.device, size, self.surface_format);
            }
            _ => (),
        }

        render_data.state.process_event(&event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        #[cfg(target_arch = "wasm32")]
        if !self.ensure_state_is_loaded() {
            return; // Still loading, skip event
        }

        #[cfg(not(target_arch = "wasm32"))]
        self.render_frame();
        #[cfg(target_arch = "wasm32")]
        self.render_data.as_ref().unwrap().window.request_redraw();

        #[cfg(not(target_arch = "wasm32"))]
        // Only handle debug window if in debug mode
        if cfg!(debug_assertions) {
            self.debug_app.as_mut().unwrap().about_to_wait();
        }
    }
}

pub fn launch(compilation: CompilationResult) {
    #[cfg(debug_assertions)]
    #[cfg(target_family = "wasm")]
    panic::set_hook(Box::new(console_error_panic_hook::hook));

    // wgpu uses `log` for all of our logging, so we initialize a logger with the `env_logger` crate.
    //
    // To change the log level, set the `RUST_LOG` environment variable. See the `env_logger`
    // documentation for more information.
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    // When the current loop iteration finishes, immediately begin a new
    // iteration regardless of whether or not new events are available to
    // process. Preferred for applications that want to render as fast as
    // possible, like games.
    event_loop.set_control_flow(ControlFlow::Poll);

    // When the current loop iteration finishes, suspend the thread until
    // another event arrives. Helps keeping CPU utilization low if nothing
    // is happening, which is preferred if the application might be idling in
    // the background.
    // event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::new(compilation);
    event_loop.run_app(&mut app).unwrap();
}

#[cfg(target_family = "wasm")]
mod wasm_workaround {
    unsafe extern "C" {
        pub(super) fn __wasm_call_ctors();
    }
}

fn main() {
    // https://github.com/rustwasm/wasm-bindgen/issues/4446
    #[cfg(target_family = "wasm")]
    unsafe { wasm_workaround::__wasm_call_ctors()};

    let compilation: CompilationResult = compile_shader!("user.slang", ["shaders"]);
    launch(compilation);
}