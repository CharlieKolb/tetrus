use amethyst::{
    core::transform::TransformBundle,
    prelude::*,
    renderer::{
        plugins::{RenderFlat2D, RenderToWindow},
        types::DefaultBackend,
        RenderingBundle,
    },
    utils::application_root_dir,
    assets::{HotReloadBundle},
    input::{InputBundle, StringBindings}
};

mod state;

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let app_root = application_root_dir()?;

    let resources = app_root.join("resources/");
    let display_config = resources.join("display_config.ron");

    let binding_path = resources.join("bindings.ron");
    let input_bundle = InputBundle::<StringBindings>::new()
        .with_bindings_from_file(binding_path)?;

    let game_data = GameDataBuilder::default()
        .with_bundle(TransformBundle::new())?
        // .with_bundle(HotReloadBundle::default())? // Doesn't work? :/
        .with_bundle(
            RenderingBundle::<DefaultBackend>::new()
                .with_plugin(
                    RenderToWindow::from_config_path(display_config)
                        .with_clear([0., 0., 0., 1.]),
                )
                .with_plugin(RenderFlat2D::default()),
        )?
        .with_bundle(input_bundle)?
        .with(state::BlockControllerSystem, "block_controller", &["input_system"])
        .with(state::MoveBlocksSystem, "block_system", &["block_controller"])
        .with(state::BoardToRealTranslatorSystem, "board_to_real_system", &["block_system"])
        ;

    let mut game = Application::new(resources, state::TetrisGameState::default(), game_data)?;
    game.run();

    Ok(())
}