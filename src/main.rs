use bevy::{color::palettes::css::*, prelude::*};
use bevy_prototype_lyon::prelude::*;

fn main() {
    App::new()
        .insert_resource(Msaa::Sample4)
        .add_plugins((DefaultPlugins, ShapePlugin))
        .add_systems(Startup, setup_system)
        .run();
}

fn setup_system(mut commands: Commands) {
    let shape = shapes::Circle {
        radius: 10.0,
        center: Vec2::ZERO,
    };
    commands.spawn(Camera2dBundle::default());

    // Use a single entity with multiple shapes to improve performance
    let shape_count = 100_000; // Keep the original count
    let mut shape_bundle = ShapeBundle {
        path: GeometryBuilder::build_as(&shape),
        ..default()
    };

    // Spawn a single entity with multiple shapes
    commands.spawn_batch((0..shape_count).map(move |_| {
        let shape_bundle = ShapeBundle {
            path: GeometryBuilder::build_as(&shape),
            ..default()
        };
        (
            shape_bundle,
            Fill::color(DARK_CYAN),
            Stroke::new(BLACK, 1.0),
        )
    }));
}
