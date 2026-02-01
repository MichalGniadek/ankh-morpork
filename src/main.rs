use avian3d::prelude::*;
use bevy::{
    color::palettes::tailwind,
    input::common_conditions::input_just_pressed,
    pbr::{Atmosphere, ScatteringMedium},
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};
use bevy_ahoy::prelude::*;
use bevy_enhanced_input::prelude::*;
use bevy_trenchbroom::prelude::*;
use bevy_trenchbroom_avian::AvianPhysicsBackend;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        // TODO: disable config creation when release
        .add_plugins(TrenchBroomPlugins(
            TrenchBroomConfig::new("ankh-morpork")
                .default_solid_scene_hooks(|| SceneHooks::new().convex_collider()),
        ))
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(TrenchBroomPhysicsPlugin::new(AvianPhysicsBackend))
        .add_plugins(EnhancedInputPlugin)
        .add_plugins(AhoyPlugins::default())
        .add_input_context::<PlayerInput>()
        .insert_resource(GlobalAmbientLight {
            color: tailwind::BLUE_400.into(),
            brightness: 450.,
            ..Default::default()
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                capture_cursor.run_if(input_just_pressed(MouseButton::Left)),
                release_cursor.run_if(input_just_pressed(KeyCode::Escape)),
            ),
        )
        .run()
}

fn capture_cursor(mut cursor: Single<&mut CursorOptions>) {
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
}

fn release_cursor(mut cursor: Single<&mut CursorOptions>) {
    cursor.visible = true;
    cursor.grab_mode = CursorGrabMode::None;
}

#[derive(Component)]
struct PlayerInput;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut scattering_mediums: ResMut<Assets<ScatteringMedium>>,
) {
    commands.spawn(SceneRoot(asset_server.load("ankh.map#Scene")));

    commands.spawn((
        Transform::from_xyz(0.0, 1.0, 0.0).looking_at(vec3(-0.2, -2.0, 0.1), Vec3::Y),
        DirectionalLight {
            color: tailwind::GREEN_100.into(),
            shadows_enabled: true,
            ..default()
        },
    ));

    let player = commands
        .spawn((
            CharacterController {
                friction_hz: 24.,
                ..default()
            },
            Collider::cylinder(0.4, 1.8),
            Transform::from_xyz(6.0, 13.0, 3.0),
            PlayerInput,
            actions!(PlayerInput[
                (
                    Action::<Movement>::new(),
                    DeadZone::default(),
                    Bindings::spawn((
                        Cardinal::wasd_keys(),
                        Cardinal::arrows(),
                        Axial::left_stick()
                    ))
                ),
                (
                    Action::<Jump>::new(),
                    bindings![KeyCode::Space,  GamepadButton::South],
                ),
                (
                    Action::<Crouch>::new(),
                    bindings![KeyCode::ControlLeft, GamepadButton::LeftTrigger],
                ),
                (
                    Action::<RotateCamera>::new(),
                    Scale::splat(0.07),
                    Bindings::spawn((
                        Spawn(Binding::mouse_motion()),
                        Axial::right_stick()
                    ))
                ),
            ]),
        ))
        .id();

    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 70f32.to_radians(),
            near: 0.01,
            ..default()
        }),
        Atmosphere::earthlike(scattering_mediums.add(ScatteringMedium::default())),
        CharacterControllerCameraOf::new(player),
    ));
}
