use c2::{prelude::*, Circle, Poly};
use opencv::{core::*, imgproc::*, types::*};
use rand::Rng;

use crate::graphics::*;

#[derive(Debug)]
pub enum Player {
    Left,
    Right,
}

pub struct Game {
    size: Size,
    ball: Ball,
    score: Score,
    reset: bool,
    single_player: bool,
    graphics: Graphics,
}

impl Game {
    pub fn new(size: Size, single_player: bool) -> Game {
        Game {
            ball: Ball::new(size),
            score: Score::new(),
            reset: false,
            size: size,
            single_player: single_player,
            graphics: Graphics::init(size),
        }
    }
    pub fn update(&mut self, shapes: &VectorOfRotatedRect) -> opencv::Result<()> {
        if self.reset {
            self.reset = false;
        }

        self.ball.translate();
        self.ball.wall_collision(self.size, self.single_player);
        self.ball.shape_collision(shapes)?;

        if self.ball.x < 0 {
            self.score.add_right();
            self.reset = true;
        }
        if !self.single_player {
            if self.ball.x + self.ball.radius > self.size.width {
                self.score.add_left();
                self.reset = true;
            }
        }
        if self.reset {
            self.ball.reset(self.size);
        }

        Ok(())
    }

    pub fn draw(&self, img: &mut Mat) -> opencv::Result<()> {
        // reset solid background
        img.set_to(&self.graphics.bg_color, &no_array()?)?;

        // draw ball
        circle(
            img,
            self.ball.get_center(),
            self.ball.radius,
            self.graphics.obj_color,
            -1,
            LINE_8,
            0,
        )?;
        // draw score
        if !self.single_player {
            put_text(
                img,
                &self.score.left.to_string(),
                self.graphics.score_pos_left,
                self.graphics.font,
                2.0,
                self.graphics.score_color,
                2,
                LINE_8,
                false,
            )?;
        }
        put_text(
            img,
            &self.score.right.to_string(),
            self.graphics.score_pos_right,
            self.graphics.font,
            2.0,
            self.graphics.score_color,
            2,
            LINE_8,
            false,
        )?;

        Ok(())
    }
}

struct Ball {
    x: i32,
    y: i32,
    vel_x: i32,
    vel_y: i32,
    radius: i32,
    starting_side: Player,
}

impl Ball {
    fn new(screen_size: Size) -> Ball {
        let mut rng = rand::thread_rng();
        Ball {
            x: screen_size.width / 2,
            y: screen_size.height / 2,
            vel_x: rng.gen_range(20..30),
            vel_y: rng.gen_range(20..30),
            radius: 10,
            starting_side: Player::Right,
        }
    }
    pub fn translate(&mut self) {
        self.x += self.vel_x;
        self.y += self.vel_y;
    }
    pub fn reset(&mut self, screen: Size) {
        let mut rng = rand::thread_rng();
        self.x = screen.width / 2;
        self.y = screen.height / 2;
        self.vel_x = rng.gen_range(15..25);
        self.vel_y = rng.gen_range(15..25);
        match self.starting_side {
            Player::Right => {
                self.vel_x *= -1;
                self.starting_side = Player::Left;
            }
            Player::Left => {
                self.starting_side = Player::Right;
            }
        }
        if rand::random() {
            self.vel_y *= 1;
        }
    }
    pub fn wall_collision(&mut self, screen: Size, single_player: bool) {
        if self.y - self.radius <= 0 {
            self.vel_y *= -1;
            self.y += 1;
        }
        if self.y + self.radius >= screen.height {
            self.vel_y *= -1;
            self.y -= 1;
        }
        if single_player {
            if self.x + self.radius >= screen.width {
                self.vel_x *= -1;
                self.x -= 1;
            }
        }
    }
    pub fn shape_collision(&mut self, shapes: &VectorOfRotatedRect) -> opencv::Result<()> {
        let circle = Circle::new([self.x as f32, self.y as f32], self.radius as f32);
        for index in 0..shapes.len() {
            let shape = shapes.get(index)?;
            let mut vertices: [Point2f; 4] = [
                Point2f::new(0.0, 0.0),
                Point2f::new(0.0, 0.0),
                Point2f::new(0.0, 0.0),
                Point2f::new(0.0, 0.0),
            ];
            shape.points(&mut vertices)?;
            let poly = Poly::from_slice(&[
                [vertices[0].x, vertices[0].y],
                [vertices[1].x, vertices[1].y],
                [vertices[2].x, vertices[2].y],
                [vertices[3].x, vertices[3].y],
            ]);
            let collided = circle.collides_with(&poly);
            if collided {
                let manifold = circle.manifold(&poly);
                let depth = manifold.depths()[0].round().abs() as i32;
                self.x -= depth * self.vel_x.signum();
                self.y -= depth * self.vel_y.signum();
                let normal = manifold.normal();
                let x = normal.x().round().abs() as i32;
                let y = normal.y().round().abs() as i32;
                if x == 0 && y == 1 {
                    self.vel_y *= -1;
                } else if x == 1 && y == 0 {
                    self.vel_x *= -1;
                } else {
                    self.vel_x *= -1;
                    self.vel_y *= -1;
                }
            }
        }
        Ok(())
    }
    pub fn get_center(&self) -> Point {
        Point::new(self.x, self.y)
    }
}

struct Score {
    left: i32,
    right: i32,
}

impl Score {
    fn new() -> Score {
        Score { left: 0, right: 0 }
    }
    fn add_left(&mut self) {
        self.left += 1
    }
    fn add_right(&mut self) {
        self.right += 1
    }
}
