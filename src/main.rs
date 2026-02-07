use avian3d::prelude::*;
use bevy::{
    color::palettes::tailwind,
    input::common_conditions::input_just_pressed,
    light::PointLightShadowMap,
    pbr::{Atmosphere, ScatteringMedium},
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};
use bevy_ahoy::{
    pickup::prop::{PreferredPickupDistanceOverride, PreferredPickupRotation},
    prelude::*,
};
use bevy_enhanced_input::prelude::{Press, *};
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
        .insert_resource(PointLightShadowMap { size: 32 })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                capture_cursor.run_if(input_just_pressed(MouseButton::Left)),
                release_cursor.run_if(input_just_pressed(KeyCode::Escape)),
                init_box,
                speedup_lights,
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

#[allow(unused)]
fn speedup_lights(q: Query<&mut PointLight, Added<PointLight>>) {
    for mut l in q {
        l.shadows_enabled = false;
    }
}

fn init_box(
    q: Query<Entity, Added<Box>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    for entity in q {
        commands.entity(entity).insert((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            // MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color_texture: Some(
                    asset_server.load("textures/timber_square_planks_cross.png"),
                ),
                perceptual_roughness: 1.,
                ..default()
            })),
            Mass(15.),
            RigidBody::Dynamic,
            TransformInterpolation,
            PreferredPickupRotation::default(),
            PreferredPickupDistanceOverride(1.5),
            CollisionLayers::new(CollisionLayer::Prop, LayerMask::ALL),
            Collider::cuboid(1., 1., 1.),
        ));
    }
}

// #[point_class(hooks(SceneHooks::new().push(update_box)))]
#[point_class]
struct Box;

#[derive(Component)]
struct PlayerInput;

#[derive(Debug, PhysicsLayer, Default)]
enum CollisionLayer {
    #[default]
    Default,
    Player,
    Prop,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut scattering_mediums: ResMut<Assets<ScatteringMedium>>,
) {
    commands.spawn(SceneRoot(asset_server.load("ankh.map#Scene")));

    commands.spawn((
        Transform::from_xyz(0.0, 1.0, 0.0).looking_at(vec3(-0.2, -2.0, 0.1), Vec3::Y),
        DirectionalLight {
            // color: tailwind::GREEN_100.into(),
            illuminance: 10.,
            shadows_enabled: true,
            ..default()
        },
    ));

    let player = commands
        .spawn((
            CharacterController {
                friction_hz: 36.,
                ..default()
            },
            Collider::cylinder(0.4, 1.8),
            CollisionLayers::new(CollisionLayer::Player, LayerMask::ALL),
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
                    Action::<RotateCamera>::new(),
                    Scale::splat(0.07),
                    Bindings::spawn((
                        Spawn(Binding::mouse_motion()),
                        Axial::right_stick()
                    ))
                ),
                (
                    Action::<PullObject>::new(),
                    ActionSettings { consume_input: true, ..default() },
                    Press::default(),
                    bindings![MouseButton::Left, MouseButton::Right]
                ),
                (
                    Action::<DropObject>::new(),
                    ActionSettings { consume_input: true, ..default() },
                    Press::default(),
                    bindings![MouseButton::Left, MouseButton::Right]
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
        PickupConfig {
            prop_filter: SpatialQueryFilter::from_mask(CollisionLayer::Prop),
            actor_filter: SpatialQueryFilter::from_mask(CollisionLayer::Player),
            obstacle_filter: SpatialQueryFilter::from_mask(CollisionLayer::Default),
            ..default()
        },
    ));
}
