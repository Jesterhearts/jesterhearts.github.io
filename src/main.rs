use std::{
    cell::RefCell,
    num::NonZeroU32,
    rc::Rc,
};

use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{
        Block,
        Paragraph,
    },
};
use ratatui_wgpu::{
    Builder,
    Dimensions,
    Font,
    WgpuBackend,
    shaders::CrtPostProcessor,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement,
    HtmlTextAreaElement,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
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
    text_input: Rc<RefCell<Option<HtmlTextAreaElement>>>,
}

pub fn main() -> anyhow::Result<()> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).unwrap();

    let event_loop = EventLoop::builder().build()?;

    let app = App {
        window: Rc::default(),
        backend: Rc::default(),
        text_input: Rc::default(),
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
        let input = self.text_input.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let (text_input, height, width) = web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| {
                    let dst = doc.get_element_by_id("glcanvas")?;

                    let input = doc
                        .create_element("textarea")
                        .ok()?
                        .dyn_into::<HtmlTextAreaElement>()
                        .ok()?;
                    input.set_value(
                        "This is a simple text editor using ratatui-wgpu.

It even supports emojis! 😊🦀🐁
On Windows, you can use WIN+. to insert and test this out!",
                    );

                    let style = input.style();
                    style.set_property("opacity", "0").ok()?;
                    style.set_property("width", "100%").ok()?;
                    style.set_property("height", "1px").ok()?;
                    style.set_property("position", "absolute").ok()?;
                    style.set_property("top", "0").ok()?;
                    style.set_property("left", "0").ok()?;
                    style.set_property("z-index", "-1").ok()?;
                    dst.append_child(&input).ok()?;

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
                        input,
                        NonZeroU32::new(bounds.height() as u32)?,
                        NonZeroU32::new(bounds.width() as u32)?,
                    ))
                })
                .expect("Failed to attach canvas");

            window
                .borrow_mut()
                .as_mut()
                .unwrap()
                .set_prevent_default(false);
            let canvas = window.borrow().as_ref().unwrap().canvas().unwrap();

            *backend.borrow_mut() = Some(
                Terminal::new(
                    Builder::from_font(
                        Font::new(include_bytes!("fonts/NotoSansMono.ttf")).unwrap(),
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

            text_input.focus().unwrap();
            *input.borrow_mut() = Some(text_input);
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
            WindowEvent::Focused(true) => {
                self.text_input.borrow().as_ref().unwrap().focus().unwrap();
                self.window
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .set_prevent_default(false);
            }
            WindowEvent::Resized(size) => {
                terminal.backend_mut().resize(size.width, size.height);
                Self::redraw(self.text_input.borrow().as_ref().unwrap(), terminal);
            }
            WindowEvent::RedrawRequested => {
                Self::redraw(self.text_input.borrow().as_ref().unwrap(), terminal);
            }
            _ => {}
        }

        self.window.borrow().as_ref().unwrap().request_redraw();
    }
}

impl App {
    fn redraw(text_input: &HtmlTextAreaElement, terminal: &mut Terminal<CrtBackend>) {
        let current = text_input.value();

        let current_start = text_input.selection_start().ok().flatten();
        let current_end = text_input.selection_end().ok().flatten();

        let text_len = current.len();
        let start = current_start.unwrap_or_default();
        let end = current_end.unwrap_or(text_len as u32);

        let start_highlight = start.min(end);
        let end_highlight = start.max(end);

        let end_highlight = if start_highlight == end_highlight {
            start_highlight + 1
        } else {
            end_highlight
        };

        let mut cur_char = 0;
        let mut lines = vec![];
        let mut highlight = false;

        for line in current.split('\n') {
            let mut spans = vec![];
            let mut cur_span = String::new();
            for c in line.graphemes(true).chain(std::iter::once(" ")) {
                if cur_char >= start_highlight && cur_char < end_highlight {
                    if !highlight {
                        highlight = true;
                        spans.push(Span::from(cur_span));
                        cur_span = String::new();
                    }
                } else if highlight {
                    highlight = false;
                    spans.push(Span::from(cur_span).style(Style::default().reversed()));
                    cur_span = String::new();
                }

                cur_span.push_str(c);
                cur_char += c.width().max(1) as u32;
            }

            if highlight {
                spans.push(Span::from(cur_span).style(Style::default().reversed()));
            } else {
                spans.push(Span::from(cur_span));
            }

            lines.push(Line::from_iter(spans));
        }

        terminal
            .draw(|f| {
                f.render_widget(
                    Paragraph::new(lines).block(Block::bordered().border_set(border::ROUNDED)),
                    f.area(),
                )
            })
            .unwrap();
    }
}
