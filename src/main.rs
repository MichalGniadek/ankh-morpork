use avian3d::prelude::*;
use bevy::{
    color::palettes::{css::GOLD, tailwind},
    core_pipeline::tonemapping::Tonemapping,
    input::common_conditions::input_just_pressed,
    light::{NotShadowCaster, PointLightShadowMap},
    pbr::{Atmosphere, ScatteringMedium},
    post_process::bloom::Bloom,
    prelude::*,
    render::{experimental::occlusion_culling::OcclusionCulling, view::Hdr},
    window::{CursorGrabMode, CursorOptions},
};
use bevy_ahoy::{
    pickup::{
        actor::AvianPickupActorState,
        input::AvianPickupInput,
        prop::{PreferredPickupDistanceOverride, PreferredPickupRotation},
    },
    prelude::*,
};
use bevy_enhanced_input::prelude::{Press, *};
use bevy_trenchbroom::{physics::SceneCollidersReady, prelude::*};
use bevy_trenchbroom_avian::AvianPhysicsBackend;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TrenchBroomPlugins(
            TrenchBroomConfig::new("ankh-morpork")
                .default_solid_scene_hooks(|| SceneHooks::new().convex_collider()),
        ))
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(TrenchBroomPhysicsPlugin::new(AvianPhysicsBackend))
        .add_plugins(EnhancedInputPlugin)
        .add_plugins(AhoyPlugins::default())
        .add_input_context::<PlayerMovement>()
        .add_input_context::<PlayerLook>()
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
                init_lever,
                init_ticket,
                lower_bars,
                check_for_river,
                update_ticket,
                #[cfg(debug_assertions)]
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

fn init_lever(q: Query<Entity, Added<Lever>>, mut commands: Commands) {
    for entity in q {
        commands.entity(entity).insert((
            RigidBody::Static,
            CollisionLayers::new(CollisionLayer::Default, LayerMask::ALL),
            Collider::cuboid(1., 0.5, 1.),
        ));
    }
}

#[point_class]
struct Box;

#[solid_class]
struct River;

#[point_class(model("models/Button.gltf"), hooks(SceneHooks::new().spawn_class_gltf::<Self>()))]
struct Lever;

#[solid_class]
struct Bars;

#[point_class(color(255 217 0))]
struct Ticket;

fn init_ticket(
    q: Query<Entity, Added<Ticket>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for entity in q {
        commands.entity(entity).insert((
            Mesh3d(meshes.add(Plane3d::new(vec3(1., 0.3, 0.).normalize(), vec2(0.3, 0.2)))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: GOLD.into(),
                cull_mode: None,
                emissive: LinearRgba {
                    red: 1. * 50.,
                    green: 0.85 * 30.,
                    blue: 0.,
                    alpha: 1.,
                },
                ..default()
            })),
            Mass(0.1),
            RigidBody::Dynamic,
            GravityScale(0.),
            NotShadowCaster,
            CollisionLayers::new(CollisionLayer::Ticket, LayerMask::NONE),
            Collider::cuboid(0.3, 0.3, 0.3),
        ));
    }
}

#[derive(Debug, Component, Default)]
struct CollectedTickets(u32);

fn update_ticket(
    tickets: Query<(Entity, &mut Transform, Has<QueuedToDespawn>), With<Ticket>>,
    mut player: Single<&mut CollectedTickets>,
    pickup: Single<(Entity, &AvianPickupActorState)>,
    mut commands: Commands,
    time: Res<Time>,
    mut avian_pickup_input_writer: MessageWriter<AvianPickupInput>,
) {
    for (entity, mut tr, queued_to_despawn) in tickets {
        tr.rotate_axis(Dir3::Y, time.delta_secs());
        if *pickup.1 == AvianPickupActorState::Holding(entity)
            || *pickup.1 == AvianPickupActorState::Pulling(entity)
        {
            avian_pickup_input_writer.write(AvianPickupInput {
                action: pickup::input::AvianPickupAction::Drop,
                actor: pickup.0,
            });
            commands.entity(entity).insert(QueuedToDespawn);
            player.0 += 1;
        } else if queued_to_despawn {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Component)]
struct QueuedToDespawn;

#[derive(Component)]
struct PlayerLook;

#[derive(Component)]
struct PlayerMovement;

#[derive(Debug, PhysicsLayer, Default)]
enum CollisionLayer {
    #[default]
    Default,
    Player,
    Prop,
    Ticket,
}

const PLAYER_START_TRANSFORM: Transform = Transform::from_xyz(6.0, 11.5, 2.0);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Transform::from_xyz(0.0, 1.0, 0.0).looking_at(vec3(-0.2, -2.0, 0.1), Vec3::Y),
        DirectionalLight {
            // color: tailwind::GREEN_100.into(),
            illuminance: 10.,
            shadows_enabled: true,
            ..default()
        },
    ));

    commands
        .spawn(SceneRoot(asset_server.load("ankh.map#Scene")))
        .observe(
            |_: On<SceneCollidersReady>,
             mut commands: Commands,
             mut scattering_mediums: ResMut<Assets<ScatteringMedium>>| {
                let mut trans = PLAYER_START_TRANSFORM;
                trans.translation.y += 4.;

                let player = commands
                    .spawn((
                        CharacterController {
                            friction_hz: 36.,
                            air_speed: 11.,
                            max_air_wish_speed: 4.,
                            filter: SpatialQueryFilter::from_mask([
                                CollisionLayer::Default,
                                CollisionLayer::Prop,
                            ]),
                            ..default()
                        },
                        CollectedTickets::default(),
                        Collider::cylinder(0.4, 1.8),
                        CollisionLayers::new(CollisionLayer::Player, LayerMask::DEFAULT),
                        trans,
                        PlayerLook,
                        actions!(PlayerLook[
                            (
                                Action::<RotateCamera>::new(),
                                Scale::splat(0.07),
                                Bindings::spawn((
                                    Spawn(Binding::mouse_motion()),
                                    Axial::right_stick()
                                ))
                            ),
                        ]),
                        PlayerMovement,
                        actions!(PlayerMovement[
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
                    // #[cfg(debug_assertions)]
                    OcclusionCulling,
                    Projection::Perspective(PerspectiveProjection {
                        fov: 70f32.to_radians(),
                        near: 0.01,
                        ..default()
                    }),
                    Hdr,
                    Bloom {
                        intensity: 0.5,
                        ..default()
                    },
                    Tonemapping::default(),
                    Atmosphere::earthlike(scattering_mediums.add(ScatteringMedium::default())),
                    CharacterControllerCameraOf::new(player),
                    PickupConfig {
                        prop_filter: SpatialQueryFilter::from_mask([
                            CollisionLayer::Prop,
                            CollisionLayer::Ticket,
                        ]),
                        actor_filter: SpatialQueryFilter::from_mask(CollisionLayer::Player),
                        obstacle_filter: SpatialQueryFilter::from_mask(CollisionLayer::Default),
                        interaction_distance: 2.5,
                        ..default()
                    },
                ));
            },
        );
}

#[derive(Debug, Component)]
struct SteppedOnButton;

fn check_for_river(
    mut state: Single<(
        Entity,
        &CharacterControllerState,
        &mut CharacterController,
        &mut Transform,
    )>,
    rivers: Query<(), With<River>>,
    lever: Query<(), With<Lever>>,
    mut commands: Commands,
    time: Res<Time>,
) {
    let Some(grounded) = state.1.grounded else {
        return;
    };
    if lever.contains(grounded.entity) {
        commands.entity(state.0).insert(SteppedOnButton);
    }
    if !rivers.contains(grounded.entity) {
        return;
    }

    commands
        .entity(state.0)
        .insert(ContextActivity::<PlayerMovement>::INACTIVE);

    state.2.standing_view_height -= 0.5 * time.delta_secs();

    if state.2.standing_view_height <= 0.1 {
        state.2.standing_view_height = CharacterController::default().standing_view_height;
        *state.3 = PLAYER_START_TRANSFORM;
        commands
            .entity(state.0)
            .insert(ContextActivity::<PlayerMovement>::ACTIVE);
    }
}

fn lower_bars(
    bars: Query<&mut Transform, With<Bars>>,
    player: Option<Single<&SteppedOnButton>>,
    time: Res<Time>,
) {
    if player.is_some() {
        for mut bar in bars {
            bar.translation.y -= 0.8 * time.delta_secs();
        }
    }
}
