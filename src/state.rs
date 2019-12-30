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

use log::info;

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

// impl Default for Block {
//     fn default() -> Self {
//         Self::new()
//     }
// }

impl Component for Block {
    type Storage = DenseVecStorage<Self>;
}

pub struct MovingBlock {
    pub time_since_drop: f32, // time in seconds since last drop
    pub time_to_drop: f32, // in blocks per second
}

impl MovingBlock {
    fn new(blocks_per_second_drop_speed: f32) -> Self {
        Self {
            time_since_drop: 0.,
            time_to_drop: 1./blocks_per_second_drop_speed,
        }
    }
}

impl Component for MovingBlock {
    type Storage = DenseVecStorage<Self>;
}

pub struct Gameboard {
    pub board: [[Option<Entity>; 10]; 24],
    pub curr_block: Option<Entity>,
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

        self.board[destroyed_lines[0]..]
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
            curr_block: None,
            done_entities: vec![],
        }
    }
}


#[derive(SystemDesc)]
pub struct MoveBlocksSystem;

impl<'s> System<'s> for MoveBlocksSystem {
    type SystemData = (
        WriteStorage<'s, MovingBlock>,
        WriteStorage<'s, Block>,
        Read<'s, Time>,
    );

    fn run(&mut self, (mut moving_block, mut transform, time): Self::SystemData) {
        let seconds = time.delta_seconds();
        for (moving_block, block) in (&mut moving_block, &mut transform).join() {
            moving_block.time_since_drop += seconds;
            if moving_block.time_since_drop >= moving_block.time_to_drop {
                if block.coord.1 != 0 {
                    block.coord.1 -= 1;
                }
                moving_block.time_since_drop %= moving_block.time_to_drop;
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct BlockControllerSystem {
    time_since_move: f32,
    time_to_move: f32,
}

impl BlockControllerSystem {
    pub fn new() -> Self {
        Self {
            time_since_move: 0.,
            time_to_move: 0.1,
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

impl<'s> System<'s> for BlockControllerSystem {
    type SystemData = (
        WriteStorage<'s, Block>,
        ReadStorage<'s, MovingBlock>,
        Read<'s, InputHandler<StringBindings>>,
        Read<'s, Gameboard>,
        Read<'s, Time>
    );

    fn run(&mut self, (mut block, moving_block, input, gameboard, time): Self::SystemData) {
        self.time_since_move += time.delta_seconds();
        if self.time_since_move < self.time_to_move {
            return;
        }
        self.time_since_move -= self.time_to_move;
        

        for (mut block, _) in (&mut block, &moving_block).join() {
            let delta : i32 = match (input.action_is_down("left"), input.action_is_down("right")) {
                (Some(true), Some(false)) => -1,
                (Some(false), Some(true)) => 1,
                _ => 0,
            };

            let prev = block.coord.0;

            block.coord.0 = clamp(0, block.coord.0 as i32 + delta, 9) as usize;
            if !gameboard.can_place_blocks(&vec![block.coord]) {
                block.coord.0 = prev;
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct BoardSettlerSystem;

impl<'s> System<'s> for BoardSettlerSystem {
    type SystemData = (
        Entities<'s>,
        ReadStorage<'s, Block>,
        WriteStorage<'s, MovingBlock>,
        Write<'s, Gameboard>,
    );

    fn run(&mut self, (entities, block, mut moving_block, mut gameboard): Self::SystemData) {
        let mut to_be_unmoved = vec![];
        for (entity, block, _) in (&entities, &block, &moving_block).join() {
            if gameboard.can_settle(&vec![block.coord]) {
                gameboard.place_blocks(&vec![(entity, block.coord)]);
                to_be_unmoved.push(entity);
                gameboard.curr_block = None;
            }
        }

        for &e in &to_be_unmoved {
            moving_block.remove(e);
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
pub struct BoardToRealTranslatorSystem;

impl<'s> System<'s> for BoardToRealTranslatorSystem {
    type SystemData = (
        ReadStorage<'s, Block>,
        WriteStorage<'s, Transform>,
    );

    fn run(&mut self, (block, mut transform): Self::SystemData) {
        let block_dimension = 16; // figure out how to read this based on state
        for (block, transform) in (&block, &mut transform).join() {
            transform.set_translation_xyz(
                (block_dimension / 2 + block.coord.0 * block_dimension) as f32,
                (block_dimension / 2 + (block.coord.1) * block_dimension ) as f32,
                0. 
            );
        }
    }
}

pub struct TetrisGameState {
    pub settings: (u32,), // todo make this a proper thing - right now only block dimension
    pub sprites: Vec<SpriteRender>,
}

impl Default for TetrisGameState {
    fn default() -> Self {
        Self {
            settings: (60,),
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

        for i in 0..10 {
            for j  in 0..23 {
                // world.create_entity()
                //     .with(Block::new(i, j))
                //     // .with(MovingBlock::new(1.))
                //     .with(Transform::default())
                //     .with(self.sprites[0].clone())
                //     .build();
            }
        }

        // world.create_entity()
        //     .with(Block::new())
        //     .with(MovingBlock::new(30.))
        //     .with(block_transform2)
        //     .with(sprites[1].clone())
        //     .build();
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
        if data.world.read_resource::<Gameboard>().curr_block == None {
            // Load our sprites and display them

            // falling block - to be set by something else at some point
            data.world.write_resource::<Gameboard>().curr_block = Some(
            data.world.create_entity()
                .with(Block::new(4, 21))
                .with(MovingBlock::new(15.))
                .with(Transform::default())
                .with(self.sprites[1].clone())
                .build());
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
    (0..2)
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