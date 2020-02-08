use amethyst::{
    assets::{AssetStorage, Loader },
    ecs::{Component, DenseVecStorage},
    core::timing::Time,
    core::transform::Transform,
    core::SystemDesc,
    derive::SystemDesc,
    input::{get_key, is_close_requested, is_key_down, VirtualKeyCode},
    input::{InputHandler, StringBindings},
    prelude::*,
    ecs::prelude::{Join, Read, Write, Entity, Entities, System, SystemData, World, ReadStorage, WriteStorage},
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
    window::ScreenDimensions,
};

use rand::{ Rng, seq::SliceRandom };

use std::iter::FromIterator;

use log::info;

type Board = [[Option<Entity>; 10]; 24];

pub struct PieceBlock {}

impl Component for PieceBlock {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Clone, Debug)]
pub struct Piece {
    pub relative_coords: Vec<[(usize, usize); 4]>,
    pub idx: usize,
    pub coord: (usize, usize),
    pub time_since_drop: f32, // time in seconds since last drop
    pub base_time_to_drop: f32, // in blocks per second
    pub curr_time_to_drop: f32, // in blocks per second
    pub block_idx: usize, // 0 to 6
}

impl Component for Piece {
    type Storage = DenseVecStorage<Self>;
}

fn make_piece_I(coord: (usize, usize), blocks_per_second_drop_speed: f32) -> Piece {
    Piece {
        relative_coords: vec![
            [(0, 0), (0, 1), (0, 2), (0, 3)],
            [(0, 0), (1, 0), (2, 0), (3, 0)],
        ],
        idx: 0,
        coord,
        time_since_drop: 0.,
        base_time_to_drop: 1./blocks_per_second_drop_speed,
        curr_time_to_drop: 1./blocks_per_second_drop_speed,
        block_idx: 0,
    }
}

fn make_piece_L(coord: (usize, usize), blocks_per_second_drop_speed: f32) -> Piece {
    Piece {
        relative_coords: vec![
            [(0, 0), (1, 0), (1, 1), (1, 2)],
            [(0, 1), (1, 1), (2, 1), (2, 0)],
            [(0, 0), (0, 1), (0, 2), (1, 2)],
            [(0, 0), (1, 0), (2, 0), (0, 1)],
        ],
        idx: 0,
        coord,
        time_since_drop: 0.,
        base_time_to_drop: 1./blocks_per_second_drop_speed,
        curr_time_to_drop: 1./blocks_per_second_drop_speed,
        block_idx: 1,
    }
}

fn make_piece_rev_L(coord: (usize, usize), blocks_per_second_drop_speed: f32) -> Piece {
    Piece {
        relative_coords: vec![
            [(0, 0), (0, 1), (0, 2), (1, 0)],
            [(0, 0), (1, 0), (2, 0), (2, 1)],
            [(1, 0), (1, 1), (1, 2), (0, 2)],
            [(0, 0), (0, 1), (1, 1), (2, 1)],
        ],
        idx: 0,
        coord,
        time_since_drop: 0.,
        base_time_to_drop: 1./blocks_per_second_drop_speed,
        curr_time_to_drop: 1./blocks_per_second_drop_speed,
        block_idx: 2,
    }
}

fn make_piece_square(coord: (usize, usize), blocks_per_second_drop_speed: f32) -> Piece {
    Piece {
        relative_coords: vec![
            [(0, 0), (0, 1), (1, 0), (1, 1)],
        ],
        idx: 0,
        coord,
        time_since_drop: 0.,
        base_time_to_drop: 1./blocks_per_second_drop_speed,
        curr_time_to_drop: 1./blocks_per_second_drop_speed,
        block_idx: 3,
    }
}

fn make_piece_T(coord: (usize, usize), blocks_per_second_drop_speed: f32) -> Piece {
    Piece {
        relative_coords: vec![
            [(0, 1), (1, 1), (2, 1), (1, 0)],
            [(0, 0), (0, 1), (0, 2), (1, 1)],
            [(0, 0), (1, 0), (2, 0), (1, 1)],
            [(0, 1), (1, 0), (1, 1), (1, 2)],
        ],
        idx: 0,
        coord,
        time_since_drop: 0.,
        base_time_to_drop: 1./blocks_per_second_drop_speed,
        curr_time_to_drop: 1./blocks_per_second_drop_speed,
        block_idx: 4,
    }
}

fn make_piece_S(coord: (usize, usize), blocks_per_second_drop_speed: f32) -> Piece {
    Piece {
        relative_coords: vec![
            [(0, 0), (0, 1), (1, 1), (1, 2)],
            [(0, 1), (1, 1), (1, 0), (2, 0)],
        ],
        idx: 0,
        coord,
        time_since_drop: 0.,
        base_time_to_drop: 1./blocks_per_second_drop_speed,
        curr_time_to_drop: 1./blocks_per_second_drop_speed,
        block_idx: 5,
    }
}

fn make_piece_Z(coord: (usize, usize), blocks_per_second_drop_speed: f32) -> Piece {
    Piece {
        relative_coords: vec![
            [(1, 0), (1, 1), (0, 1), (0, 2)],
            [(0, 0), (1, 0), (1, 1), (2, 1)],
        ],
        idx: 0,
        coord,
        time_since_drop: 0.,
        base_time_to_drop: 1./blocks_per_second_drop_speed,
        curr_time_to_drop: 1./blocks_per_second_drop_speed,
        block_idx: 6,
    }
}

fn has_collision(piece: &Piece, board: &Board) -> bool {
    for &(x, y) in piece.relative_coords[piece.idx].iter() {
        let abs_x = piece.coord.0 + x;
        let abs_y = piece.coord.1 as i64 - y as i64;
        if abs_x >= 10 || abs_y < 0 || board[abs_y as usize][abs_x] != None {
            return true;
        }
    }
    false
}

impl Piece {
    // todo next and prev with bound checks and possible reverse
    fn next(&mut self, board: &Board)  {
        // backwards feels better
        let prev_idx = self.idx;
        self.idx = (self.idx + 3) % self.relative_coords.len();
        
        if has_collision(&self, &board) {
            // try again with left, right, up and down (all combinations?)
            self.idx = prev_idx;
        }
    }

    fn get_abs(&self) -> Vec<(usize, usize)> {
        self.relative_coords[self.idx].iter().map(|&(lX, lY)| (lX + self.coord.0, lY + self.coord.1)).collect()
    }

    fn move_down(&mut self, board: &Board) {
        if self.coord.1 != 0 {
            self.coord.1 -= 1;
        }
        if has_collision(&self, &board) {
            // self.coord.1 += 1;
        }
    }
}

pub struct PieceGenerator {
    current: Vec<Piece>,
    next_pieces: Vec<Piece>,
    options: [Piece; 7],
}

impl PieceGenerator {
    fn new() -> Self {
        let mut optionsInput =  [
            make_piece_I((0, 0), 0.),
            make_piece_S((0, 0), 0.),
            make_piece_Z((0, 0), 0.),
            make_piece_L((0, 0), 0.),
            make_piece_rev_L((0, 0), 0.),
            make_piece_square((0, 0), 0.),
            make_piece_T((0, 0), 0.),            
        ];
        let options = optionsInput.clone();
        optionsInput.shuffle(&mut rand::thread_rng());
        let current = Vec::from_iter(optionsInput.iter().cloned());
        
        optionsInput.shuffle(&mut rand::thread_rng());
        let next_pieces = Vec::from_iter(optionsInput.iter().cloned());


        Self {
            options,
            current,
            next_pieces
        }
    }

    fn peek(&self) -> Piece {
        self.current[0].clone()
    }

    fn next(&mut self, coord: (usize, usize), blocks_per_second_drop_speed: f32) -> Piece {
        let mut out = if self.current.len() == 1 {
            let piece = self.current[0].clone();
            self.options.shuffle(&mut rand::thread_rng());

            std::mem::swap(&mut self.current, &mut self.next_pieces);
            self.next_pieces = Vec::from_iter(self.options.iter().cloned());

            piece
        } else {
            self.current.remove(0)
        };

        out.coord = coord;
        out.base_time_to_drop = 1./blocks_per_second_drop_speed;
        out.curr_time_to_drop = 1./blocks_per_second_drop_speed;
        out
    }
}

pub struct Block {
    pub coord: (usize, usize),
}

impl Block {
    fn new(x: usize, y: usize) -> Self {
        Self {
            coord: (x, y),
        }
    }
}

fn coord_to_transform((x, y): (usize, usize)) -> Transform {
    let block_dimension = 16; // figure out how to read this based on state
    let mut transform = Transform::default();
    transform.set_translation_xyz(
        (block_dimension / 2 + x * block_dimension) as f32,
        (block_dimension / 2 + y * block_dimension) as f32,
        0. 
    );
    transform
}

// impl Default for Block {
//     fn default() -> Self {
//         Self::new()
//     }
// }

impl Component for Block {
    type Storage = DenseVecStorage<Self>;
}


pub struct Gameboard {
    pub board: [[Option<Entity>; 10]; 24],
    pub curr_piece: Option<Entity>,
    pub done_entities: Vec<Entity>,
}

impl Gameboard {
    pub fn can_place_blocks(&self, blocks: &Vec<(usize, usize)>) -> bool {
        for &(x, y) in blocks {
            if x >= 10 || y >= 24 {
                return false;
            }

            if self.board[y][x] != None {
                return false;
            }
        }
        return true;
    }

    pub fn place_blocks(&mut self, blocks: &Vec<(Entity, (usize, usize))>) {
        for &(entity, (x, y)) in blocks {
            self.board[y][x] = Some(entity);
        }
    }

    pub fn override_entity(&mut self, entity: Entity, coord: (usize, usize)) {
        self.board[coord.1][coord.0] = Some(entity);
    }

    pub fn can_settle(&self, blocks: &Vec<(usize, usize)>) -> bool {
        for &(x, y) in blocks {
            if y == 0 || self.board[y - 1][x] != None {
                return true;
            } 
        }

        return false;
    }

    pub fn clear_lines(&mut self) -> Vec<(Entity, (usize, usize))> {
        let destroyed_lines = self.board
            .iter()
            .enumerate()
            .filter_map(|(i, &line)| if line.iter().all(|&elem| elem != None) { Some(i) } else { None })
            .collect::<Vec<usize>>();
        
        if destroyed_lines.len() == 0 {
            return vec![];
        }

        let new_to_old_mapping = (0..self.board.len())
            .filter(|i| destroyed_lines.iter().find(|&e| e == i) == None)
            .enumerate()
            .collect::<Vec<(usize, usize)>>();

        let board = self.board;

        self.done_entities.extend(
            destroyed_lines
                .iter()
                .flat_map(|&idx| board[idx].iter().filter_map(|&e| e))
        );

        for &(new_line, old_line) in &new_to_old_mapping {
            self.board[new_line] = self.board[old_line];
        }

        for idx in new_to_old_mapping.len()..24 {
            self.board[idx] = [None; 10];
        }

        self.board
            .iter()
            .enumerate()
            .flat_map(|(j, line)| line
                                    .iter()
                                    .enumerate()
                                    .filter_map(|(i, &e)| e.map(|x| (i, x)))
                                    .map(move |(i, e)| (e, (i, j)))
            )
            .collect()
    }
}

impl Default for Gameboard { 
    fn default() -> Self {
        Self {
            board: [[None; 10]; 24],
            curr_piece: None,
            done_entities: vec![],
        }
    }
}


#[derive(SystemDesc)]
pub struct MovePieceSystem;

impl<'s> System<'s> for MovePieceSystem {
    type SystemData = (
        WriteStorage<'s, Piece>,
        Read<'s, Gameboard>,
        Read<'s, Time>,
    );

    fn run(&mut self, (mut pieces, gameboard, time): Self::SystemData) {
        let seconds = time.delta_seconds();
        for piece in (&mut pieces).join() {
            piece.time_since_drop += seconds;
            if piece.time_since_drop >= piece.curr_time_to_drop {
                piece.move_down(&gameboard.board);
                piece.time_since_drop %= piece.curr_time_to_drop;
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct PieceControllerSystem {
    curr_move_cd: f32,
    move_cd: f32,
    curr_rotate_cd: f32,
    rotate_cd: f32,
}

impl PieceControllerSystem {
    pub fn new() -> Self {
        Self {
            curr_move_cd: 0.,
            move_cd: 0.08,
            curr_rotate_cd: 0.,
            rotate_cd: 0.2,
        }
    }
}

fn clamp<T: PartialOrd> (min: T, val: T, max: T) -> T {
    if min > val {
        min
    }
    else if max < val {
        max
    }
    else {
        val
    }
}

impl<'s> System<'s> for PieceControllerSystem {
    type SystemData = (
        WriteStorage<'s, Piece>,
        Read<'s, InputHandler<StringBindings>>,
        Read<'s, Gameboard>,
        Read<'s, Time>
    );

    fn run(&mut self, (mut pieces, input, gameboard, time): Self::SystemData) {
        // this only works with ever having one piece
        // rotate_cd behaves weirdly
        for mut piece in (&mut pieces).join() {
            if input.action_is_down("down").unwrap_or(false) {
                piece.curr_time_to_drop = 0.2 * piece.base_time_to_drop;
            }
            else {
                piece.curr_time_to_drop = piece.base_time_to_drop;
            }
            
            if self.curr_rotate_cd == 0. {
                if input.action_is_down("up").unwrap_or(false) {
                    piece.next(&gameboard.board);
                    self.curr_rotate_cd = self.rotate_cd;
                }
            }
            else {
                self.curr_rotate_cd = f32::max(0., self.curr_rotate_cd - time.delta_seconds());
                if !input.action_is_down("up").unwrap_or(false) {
                    self.curr_rotate_cd = 0.;
                }
            }
            

            if self.curr_move_cd == 0. {
                let delta : i32 = match (input.action_is_down("left"), input.action_is_down("right")) {
                    (Some(true), Some(false)) => -1,
                    (Some(false), Some(true)) => 1,
                    _ => 0,
                };

                if delta != 0 {
                    self.curr_move_cd = self.move_cd;
                }
    
                let prev = piece.coord.0;
    
                piece.coord.0 = clamp(0, piece.coord.0 as i32 + delta, 9) as usize;
                if !gameboard.can_place_blocks(&piece.get_abs()) {
                    piece.coord.0 = prev;
                }
            }
            else {
                self.curr_move_cd = clamp(0., self.curr_move_cd - time.delta_seconds(), self.move_cd);
                if !input.action_is_down("left").unwrap_or(false) && !input.action_is_down("right").unwrap_or(false) {
                    self.curr_move_cd = 0.;
                }
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct BoardSettlerSystem;

impl<'s> System<'s> for BoardSettlerSystem {
    type SystemData = (
        Entities<'s>,
        WriteStorage<'s, Piece>,
        WriteStorage<'s, PieceBlock>,
        ReadStorage<'s, Block>,
        Write<'s, Gameboard>,
    );

    fn run(&mut self, (entities, mut pieces, mut piece_blocks, blocks, mut gameboard): Self::SystemData) {
        let mut to_be_deleted = vec![];
        for (entity, piece) in (&entities, &pieces).join() {
            if gameboard.can_settle(&piece.get_abs()) {
                gameboard.place_blocks(&piece.get_abs().iter().map(|&abs| (entity, abs)).collect());
                to_be_deleted.push(entity);
                gameboard.curr_piece = None;
            }
        }

        for &e in &to_be_deleted {
            pieces.remove(e);

            let entities_tbr = (&entities, &mut piece_blocks).join().map(|(e, _)| e).collect::<Vec<Entity>>();
            for e in entities_tbr {
                if let Some(block) = blocks.get(e) {
                    gameboard.override_entity(e, block.coord);
                }
                piece_blocks.remove(e);
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct BoardLineClearerSystem;

impl<'s> System<'s> for BoardLineClearerSystem {
    type SystemData = (
        Entities<'s>,
        WriteStorage<'s, Block>,
        Write<'s, Gameboard>
    );

    fn run(&mut self, (entities, mut blocks, mut gameboard): Self::SystemData) {
        let entity_map : std::collections::HashMap<Entity, (usize, usize)> = gameboard.clear_lines().into_iter().collect();
        for (entity, mut block) in (&entities, &mut blocks).join() {
            if let Some(&coord) = entity_map.get(&entity) {
                block.coord = coord;
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct PieceSyncSystem;

impl<'s> System<'s> for PieceSyncSystem {
    type SystemData = (
        ReadStorage<'s, Piece>,
        ReadStorage<'s, PieceBlock>,
        WriteStorage<'s, Block>
    );

    fn run(&mut self, (pieces, piece_blocks, mut blocks): Self::SystemData) {
        for piece in (pieces).join() {
            let coords = piece.get_abs();
            for (idx, (pB, mut block)) in (&piece_blocks, &mut blocks).join().enumerate() {
                if idx < 4 {
                    block.coord = coords[idx];
                }
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct BoardToRealTranslatorSystem;

impl<'s> System<'s> for BoardToRealTranslatorSystem {
    type SystemData = (
        ReadStorage<'s, Block>,
        WriteStorage<'s, Transform>,
    );

    fn run(&mut self, (block, mut transform): Self::SystemData) {
        for (block, transform) in (&block, &mut transform).join() {
            transform.set_translation(*coord_to_transform(block.coord).translation());
        }
    }
}

pub struct TetrisGameState {
    pub settings: (u32,), // todo make this a proper thing - right now only block dimension
    pub pieceGenerator: PieceGenerator,
    pub sprites: Vec<SpriteRender>,
}

impl Default for TetrisGameState {
    fn default() -> Self {
        Self {
            settings: (60,),
            pieceGenerator: PieceGenerator::new(),
            sprites: vec![],
        }
    }
}

impl SimpleState for TetrisGameState {
    // On start will run when this state is initialized. For more
    // state lifecycle hooks, see:
    // https://book.amethyst.rs/stable/concepts/state.html#life-cycle
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        let world = data.world;

        world.insert(Gameboard::default());

        // Get the screen dimensions so we can initialize the camera and
        // place our sprites correctly later. We'll clone this since we'll
        // pass the world mutably to the following functions.
        let dimensions = (*world.read_resource::<ScreenDimensions>()).clone();

        // Place the camera
        init_camera(world, &dimensions);

        // Load our sprites and display them
        self.sprites = load_sprites(world);
    }

    // fn handle_event(
    //     &mut self,
    //     mut _data: StateData<'_, GameData<'_, '_>>,
    //     event: StateEvent,
    // ) -> SimpleTrans {
    //     if let StateEvent::Window(event) = &event {
    //         // Check if the window should be closed
    //         if is_close_requested(&event) || is_key_down(&event, VirtualKeyCode::Escape) {
    //             return Trans::Quit;
    //         }

    //         // Listen to any key events
    //         if let Some(event) = get_key(&event) {
    //             info!("handling key event: {:?}", event);
    //         }

    //         // If you're looking for a more sophisticated event handling solution,
    //         // including key bindings and gamepad support, please have a look at
    //         // https://book.amethyst.rs/stable/pong-tutorial/pong-tutorial-03.html#capturing-user-input
    //     }

    //     // Keep going
    //     Trans::None
    // }

    fn update(&mut self, data: &mut StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
        if data.world.read_resource::<Gameboard>().curr_piece == None {
            // Load our sprites and display them

            let piece = self.pieceGenerator.next((4, 20), 5.);
            let block_idx = piece.block_idx;
            // falling block - to be set by something else at some point
            data.world.write_resource::<Gameboard>().curr_piece = Some(
                data.world.create_entity()
                    .with(piece)
                    .build()
                );

            for i in 0..4 {
                data.world.create_entity()
                    .with(PieceBlock {})
                    .with(Block::new(4, 20 + i))
                    .with(coord_to_transform((4, 20 + i)))
                    .with(self.sprites[block_idx].clone())
                    .build();
            }
        }

        let mut to_be_deleted = vec![];
        std::mem::swap(&mut to_be_deleted, &mut data.world.write_resource::<Gameboard>().done_entities);
        for e in to_be_deleted {
            data.world.delete_entity(e).ok();
        }

        Trans::None
    }
}

fn init_camera(world: &mut World, dimensions: &ScreenDimensions) {
    // Center the camera in the middle of the screen, and let it cover
    // the entire screen
    let mut transform = Transform::default();
    transform.set_translation_xyz(dimensions.width() * 0.5, dimensions.height() * 0.5, 1.);

    world
        .create_entity()
        .with(Camera::standard_2d(dimensions.width(), dimensions.height()))
        .with(transform)
        .build();
}

fn load_sprites(world: &mut World) -> Vec<SpriteRender> {
    // Load the texture for our sprites. We'll later need to
    // add a handle to this texture to our `SpriteRender`s, so
    // we need to keep a reference to it.
    let texture_handle = {
        let loader = world.read_resource::<Loader>();
        let texture_storage = world.read_resource::<AssetStorage<Texture>>();
        loader.load(
            "sprites/blocks.png",
            ImageFormat::default(),
            (),
            &texture_storage,
        )
    };

    // Load the spritesheet definition file, which contains metadata on our
    // spritesheet texture.
    let sheet_handle = {
        let loader = world.read_resource::<Loader>();
        let sheet_storage = world.read_resource::<AssetStorage<SpriteSheet>>();
        loader.load(
            "sprites/blocks.ron",
            SpriteSheetFormat(texture_handle),
            (),
            &sheet_storage,
        )
    };

    // Create our sprite renders. Each will have a handle to the texture
    // that it renders from. The handle is safe to clone, since it just
    // references the asset.
    (0..7)
        .map(|i| SpriteRender {
            sprite_sheet: sheet_handle.clone(),
            sprite_number: i,
        })
        .collect()
}

// fn init_sprites(world: &mut World, sprites: &[SpriteRender], dimensions: &ScreenDimensions) {
//     for (i, sprite) in sprites.iter().enumerate() {
//         // Center our sprites around the center of the window
//         let x = (i as f32 - 1.) * 100. + dimensions.width() * 0.5;
//         let y = (i as f32 - 1.) * 100. + dimensions.height() * 0.5;
//         let mut transform = Transform::default();
//         transform.set_translation_xyz(x, y, 0.);

//         // Create an entity for each sprite and attach the `SpriteRender` as
//         // well as the transform. If you want to add behaviour to your sprites,
//         // you'll want to add a custom `Component` that will identify them, and a
//         // `System` that will iterate over them. See https://book.amethyst.rs/stable/concepts/system.html
//         world
//             .create_entity()
//             .with(sprite.clone())
//             .with(transform)
//             .build();
//     }
// }