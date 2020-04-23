extern crate glutin_window;
extern crate graphics;
extern crate image as im;
extern crate opengl_graphics;
extern crate piston;
extern crate piston_window;
extern crate sprite;
#[macro_use]
extern crate bitflags;

use crate::piston::EventLoop;
use crate::graphics::Transformed;
use piston::{Event};
use piston_window::PistonWindow as Window;
use opengl_graphics::{OpenGL};
use piston::event_loop::{EventSettings, Events};
use piston::input::{RenderArgs, RenderEvent, UpdateArgs, UpdateEvent};
use piston::window::WindowSettings;
use im::{ImageBuffer, Rgba};
use piston_window::{G2dTextureContext, TextureContext, G2dTexture, Texture, TextureSettings, image, clear, Filter};
use rand::Rng;
use std::path::Path;
use std::collections::HashMap;
use sprite::Sprite;
use std::rc::Rc;

const SolidBreakablePixel: Rgba<u8> = Rgba([255, 255, 255, 255]);
const EMPTY_PIXEL: Rgba<u8> = Rgba([0, 0, 0, 255]);

trait CollisionMap {
    fn get_pixel_safe(&self, x: u32, y: u32) -> Result<&im::Rgba<u8>, ()>;
    fn get_pixel_mut_safe(&mut self, x: u32, y: u32) -> Result<&mut im::Rgba<u8>, ()>;
}

impl CollisionMap for ImageBuffer<Rgba<u8>, Vec<u8>> {
    fn get_pixel_mut_safe(&mut self, x: u32, y: u32) -> Result<&mut im::Rgba<u8>, ()> {
        if x < self.width() && y < self.height() {
            Ok(self.get_pixel_mut(x, y))
        } else {
            Err(())
        }
    }
    fn get_pixel_safe(&self, x: u32, y: u32) -> Result<&im::Rgba<u8>, ()> {
        if x < self.width() && y < self.height() {
            Ok(self.get_pixel(x, y))
        } else {
            Err(())
        }
    }
}

trait CollisionMapPixel {
    fn is_solid(&self) -> bool;
    fn is_breakable(&self) -> bool;
}

impl CollisionMapPixel for Rgba<u8> {
    fn is_solid(&self) -> bool {
        self[0] != 0
    }
    fn is_breakable(&self) -> bool {
        self[2] != 0
    }
}

bitflags! {
    struct Actions: u32 {
        const WALK = 0x01;
        const DIG = 0x02;
    }
}

// pub struct Animation {
//     sprite_id: String,
//
// }

enum FacingDirection {
    Left,
    Right,
}

pub struct Lemming {
    x: u32,
    y: u32,
    direction: FacingDirection,
    actions: Actions,
}

impl Lemming {
    fn new(x: u32, y: u32, direction: FacingDirection) -> Lemming {
        Lemming {
            x,
            y,
            direction,
            actions: Actions::DIG | Actions::WALK,
        }
    }

    fn x_speed(&self) -> i32 {
        match self.direction {
            FacingDirection::Left => -1,
            FacingDirection::Right => 1,
        }
    }

    fn on_map(&self, environment: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> bool {
        self.x < environment.width() && self.y < environment.height()
    }

    fn on_ground(&self, environment: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> bool {
        if !self.on_map(environment) {
            return false;
        }
        if environment.get_pixel_safe(self.x, self.y + 1).unwrap_or(&EMPTY_PIXEL).is_solid() {
            return true;
        }
        return false;
    }

    fn fall(&mut self, environment: &ImageBuffer<Rgba<u8>, Vec<u8>>) {
        if !self.on_ground(environment) {
            self.y += 1;
        }
    }

    fn walk(&mut self, environment: &ImageBuffer<Rgba<u8>, Vec<u8>>) {
        if self.on_map(environment) {
            if self.on_ground(environment) {
                if environment.get_pixel_safe((self.x as i32 + self.x_speed()) as u32, self.y).unwrap_or(&EMPTY_PIXEL).is_solid() {
                    for y in 0..6 {
                        if !environment.get_pixel_safe((self.x as i32 + self.x_speed()) as u32, self.y - y).unwrap_or(&EMPTY_PIXEL).is_solid() {
                            self.y -= y;
                            break;
                        }
                    }
                }
                if environment.get_pixel_safe((self.x as i32 + self.x_speed()) as u32, self.y).unwrap_or(&EMPTY_PIXEL).is_solid() {
                    self.direction = match self.direction {
                        FacingDirection::Left => FacingDirection::Right,
                        FacingDirection::Right => FacingDirection::Left,
                    }
                }
                self.x = (self.x as i32 + self.x_speed()) as u32;
            }
        }
    }

    fn dig(&mut self, environment: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
        if self.on_map(environment) {
            if !self.on_ground(environment) {
                //self.actions.remove(Actions::DIG);
                return;
            }
            for x in 0..6 {
                if let Ok(groud_pixel) = environment.get_pixel_mut_safe(self.x - 3 + x, self.y + 1) {
                    if groud_pixel.is_breakable() {
                        *groud_pixel = EMPTY_PIXEL;
                    }
                }
            }
            self.y += 1;
        }
    }

    fn update(&mut self, environment: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
        if self.actions.contains(Actions::WALK) {
            self.walk(environment);
        }
        self.fall(environment);
        if self.actions.contains(Actions::DIG) {
            self.dig(environment);
        }
    }
}

pub struct App {
    canvas: ImageBuffer<Rgba<u8>, Vec<u8>>, // Solid layer
    texture: G2dTexture,
    window: Window,
    texture_context: G2dTextureContext,
    lemmings: Vec<Lemming>,
    sprites: HashMap<String, Sprite<G2dTexture>>,
}

impl App {
    fn render(&mut self, args: &RenderArgs, event: &Event) {

        const background_color: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
        const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];

        self.texture.update(&mut self.texture_context, &self.canvas).unwrap();

        let texture = &mut self.texture;
        texture.update(&mut self.texture_context, &self.canvas).unwrap();

        let texture_context = &mut self.texture_context;
        let lemmings = &self.lemmings;
        let sprites = &self.sprites;

        let window_scale = [
            self.window.window.ctx.window().get_inner_size().unwrap().width as f64 / self.canvas.width() as f64,
            self.window.window.ctx.window().get_inner_size().unwrap().height as f64 / self.canvas.height() as f64];

        let window_size = [self.window.window.ctx.window().get_inner_size().unwrap().width, self.window.window.ctx.window().get_inner_size().unwrap().height];
        let canvas_size = [self.canvas.width() as f64, self.canvas.height() as f64];

        self.window.draw_2d(event, |c, gl, device| {
            // Clear the screen.
            clear(background_color, gl);

            let window_transform = c.transform.scale(window_scale[0], window_scale[1]);//.trans(1f64, canvas_size[1] - window_size[1]);//.scale(window_scale[0], window_scale[1]);

            texture_context.encoder.flush(device);
            image(texture, window_transform, gl);

            for lemming in lemmings {
                let transform = window_transform
                    .trans(lemming.x.into(), lemming.y.into());

                sprites["lemming"].draw(transform, gl);
                //image(&sprites["lemming"], transform, gl);
            }
        });
    }

    fn _step_environment_gravity(&mut self) {
        for x in 0..self.canvas.width() {
            for y in (0..self.canvas.height()).rev() {
                let pixel = self.canvas.get_pixel(x, y);
                let empty = pixel[0] == 0;
                if empty && y > 0 {
                    let above_pixel = self.canvas.get_pixel(x, y - 1).clone();

                    if above_pixel[0] != 0 {
                        {
                            let pixel = self.canvas.get_pixel_mut(x, y);
                            pixel[0] = above_pixel[0];
                            pixel[1] = above_pixel[1];
                            pixel[2] = above_pixel[2];
                            pixel[3] = above_pixel[3];
                        }
                        {
                            let pixel = self.canvas.get_pixel_mut(x, y - 1);
                            pixel[0] = 0;
                            pixel[1] = 0;
                            pixel[2] = 0;
                            pixel[3] = 0;
                        }
                    }
                }
            }
        }
    }

    fn update(&mut self, args: &UpdateArgs) {
        // Rotate 2 radians per second.
        // self.self.window.window.ctx.window().get_inner_size().unwrap().width
        //self.canvas.put_pixel(rand::random::<u32>() % self.canvas.width(), rand::random::<u32>() % self.canvas.height(), im::Rgba([255, 255, 255, 255]));

        //self._step_environment_gravity();
        let environment = &mut self.canvas;
        for lemming in &mut self.lemmings {
            lemming.update(environment);
        }
    }
}

fn main() {
    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;

    let window_width: u32 = 800;
    let window_height: u32 = 600;

    // Create an Glutin window.
    let mut window: Window = WindowSettings::new("Lemrus", [window_width, window_height])
        .graphics_api(opengl)
        .exit_on_esc(true)
        .build()
        .unwrap();

    //let storage = vec![0; 4 * window_width as usize * window_height as usize];

    let texture_settings = TextureSettings::new().filter(Filter::Nearest);

    let canvas: ImageBuffer<Rgba<u8>, Vec<u8>> = im::open("level.png").unwrap().into_rgba();//ImageBuffer::from_raw(window_width, window_height, storage).unwrap();
    let mut texture_context = window.create_texture_context();
    let texture: G2dTexture = Texture::from_image(
            &mut texture_context,
            &canvas,
            &texture_settings,
        ).unwrap();

    // Create a new game and run it.
    let mut app = App {
        canvas: canvas,
        texture: texture,
        window: window,
        texture_context: texture_context,
        lemmings: Vec::new(),
        sprites: HashMap::new(),
    };

    let lemming_image = im::open("lem.png").unwrap().into_rgba();
    let lemming_texture = Rc::from(Texture::from_image(
            &mut app.texture_context,
            &lemming_image,
            &texture_settings,
        ).unwrap());

    let mut lemming_sprite = Sprite::from_texture(lemming_texture);
    lemming_sprite.set_anchor(0.5, 0.9);
    app.sprites.insert("lemming".to_string(), lemming_sprite);

    app.lemmings.push(Lemming::new(100, 50, FacingDirection::Right));

    let mut events = Events::new(EventSettings::new().ups(20u64));
    while let Some(e) = events.next(&mut app.window) {
        if let Some(args) = e.render_args() {
            app.render(&args, &e);
        }

        if let Some(args) = e.update_args() {
            app.update(&args);
        }
    }
}
