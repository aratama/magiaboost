use super::actor::Actor;
use super::book_shelf::BookShelf;
use crate::config::GameConfig;
use crate::constant::{BULLET_GROUP, ENEMY_GROUP, WALL_GROUP};
use crate::states::GameState;
use crate::world::wall::WallCollider;
use crate::{asset::GameAssets, audio::play_se};
use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::{Aseprite, AsepriteSliceBundle};
use bevy_kira_audio::Audio;
use bevy_light_2d::light::PointLight2d;
use bevy_particle_systems::{
    ColorOverTime, JitteredValue, ParticleBurst, ParticleSystem, ParticleSystemBundle, Playing,
};
use bevy_rapier2d::prelude::*;
use std::collections::HashSet;
use uuid::Uuid;

const SLICE_NAME: &str = "bullet";

static BULLET_Z: f32 = 10.0;

static BULLET_IMPULSE: f32 = 20000.0;

pub const BULLET_RADIUS: f32 = 5.0;

const BULLET_DAMAGE: i32 = 5;

const BULLET_LIFETIME: u32 = 240;

// 弾丸発射時の、キャラクターと弾丸の間隔
// 小さすぎると、キャラクターの移動時に発射したときに自分自身が衝突してしまうが、
// 大きすぎるとキャラクターと弾丸の位置が離れすぎて不自然
pub const BULLET_SPAWNING_MARGIN: f32 = 4.0;

#[derive(Component, Reflect)]
pub struct Bullet {
    life: u32,
    damage: i32,
    impulse: f32,
    owner: Option<Uuid>,
}

#[derive(Bundle)]
pub struct BulletBundle {
    name: Name,
    bullet: Bullet,
    transform: Transform,
}

pub fn spawn_bullet(
    commands: &mut Commands,
    aseprite: Handle<Aseprite>,
    position: Vec2,
    velocity: Vec2,
    owner: Option<Uuid>,
    assets: &Res<GameAssets>,
    audio: &Res<Audio>,
    config: &Res<GameConfig>,
) {
    play_se(audio, config, assets.suburi.clone());

    commands.spawn((
        Name::new("bullet"),
        StateScoped(GameState::InGame),
        Bullet {
            life: BULLET_LIFETIME,
            damage: BULLET_DAMAGE,
            impulse: BULLET_IMPULSE,
            owner,
        },
        AsepriteSliceBundle {
            aseprite,
            slice: SLICE_NAME.into(),
            transform: Transform::from_xyz(position.x, position.y, BULLET_Z)
                * Transform::from_rotation(Quat::from_rotation_z(velocity.to_angle())), // .looking_to(velocity.extend(BULLET_Z), Vec3::Z)
            ..default()
        },
        (
            Velocity {
                linvel: velocity,
                angvel: 0.0,
            },
            KinematicCharacterController::default(),
            RigidBody::KinematicVelocityBased,
            // 弾丸が大きくなると衝突時の位置の精度が悪化するので小さくしてあります
            Collider::ball(BULLET_RADIUS),
            GravityScale(0.0),
            // https://rapier.rs/docs/user_guides/bevy_plugin/colliders#active-collision-types
            ActiveCollisionTypes::default() | ActiveCollisionTypes::KINEMATIC_STATIC,
            ActiveEvents::COLLISION_EVENTS,
            Sleeping::disabled(),
            Ccd::enabled(),
            // https://rapier.rs/docs/user_guides/bevy_plugin/colliders#collision-groups-and-solver-groups
            CollisionGroups::new(BULLET_GROUP, WALL_GROUP | ENEMY_GROUP),
        ),
        PointLight2d {
            radius: 50.0,
            intensity: 1.0,
            falloff: 10.0,
            color: Color::hsl(245.0, 1.0, 0.6),
            ..default()
        },
    ));
}

fn update_bullet(
    mut commands: Commands,
    mut bullet_query: Query<(Entity, &mut Bullet, &Transform, &Velocity)>,
    mut enemy_query: Query<(&mut Actor, &mut ExternalImpulse)>,
    mut bookshelf_query: Query<&mut BookShelf>,
    assets: Res<GameAssets>,
    mut collision_events: EventReader<CollisionEvent>,
    wall_collider_query: Query<Entity, With<WallCollider>>,
    audio: Res<Audio>,
    config: Res<GameConfig>,
) {
    // 弾丸のライフタイムを減らし、ライフタイムが尽きたら削除
    for (entity, mut bullet, _, _) in bullet_query.iter_mut() {
        bullet.life -= 1;
        if bullet.life <= 0 {
            commands.entity(entity).despawn_recursive();
        }
    }

    let mut despownings: HashSet<Entity> = HashSet::new();

    for collision_event in collision_events.read() {
        match collision_event {
            CollisionEvent::Started(a, b, _) => {
                if !process_bullet_event(
                    &mut commands,
                    &assets,
                    &mut bullet_query,
                    &mut enemy_query,
                    &mut bookshelf_query,
                    &mut despownings,
                    &a,
                    &b,
                    &wall_collider_query,
                    &audio,
                    &config,
                ) {
                    process_bullet_event(
                        &mut commands,
                        &assets,
                        &mut bullet_query,
                        &mut enemy_query,
                        &mut bookshelf_query,
                        &mut despownings,
                        &b,
                        &a,
                        &wall_collider_query,
                        &audio,
                        &config,
                    );
                }
            }
            _ => {}
        }
    }
}

fn process_bullet_event(
    mut commands: &mut Commands,
    assets: &Res<GameAssets>,
    query: &Query<(Entity, &mut Bullet, &Transform, &Velocity)>,

    // TODO プレイヤーキャラくらーにもダメージが入るようにする
    actors: &mut Query<(&mut Actor, &mut ExternalImpulse)>,
    bookshelf_query: &mut Query<&mut BookShelf>,
    respownings: &mut HashSet<Entity>,
    a: &Entity,
    b: &Entity,
    wall_collider_query: &Query<Entity, With<WallCollider>>,
    audio: &Res<Audio>,
    config: &Res<GameConfig>,
) -> bool {
    if let Ok((bullet_entity, bullet, bullet_transform, bullet_velocity)) = query.get(*a) {
        let bullet_position = bullet_transform.translation.truncate();

        // 弾丸が壁の角に当たった場合、衝突イベントが同時に複数回発生するため、
        // すでにdespownしたentityに対して再びdespownしてしまうことがあり、
        // 警告が出るのを避けるため、処理済みのentityを識別するセットを使っています
        // https://github.com/bevyengine/bevy/issues/5617
        if !respownings.contains(&bullet_entity) {
            respownings.insert(bullet_entity.clone());
            commands.entity(bullet_entity).despawn_recursive();
            spawn_particle_system(&mut commands, bullet_position);

            if let Ok((mut actor, mut impilse)) = actors.get_mut(*b) {
                // 弾丸がアクターに衝突したとき
                // このクエリにはプレイヤーキャラクター自身、発射したキャラクター自身も含まれることに注意
                // 弾丸の詠唱者自身に命中した場合はダメージやノックバックはなし
                if bullet.owner == None || Some(actor.uuid) != bullet.owner {
                    actor.life = (actor.life - bullet.damage).max(0);
                    impilse.impulse += bullet_velocity.linvel.normalize_or_zero() * bullet.impulse;
                    play_se(&audio, config, assets.dageki.clone());
                }
            } else if let Ok(mut bookshelf) = bookshelf_query.get_mut(*b) {
                // 弾丸が本棚に衝突したとき
                // TODO: この調子で破壊可能オブジェクトを増やすと、システムの引数やifの分岐が増えてしまう
                // Breakableコンポーネントにしてまとめる？
                // でも破壊したときの効果が物体によって異なるのでまとめられない？
                bookshelf.life -= bullet.damage;
                play_se(&audio, config, assets.dageki.clone());
            } else if let Ok(_) = wall_collider_query.get(*b) {
                play_se(&audio, config, assets.asphalt.clone());
            } else {
                play_se(&audio, config, assets.shibafu.clone());
            }
            true
        } else {
            false
        }
    } else {
        false
    }
}

fn spawn_particle_system(commands: &mut Commands, position: Vec2) {
    commands
        // Add the bundle specifying the particle system itself.
        .spawn((
            Name::new("particle system"),
            StateScoped(GameState::InGame),
            ParticleSystemBundle {
                transform: Transform::from_translation(position.extend(BULLET_Z)),
                particle_system: ParticleSystem {
                    spawn_rate_per_second: 0.0.into(),
                    max_particles: 100,
                    initial_speed: JitteredValue::jittered(50.0, -50.0..50.0),
                    lifetime: JitteredValue::jittered(0.2, -0.05..0.05),
                    color: ColorOverTime::Constant(Color::WHITE),
                    bursts: vec![ParticleBurst {
                        // このシステムのスケジュールをUpdate意外に設定し、このtimeを0.0にすると、
                        // パーティクルシステムを設置してそのGlobalTransformが更新される前にパーティクルが生成されてしまうため、
                        // パーティクルの発生位置が原点になってしまうことに注意
                        // 0.1くらいにしておくと0.0ではないので大丈夫っぽい
                        time: 0.1,
                        count: 20,
                    }],
                    system_duration_seconds: 0.2,
                    ..ParticleSystem::oneshot()
                },
                ..ParticleSystemBundle::default()
            },
            Playing,
            PointLight2d {
                radius: 50.0,
                intensity: 1.0,
                falloff: 10.0,
                color: Color::hsl(245.0, 1.0, 0.6),
                ..default()
            },
        ));
}

pub struct BulletPlugin;

impl Plugin for BulletPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            update_bullet.run_if(in_state(GameState::InGame)),
        );
        app.register_type::<Bullet>();
    }
}
