#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

extern crate ggez;

use ggez::*;
use ggez::input::keyboard::{KeyCode, KeyMods};
use ggez::audio::SoundSource;
use ggez::graphics::Color;

use std::env;
use std::path;

use rand::Rng;

//constants related to the game
const WINDOW_WIDTH: f32 = 800.0;
const WINDOW_HEIGHT: f32 = 600.0;

//the framerate we want
const DESIRED_FPS: u32 = 60;

//the number of pillars which will be active during the game
const NUM_PILLARS: u32 = 5;
//the gap/opening between two blocks of a pillar
const PILLAR_GAP: f32 = 220.0;
//distance between two pillars
const PILLAR_DISTANCE: f32 = 300.0;
//width of a pillar block
const PILLAR_WIDTH: f32 = 80.0;
//the amount by which the speed of the pillars increases every frame
const PILLAR_ACCELERATION: f32 = -0.0004;

//the gravity in the game 
const GRAVITY: f32 = 1.0;

//the amount the player jumps 
const JUMP_AMOUNT: f32 = -10.0;

//utility function which checks if two rects are colliding
fn collide_rect(rect1: &graphics::Rect, rect2: &graphics::Rect) -> bool {
    let rect1right = rect1.x + rect1.w;
    let rect1bottom = rect1.y + rect1.h;
    let rect2right = rect2.x + rect2.w;
    let rect2bottom = rect2.y + rect2.h;
    (rect1.x < rect2right) && (rect1right > rect2.x) && (rect1.y < rect2bottom) && (rect1bottom > rect2.y)
}

//structure which stores all the sounds of the game 
struct Sounds {
    jump: audio::Source,
    switch: audio::Source,
    clink: audio::Source,
    crash: audio::Source,
}

//structure which stores all the text elements of the game
struct Texts {
    intro: graphics::Text,
    intro_pos: mint::Point2<f32>,
    intro_offscreen: bool,
    score: graphics::Text,
    restart: graphics::Text,
}

//the pillar struct which contains the properties such as color and dimensions
#[derive(Clone)]
struct Pillar {
    color: Color,
    top: graphics::Rect,
    bottom: graphics::Rect,
}

//methods for updating and drawing the pillars
impl Pillar {
    fn update(&mut self, colors_list: &Vec<Color>, last_x: f32, speed: f32) -> GameResult {
        //make the pillars move to the left
        self.top.x += speed;
        self.bottom.x += speed;
        //check if pillars cross the screen
        if self.top.x + PILLAR_WIDTH <= 0.0 {
            //wrap them back to the right
            self.top.x = last_x + PILLAR_DISTANCE;
            self.bottom.x = last_x + PILLAR_DISTANCE;
            //give them a new color
            self.color = colors_list[rand::thread_rng().gen_range(0, colors_list.len())];
            //give them a new height 
            let height = rand::thread_rng().gen_range(0.0, WINDOW_HEIGHT - PILLAR_GAP);
            //adjust the dimensions of the pillars to match the new height
            self.top.h = height;
            self.bottom.y = height + PILLAR_GAP;
            self.bottom.h = WINDOW_HEIGHT - (height + PILLAR_GAP);
        }
        //return
        Ok(())
    }
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        //create a mesh for the top half
        let top_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            self.top,
            self.color,
        )?;
        //create a mesh for the bottom half
        let bottom_mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            self.bottom,
            self.color,
        )?;
        //draw both the meshes 
        graphics::draw(ctx, &top_mesh, graphics::DrawParam::default())?;
        graphics::draw(ctx, &bottom_mesh, graphics::DrawParam::default())?;
        //return 
        Ok(())
    }
}

//the struct which stores properties of the player 
struct Player {
    color_index: usize,
    color: Color, 
    body: graphics::Rect,
    velocity: mint::Point2<f32>,
}

//methods for updating and drawing the player
impl Player {
    fn update(&mut self, pillars: &mut Vec<Pillar>, game_over: &mut bool, sounds: &mut Sounds, score: &mut u32) -> GameResult {
        //check if player is inside/below/above any of the pillars
        for pillar in pillars {
            if self.body.x < pillar.top.x + pillar.top.w && self.body.x + self.body.w > pillar.top.x {
                if pillar.color != self.color {
                    //DEBUG-REMOVE
                    //println!("full collision not checked - WRONG COLOR");
                    let _ = sounds.crash.play();
                    *game_over = true;
                }
                else if collide_rect(&self.body, &pillar.top) || collide_rect(&self.body, &pillar.bottom) {
                    //DEBUG-REMOVE
                    //println!("CORRECT COLOR BUT U COLLIDE MAN");
                    let _ = sounds.crash.play();
                    *game_over = true;
                }
                //check if middle parts (horizontal) align
                if self.body.x as i32 + (self.body.w / 2.0) as i32 == pillar.top.x as i32 + (pillar.top.w / 2.0) as i32 {
                    //DEBUG-REMOVE
                    //println!("clink!");
                    //increment score
                    *score += 1;
                    let _ = sounds.clink.play();
                }
            }
        }
        //add gravity to player's velocity and accelerate the player
        self.velocity.y += GRAVITY;
        self.body.y += self.velocity.y;
        //if the player is about ot go off screen, negate the added gravity
        if self.body.y + self.body.h >= WINDOW_HEIGHT {
            self.velocity.y = -GRAVITY;
        }
        Ok(())
    }
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        //create a drawable mesh for the player 
        let player_mesh = graphics::Mesh::new_rectangle(
            ctx, 
            graphics::DrawMode::fill(),
            self.body,
            self.color,
        )?;
        //draw the player
        graphics::draw(ctx, &player_mesh, graphics::DrawParam::default())?;
        Ok(())
    }
}

//the main state of the game
struct MainState {
    colors: Vec<Color>,
    player: Player,
    pillars: Vec<Pillar>,
    pillar_speed: f32,
    game_over: bool,
    sounds: Sounds,
    score: u32,
    texts: Texts,
}

impl MainState {
    fn new(ctx: &mut Context) -> Self {
        //random number generator 
        let mut rng = rand::thread_rng();
        //color scheme used in the game
        let colors_list: Vec<Color> = vec![
            [0.13725491, 0.23921569, 0.3019608, 1.0].into(),
            [0.99607843, 0.49803922, 0.1764706, 1.0].into(),
            [0.9882353, 0.7921569, 0.27450982, 1.0].into(),
            [0.6313726, 0.75686276, 0.5058824, 1.0].into(),
            [0.38039216, 0.60784316, 0.5411765, 1.0].into(),
        ];
        //create sounds
        let mut clink_sound = audio::Source::new(ctx, "/clink.wav").unwrap();
        let mut jump_sound = audio::Source::new(ctx, "/perc.wav").unwrap();
        jump_sound.set_volume(0.6);
        let mut crash_sound = audio::Source::new(ctx, "/crash.wav").unwrap();
        let mut switch_sound = audio::Source::new(ctx, "/switch.wav").unwrap();
        //create score text 
        let font = graphics::Font::new(ctx, "/Raleway-Black.ttf").unwrap();
        let score_text = graphics::Text::new(("0", font, 40.0));
        let intro_text = graphics::Text::new(("Space to jump\nCtrl to switch colors", font, 30.0));
        let restart_text = graphics::Text::new(("Oof! Press Enter to restart", font, 30.0));
        //create a vector of pillars
        let mut pv = Vec::new();
        for i in 0..NUM_PILLARS {
            let height: f32 = rng.gen_range(0.0, WINDOW_HEIGHT - PILLAR_GAP);
            pv.push(Pillar {
                //the color of the pillar
                color: colors_list[rng.gen_range(0, colors_list.len())],
                //the top block/half of the pillar
                top: graphics::Rect::new(
                    //x
                    WINDOW_WIDTH + PILLAR_DISTANCE * i as f32,
                    //y
                    0.0,
                    //width
                    PILLAR_WIDTH,
                    //height
                    height,
                ),
                bottom: graphics::Rect::new(
                    //x
                    WINDOW_WIDTH + PILLAR_DISTANCE * i as f32,
                    //y
                    height + PILLAR_GAP,
                    //width
                    PILLAR_WIDTH,
                    //height
                    WINDOW_HEIGHT - (height + PILLAR_GAP),
                ),
            });
        }
        //stores the color index for the player 
        let color_index: usize = rng.gen_range(0, colors_list.len());
        //return a MainState
        MainState {
            colors: colors_list.clone(),
            //the player object
            player: Player {
                color_index: color_index,
                color: colors_list[color_index],
                body: graphics::Rect::new(
                    //x
                    WINDOW_WIDTH / 2.0,
                    //y
                    WINDOW_HEIGHT / 2.0,
                    //width
                    50.0,
                    //height
                    50.0,
                ),
                //the velocity of the player 
                velocity: mint::Point2 {
                    x: 0.0,
                    y: 0.0,
                },
            },
            //the vector of pillars
            pillars: pv,
            pillar_speed: -1.0,
            game_over: false,
            sounds: Sounds {
                jump: jump_sound,
                switch: switch_sound,
                clink: clink_sound,
                crash: crash_sound,
            },
            //player's score
            score: 0,
            texts: Texts {
                intro: intro_text,
                intro_pos: mint::Point2 {
                    x: WINDOW_WIDTH / 3.0,
                    y: WINDOW_HEIGHT / 2.2,
                },
                intro_offscreen: false,
                score: score_text,
                restart: restart_text,
            }
        }
    }
    //resets the game 
    fn reset(&mut self, ctx: &mut Context) {
        *self = MainState::new(ctx);
    }
}

//implementing event handler so that the event loop can run on MainState 
impl event::EventHandler for MainState {
    //update function
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        //make sure the game runs at 60fps
        while timer::check_update_time(ctx, DESIRED_FPS) {
            //only update if the game is not over
            if !self.game_over {
                //increase speed of the pillars
                self.pillar_speed += PILLAR_ACCELERATION;
                //update the pillars
                let mut i: usize = 0;
                let pillars = self.pillars.clone();
                for pillar in &mut self.pillars {
                    //calculate the position of the pillar which is in front of the current pillar 
                    let last_x: f32 = if i == 0 {
                        pillars[pillars.len() - 1].top.x
                    }
                    else {
                        pillars[i - 1].top.x
                    };
                    pillar.update(&self.colors, last_x, self.pillar_speed)?;
                    i+=1;
                }
                //update the player
                self.player.update(&mut self.pillars, &mut self.game_over, &mut self.sounds, &mut self.score)?;
                //update the score text to the score
                self.texts.score.fragments_mut()[0].text = self.score.to_string();
                //update the position of the intro text 
                if !self.texts.intro_offscreen {
                    self.texts.intro_pos.x += self.pillar_speed;
                    if self.texts.intro_pos.x + self.texts.intro.width(ctx) as f32 <= 0.0 {
                        self.texts.intro_offscreen = true;
                    }
                }
            }
        }
        Ok(())
    }
    //draw function
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        //clear the screen
        graphics::clear(ctx, graphics::WHITE);
        //draw the intro text if it is not off screen
        if !self.texts.intro_offscreen {
            graphics::draw(ctx, &self.texts.intro, graphics::DrawParam::default()
                .dest(self.texts.intro_pos)
                .color(graphics::BLACK)
            )?;
        }
        //draw the pillars
        for pillar in &mut self.pillars {
            pillar.draw(ctx)?;
        }
        //draw the player 
        self.player.draw(ctx)?;
        //draw the text
        graphics::draw(ctx, &self.texts.score, graphics::DrawParam::default().dest(mint::Point2 {
            x: 20.0,
            y: 20.0,
        }).color(graphics::BLACK))?;
        //if the player lost display game over text 
        if self.game_over {
            graphics::draw(ctx, &self.texts.restart, graphics::DrawParam::default().dest(mint::Point2 {
                x: WINDOW_WIDTH/3.0,
                y: WINDOW_HEIGHT/2.2,
            }).color(graphics::BLACK))?;
        }
        //display the stuff that was drawn
        graphics::present(ctx)?;
        Ok(())
    }
    //when a key is pressed
    fn key_down_event(&mut self, ctx: &mut Context, keycode: KeyCode, _keymods: KeyMods, _repeat: bool) {
        if !self.game_over {
            //if the user pressed space
            if keycode == KeyCode::Space {
                //make the player jump by adding negative velocity
                self.player.velocity.y = JUMP_AMOUNT;
                //play jump sound
                let _ = self.sounds.jump.play();
            }
            else if keycode == KeyCode::LControl || keycode == KeyCode::RControl {
                //increment the color index by 1 and wrap if it exceeds the length
                self.player.color_index = (self.player.color_index + 1) % self.colors.len();
                //assign new color to player 
                self.player.color = self.colors[self.player.color_index];
                //play switch sound
                let _ = self.sounds.switch.play();
            }
        }
        else if keycode == KeyCode::Return {
            self.reset(ctx);
        }
    }
    //when a key is released
    fn key_up_event(&mut self, ctx: &mut Context, keycode: KeyCode, _keymods: KeyMods) {
    }
}

fn main() -> GameResult {
    //loading resource dir 
    let resource_dir = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = path::PathBuf::from(manifest_dir);
        path.push("resources");
        path
    }
    else {
        path::PathBuf::from("resources")
    };

    //getting context and event loop
    let (ctx, event_loop) = &mut ContextBuilder::new("ball", "udbhav")
        .add_resource_path(resource_dir)
        .window_setup(ggez::conf::WindowSetup::default()
            .title("flappy cube color game thing - udbhav")
            .icon("/icon.ico")
        )
        .build().unwrap();

    //building the main state of the game     
    let state = &mut MainState::new(ctx);

    //play music
    let mut music = audio::Source::new(ctx, "/flappy cube.ogg")?;
    music.set_repeat(true);
    let _ = music.play();

    //running the main state 
    event::run(ctx, event_loop, state)
}