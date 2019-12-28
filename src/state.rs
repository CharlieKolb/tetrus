use amethyst::{
    assets::{AssetStorage, Loader },
    ecs::{Component, DenseVecStorage},
    core::timing::Time,
    core::transform::Transform,
    core::SystemDesc,
    derive::SystemDesc,
    input::{get_key, is_close_requested, is_key_down, VirtualKeyCode},
    prelude::*,
    ecs::prelude::{Join, Read, Entity, System, SystemData, World, ReadStorage, WriteStorage},
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
    window::ScreenDimensions,
};

use log::info;

pub struct Block {
    pub coord: (u32, u32),
}

impl Block {
    fn new(x: u32, y: u32) -> Self {
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
}

impl Default for Gameboard { 
    fn default() -> Self {
        Self {
            board: [[None; 10]; 24],
            curr_block: None,
        }
    }
}

pub struct TetrisGameState {
    pub gameboard: Gameboard,
    pub settings: (u32,), // todo make this a proper thing - right now only block dimension
}

impl Default for TetrisGameState {
    fn default() -> Self {
        Self {
            gameboard: Gameboard::default(),
            settings: (60,)
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
                if block.coord.1 == 0 {
                    // make static block / remove moving_block
                }
                else {
                    block.coord.1 -= 1;
                }
                moving_block.time_since_drop %= moving_block.time_to_drop;
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



impl SimpleState for TetrisGameState {
    // On start will run when this state is initialized. For more
    // state lifecycle hooks, see:
    // https://book.amethyst.rs/stable/concepts/state.html#life-cycle
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        let world = data.world;
        world.register::<Block>();
        world.register::<MovingBlock>();

        world.insert(self.settings.0);

        // Get the screen dimensions so we can initialize the camera and
        // place our sprites correctly later. We'll clone this since we'll
        // pass the world mutably to the following functions.
        let dimensions = (*world.read_resource::<ScreenDimensions>()).clone();

        // Place the camera
        init_camera(world, &dimensions);

        // Load our sprites and display them
        let sprites = load_sprites(world);



        self.gameboard.curr_block = Some(world.create_entity()
            .with(Block::new(4, 21))
            .with(MovingBlock::new(1.))
            .with(Transform::default())
            .with(sprites[0].clone())
            .build());

        for i in 0..10 {
            for j  in 0..23 {
                world.create_entity()
                    .with(Block::new(i, j))
                    // .with(MovingBlock::new(1.))
                    .with(Transform::default())
                    .with(sprites[0].clone())
                    .build();
            }
        }

        // world.create_entity()
        //     .with(Block::new())
        //     .with(MovingBlock::new(30.))
        //     .with(block_transform2)
        //     .with(sprites[1].clone())
        //     .build();
    }

    fn handle_event(
        &mut self,
        mut _data: StateData<'_, GameData<'_, '_>>,
        event: StateEvent,
    ) -> SimpleTrans {
        if let StateEvent::Window(event) = &event {
            // Check if the window should be closed
            if is_close_requested(&event) || is_key_down(&event, VirtualKeyCode::Escape) {
                return Trans::Quit;
            }

            // Listen to any key events
            if let Some(event) = get_key(&event) {
                info!("handling key event: {:?}", event);
            }

            // If you're looking for a more sophisticated event handling solution,
            // including key bindings and gamepad support, please have a look at
            // https://book.amethyst.rs/stable/pong-tutorial/pong-tutorial-03.html#capturing-user-input
        }

        // Keep going
        Trans::None
    }

    fn update(&mut self, _: &mut StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
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