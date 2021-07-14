use std::{cell::RefCell, rc::Rc};

use crate::{
    engine::vector::Vector2,
    engine::world::World,
    gpu::{self, ISSBO, SSBO},
    player::Player,
};
use serde::Deserialize;

#[derive(Deserialize, Clone)]
#[repr(C)]
pub struct Sprite {
    pub position: Vector2<f32>,
    texture_idx: u32,
}

impl ISSBO for Sprite {}

impl Sprite {
    pub fn new(position: Vector2<f32>, texture_idx: u32) -> Sprite {
        Sprite {
            position,
            texture_idx,
        }
    }
}

///This struct is for retrieving intermediate sprite processing results
#[allow(dead_code)]
#[derive(Default)]
#[repr(C)]
pub struct SpritePreprocessResult {
    sprite_width: i32,
    sprite_height: i32,

    pub draw_start_y: i32,
    pub draw_end_y: i32,
    pub draw_start_x: i32,
    pub draw_end_x: i32,

    sprite_screen_x: i32,
    pub transform_y: f64,
}

pub struct Sprites {
    _gpu_capacity: isize,
    sprites: Vec<Rc<RefCell<Sprite>>>,
    _sprites_ssbo: SSBO,
    _preprocess_ssbo: SSBO,
}

impl Sprites {
    pub fn new(world: &World, player: &Player) -> Sprites {
        let sprites: Vec<Rc<RefCell<Sprite>>> = world
            .sprites()
            .iter()
            .map(|f| Rc::new(RefCell::new(f.clone())))
            .collect();

        let _sprites_ssbo = gpu::SSBO::empty(
            8,
            (sprites.len() * std::mem::size_of::<Sprite>()) as isize,
            gl::DYNAMIC_DRAW,
        );

        let _preprocess_ssbo = gpu::SSBO::empty(
            9,
            (sprites.len() * std::mem::size_of::<SpritePreprocessResult>()) as isize,
            gl::DYNAMIC_DRAW,
        );

        let mut sprites = Sprites {
            _gpu_capacity: sprites.len() as isize,
            sprites,
            _sprites_ssbo,
            _preprocess_ssbo,
        };

        sprites.update(player);

        sprites
    }

    ///Add a new sprite to the world
    ///Triggers copying the new sprite list to the gpu
    pub fn add(&mut self, sprite: Sprite, player: &Player) -> Rc<RefCell<Sprite>> {
        let sprite = Rc::new(RefCell::new(sprite));
        self.sprites.push(sprite.clone());

        if self.sprites.len() >= self._gpu_capacity as usize {
            self._gpu_capacity *= 2;
            self._sprites_ssbo = gpu::SSBO::empty(
                8,
                self._gpu_capacity * std::mem::size_of::<Sprite>() as isize,
                gl::DYNAMIC_DRAW,
            );
            self._preprocess_ssbo = gpu::SSBO::empty(
                9,
                self._gpu_capacity * std::mem::size_of::<SpritePreprocessResult>() as isize,
                gl::DYNAMIC_DRAW,
            );
        }

        self.update(player);
        sprite
    }

    ///Triggers updating the gpu data
    pub fn update(&mut self, player: &Player) {
        self.sort(player);
        for i in 0..self.sprites.len() {
            let s = self.sprites[i].borrow_mut();

            self._sprites_ssbo
                .update(&*s, (i * std::mem::size_of::<Sprite>()) as isize);
        }
    }

    ///Sorts the sprite array by the distance to the player
    fn sort(&mut self, player: &Player) {
        self.sprites.sort_by(|a, b| {
            let a = a.borrow_mut();
            let b = b.borrow_mut();

            let da = a.position.dist_sq(player.position());
            let db = b.position.dist_sq(player.position());

            db.partial_cmp(&da).unwrap()
        });
    }

    pub fn preprocess_result(&self, index: u32) -> SpritePreprocessResult {
        self._preprocess_ssbo.retrieve::<SpritePreprocessResult>(
            index as isize * std::mem::size_of::<SpritePreprocessResult>() as isize,
        )
    }

    pub fn len(&self) -> usize {
        return self.sprites.len();
    }
}