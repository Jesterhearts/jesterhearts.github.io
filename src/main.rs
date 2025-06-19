use std::{
    cell::RefCell,
    num::NonZeroU32,
    rc::Rc,
};

use ratatui::{
    prelude::*,
    widgets::Block,
};
use ratatui_wgpu::{
    Builder,
    Dimensions,
    Font,
    WgpuBackend,
    shaders::CrtPostProcessor,
};
use tui_textarea::{
    Input,
    Key,
    TextArea,
};
use web_sys::HtmlCanvasElement;
use winit::{
    application::ApplicationHandler,
    event::{
        ElementState,
        WindowEvent,
    },
    event_loop::EventLoop,
    platform::web::*,
    window::{
        Window,
        WindowAttributes,
    },
};

type CrtBackend = WgpuBackend<'static, 'static, CrtPostProcessor>;

struct App {
    window: Rc<RefCell<Option<Window>>>,
    backend: Rc<RefCell<Option<Terminal<CrtBackend>>>>,
    ed: TextArea<'static>,
    ctrl_down: bool,
    alt_down: bool,
    shift_down: bool,
}

fn main() -> anyhow::Result<()> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).unwrap();

    let event_loop = EventLoop::builder().build()?;
    let mut ed = TextArea::new(vec![
        "🐭🦀🧀 This is a simple text editor using Ratatui and Wgpu. 🧀🦀🐭".to_string(),
        String::default(),
        "IME is currently disabled in this example because supporting it requires substantial additional work with hidden text areas.".to_string(),

    ]);
    ed.set_block(Block::bordered());

    let app = App {
        window: Rc::default(),
        backend: Rc::default(),
        ed,
        alt_down: false,
        ctrl_down: false,
        shift_down: false,
    };
    event_loop.spawn_app(app);

    Ok(())
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window = Rc::new(RefCell::new(Some(
            event_loop
                .create_window(WindowAttributes::default().with_title("Ratatui Wgpu Text Editor"))
                .unwrap(),
        )));

        let window = self.window.clone();
        let backend = self.backend.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let (height, width) = web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| {
                    let dst = doc.get_element_by_id("glcanvas")?;

                    let canvas: HtmlCanvasElement = window.borrow().as_ref()?.canvas()?;
                    let style = canvas.style();
                    style.set_property("display", "block").ok()?;
                    style.set_property("width", "100%").ok()?;
                    style.set_property("height", "100%").ok()?;
                    style.set_property("position", "absolute").ok()?;
                    style.set_property("top", "0").ok()?;
                    style.set_property("left", "0").ok()?;
                    style.set_property("z-index", "1").ok()?;

                    dst.append_with_node_1(&web_sys::Element::from(canvas.clone()))
                        .ok()?;

                    let bounds = canvas.get_bounding_client_rect();
                    Some((
                        NonZeroU32::new(bounds.height() as u32)?,
                        NonZeroU32::new(bounds.width() as u32)?,
                    ))
                })
                .expect("Failed to attach canvas");

            let canvas = window.borrow().as_ref().unwrap().canvas().unwrap();

            *backend.borrow_mut() = Some(
                Terminal::new(
                    Builder::from_font(
                        Font::new(include_bytes!("fonts/CaskaydiaMonoNerdFont-Regular.ttf"))
                            .unwrap(),
                    )
                    .with_fonts(vec![
                        Font::new(include_bytes!("fonts/NotoColorEmoji-Regular.ttf")).unwrap(),
                    ])
                    .with_width_and_height(Dimensions { width, height })
                    .build_with_target(wgpu::SurfaceTarget::Canvas(canvas))
                    .await
                    .unwrap(),
                )
                .unwrap(),
            );
        });
    }

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let mut terminal = self.backend.borrow_mut();
        let Some(terminal) = terminal.as_mut() else {
            return;
        };

        match event {
            WindowEvent::Resized(size) => {
                terminal.backend_mut().resize(size.width, size.height);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let key = match event.logical_key {
                    winit::keyboard::Key::Named(named_key) => match named_key {
                        winit::keyboard::NamedKey::Backspace => Key::Backspace,
                        winit::keyboard::NamedKey::Enter => Key::Enter,
                        winit::keyboard::NamedKey::Escape => Key::Esc,
                        winit::keyboard::NamedKey::Tab => Key::Tab,
                        winit::keyboard::NamedKey::ArrowDown => Key::Down,
                        winit::keyboard::NamedKey::ArrowLeft => Key::Left,
                        winit::keyboard::NamedKey::ArrowRight => Key::Right,
                        winit::keyboard::NamedKey::ArrowUp => Key::Up,
                        winit::keyboard::NamedKey::Control => {
                            self.ctrl_down = event.state == ElementState::Pressed;
                            return;
                        }
                        winit::keyboard::NamedKey::Alt => {
                            self.alt_down = event.state == ElementState::Pressed;
                            return;
                        }
                        winit::keyboard::NamedKey::Shift => {
                            self.shift_down = event.state == ElementState::Pressed;
                            return;
                        }
                        winit::keyboard::NamedKey::End => Key::End,
                        winit::keyboard::NamedKey::Home => Key::Home,
                        winit::keyboard::NamedKey::PageDown => Key::PageDown,
                        winit::keyboard::NamedKey::PageUp => Key::PageUp,
                        winit::keyboard::NamedKey::Copy => Key::Copy,
                        winit::keyboard::NamedKey::Cut => Key::Cut,
                        winit::keyboard::NamedKey::Delete => Key::Delete,
                        winit::keyboard::NamedKey::Paste => Key::Paste,
                        winit::keyboard::NamedKey::Space => Key::Char(' '),
                        winit::keyboard::NamedKey::F1 => Key::F(1),
                        winit::keyboard::NamedKey::F2 => Key::F(2),
                        winit::keyboard::NamedKey::F3 => Key::F(3),
                        winit::keyboard::NamedKey::F4 => Key::F(4),
                        winit::keyboard::NamedKey::F5 => Key::F(5),
                        winit::keyboard::NamedKey::F6 => Key::F(6),
                        winit::keyboard::NamedKey::F7 => Key::F(7),
                        winit::keyboard::NamedKey::F8 => Key::F(8),
                        winit::keyboard::NamedKey::F9 => Key::F(9),
                        winit::keyboard::NamedKey::F10 => Key::F(10),
                        winit::keyboard::NamedKey::F11 => Key::F(11),
                        winit::keyboard::NamedKey::F12 => Key::F(12),
                        winit::keyboard::NamedKey::F13 => Key::F(13),
                        winit::keyboard::NamedKey::F14 => Key::F(14),
                        winit::keyboard::NamedKey::F15 => Key::F(15),
                        winit::keyboard::NamedKey::F16 => Key::F(16),
                        winit::keyboard::NamedKey::F17 => Key::F(17),
                        winit::keyboard::NamedKey::F18 => Key::F(18),
                        winit::keyboard::NamedKey::F19 => Key::F(19),
                        winit::keyboard::NamedKey::F20 => Key::F(20),
                        winit::keyboard::NamedKey::F21 => Key::F(21),
                        winit::keyboard::NamedKey::F22 => Key::F(22),
                        winit::keyboard::NamedKey::F23 => Key::F(23),
                        winit::keyboard::NamedKey::F24 => Key::F(24),
                        winit::keyboard::NamedKey::F25 => Key::F(25),
                        winit::keyboard::NamedKey::F26 => Key::F(26),
                        winit::keyboard::NamedKey::F27 => Key::F(27),
                        winit::keyboard::NamedKey::F28 => Key::F(28),
                        winit::keyboard::NamedKey::F29 => Key::F(29),
                        winit::keyboard::NamedKey::F30 => Key::F(30),
                        winit::keyboard::NamedKey::F31 => Key::F(31),
                        winit::keyboard::NamedKey::F32 => Key::F(32),
                        winit::keyboard::NamedKey::F33 => Key::F(33),
                        winit::keyboard::NamedKey::F34 => Key::F(34),
                        winit::keyboard::NamedKey::F35 => Key::F(35),
                        _ => return,
                    },
                    winit::keyboard::Key::Character(c) => {
                        Key::Char(c.chars().next().unwrap_or_default())
                    }
                    _ => return,
                };

                if event.state == ElementState::Pressed {
                    self.ed.input(Input {
                        key,
                        ctrl: self.ctrl_down,
                        alt: self.alt_down,
                        shift: self.shift_down,
                    });
                }
            }
            WindowEvent::RedrawRequested => {
                terminal
                    .draw(|f| f.render_widget(&self.ed, f.area()))
                    .unwrap();
            }
            _ => {}
        }

        self.window.borrow().as_ref().unwrap().request_redraw();
    }
}
