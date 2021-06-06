extern crate sdl2;

use std::rc::Rc;
use std::cell::RefCell;

use sdl2::video::{Window, WindowContext};
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::rect::{Point, Rect};
use sdl2::pixels::Color;
use sdl2::Sdl;

pub struct Display {
    sdl_context: Rc<RefCell<Sdl>>,
    // window: Window,
    canvas: Canvas<Window>,
    // texture_creator: TextureCreator,
    texture: Texture,
}

impl Display {
    pub fn new() -> Result<Display, String> { // sdl_context: Rc<RefCell<Sdl>>) {
        let sdl_context = Rc::new(RefCell::new(sdl2::init()?));

        let video_subsystem = sdl_context.borrow().video()?;

        let window = video_subsystem.window("peek6502", 800, 600)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;

        let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

        let texture_creator : TextureCreator<_> = canvas.texture_creator();

        let mut texture = texture_creator.create_texture_target(None, 64, 128).map_err(|e| e.to_string())?;


        Ok(Display {
            sdl_context: sdl_context,
            // window: window,
            canvas: canvas,
            // texture_creator: texture_creator,
            texture: texture,
        })
    }

    pub fn base_char_loc(&self, index: i32) -> (i32, i32) {
        ((index % 8) * 8, (index / 8) * 8)
    }

    // TODO : This looping has got to be slow. Find a way to do blocks of memory.
    pub fn set_char_texture(&mut self, char_index: i32, char_data: [u8; 8]) {
        let (base_x, base_y) = self.base_char_loc(char_index);
        let mut row_data = 0_u8;

        self.canvas.with_texture_canvas(&mut self.texture, |texture_canvas| {
            for j in 0..8 {
                row_data = char_data[j];
                for i in 0..8 {
                    let mask = 0x80_u8 >> i;
                    let is_pixel_on = row_data & mask == mask;
                    if is_pixel_on {
                        texture_canvas.set_draw_color(Color::RGB(255, 0, 0));
                    } else {
                        texture_canvas.set_draw_color(Color::RGB(128, 128, 128));
                    }
                    texture_canvas.draw_point(Point::new(base_x + (i as i32), base_y + (j as i32)))
                        .expect("could not draw point");
                }
            }
        });
    }

    pub fn draw_charset(&mut self) -> Result<(), String> {
        for event in self.sdl_context.borrow().event_pump()?.poll_iter() {
            match event {
                // Event::KeyDown { keycode: Some(Keycode::Escape), .. } |
                // Event::Quit { .. } => break 'mainloop,
                _ => {}
            }
        }

        self.canvas.copy(&self.texture,
            None,
            Rect::new(0, 0, 64, 128))?;

            // canvas.clear();
        self.canvas.present();

        Ok(())
    }

    pub fn draw_background(&self) {


    }
}
