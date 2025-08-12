use slang_playground_compiler::CompilationResult;
use slang_renderer::Renderer;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
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

struct App {
    state: Option<Renderer>,
    #[cfg(target_arch = "wasm32")]
    state_receiver: Option<futures::channel::oneshot::Receiver<Renderer>>,
    #[cfg(not(target_arch = "wasm32"))]
    debug_app: Option<DebugAppState>,
    compilation: Option<CompilationResult>,
}
impl App {
    fn new(compilation: CompilationResult) -> Self {
        Self {
            state: None,
            #[cfg(target_arch = "wasm32")]
            state_receiver: None,
            #[cfg(not(target_arch = "wasm32"))]
            debug_app: None,
            compilation: Some(compilation),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn ensure_state_is_loaded(&mut self) -> bool {
        if self.state.is_some() {
            return true;
        }

        if let Some(receiver) = self.state_receiver.as_mut() {
            if let Ok(Some(state)) = receiver.try_recv() {
                self.state = Some(state);
                self.state_receiver = None;
            }
        }
        self.state.is_some()
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
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
        // Create window object
        let window = Arc::new(event_loop.create_window(builder).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            let state = pollster::block_on(Renderer::new(
                window.clone(),
                self.compilation.take().unwrap(),
            ));
            self.state = Some(state);
        }

        #[cfg(target_arch = "wasm32")]
        {
            let (sender, receiver) = futures::channel::oneshot::channel();
            self.state_receiver = Some(receiver);
            let compilation = self.compilation.take().unwrap();
            wasm_bindgen_futures::spawn_local(async move {
                let state = Renderer::new(window.clone(), compilation).await;
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
                self.state.as_ref().unwrap().uniform_components.clone(),
            ));
            self.debug_app = Some(debug_state);
            self.state.as_ref().unwrap().window.focus_window();
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

        let state = self.state.as_mut().unwrap();
        state.process_event(&event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        #[cfg(target_arch = "wasm32")]
        if !self.ensure_state_is_loaded() {
            return; // Still loading, skip event
        }

        let state = self.state.as_mut().unwrap();
        #[cfg(not(target_arch = "wasm32"))]
        state.render();
        #[cfg(target_arch = "wasm32")]
        state.window.request_redraw();

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