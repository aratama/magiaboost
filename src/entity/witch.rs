use super::actor::Actor;
use crate::asset::GameAssets;
use crate::constant::*;
use crate::hud::life_bar::{spawn_life_bar, LifeBarResource};
use crate::states::GameState;
use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::*;
use bevy_light_2d::light::{PointLight2d, PointLight2dBundle};
use bevy_rapier2d::prelude::*;
use uuid::Uuid;

pub const WITCH_COLLIDER_RADIUS: f32 = 5.0;

#[derive(Component)]
pub struct LightWithWitch {
    owner: Entity,
}

pub fn spawn_witch<T: Component>(
    commands: &mut Commands,
    assets: &Res<GameAssets>,
    position: Vec2,
    angle: f32,
    uuid: Uuid,
    name: Option<String>,
    life: i32,
    max_life: i32,
    res: &Res<LifeBarResource>,
    controller: T,
) {
    let mut entity = commands.spawn((
        Name::new("witch"),
        StateScoped(GameState::InGame),
        Actor {
            uuid,
            cooltime: 0,
            life,
            max_life,
            latest_damage: 0,
            pointer: Vec2::from_angle(angle),
        },
        controller,
        AsepriteAnimationBundle {
            aseprite: assets.player.clone(),
            transform: Transform::from_translation(position.extend(1.0)),
            animation: Animation::default().with_tag("idle").with_speed(0.2),
            sprite: Sprite {
                // flip_x: true,
                // ここもanchorは効かないことに注意。Aseprite側のpivotで設定
                // anchor: bevy::sprite::Anchor::Custom(Vec2::new(0.0, 1.0)),
                ..default()
            },
            ..default()
        },
        RigidBody::Dynamic,
        Velocity::default(),
        Collider::ball(WITCH_COLLIDER_RADIUS),
        GravityScale(0.0),
        LockedAxes::ROTATION_LOCKED,
        Damping {
            linear_damping: 6.0,
            angular_damping: 1.0,
        },
        ExternalForce::default(),
        ExternalImpulse::default(),
        CollisionGroups::new(ENEMY_GROUP, ENEMY_GROUP | WALL_GROUP | BULLET_GROUP),
    ));

    let index = entity.id();

    entity.with_children(move |spawn_children| {
        // リモートプレイヤーの名前
        // 自分のプレイヤーキャラクターは名前を表示しません
        if let Some(name) = name {
            let mut sections = Vec::new();
            sections.push(TextSection {
                value: name,
                style: TextStyle {
                    color: Color::hsla(120.0, 1.0, 0.5, 0.3),
                    font_size: 10.0,
                    ..default()
                },
            });
            spawn_children.spawn(Text2dBundle {
                text: Text {
                    sections,
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 20.0, 100.0),
                ..default()
            });
        }

        // リモートプレイヤーのライフバー
        // spawn_actor_life_bar(spawn_children, &mut meshes, &mut materials);

        spawn_life_bar(spawn_children, &res);
    });

    // SpriteBundle に PointLight2d を追加すると、画面外に出た時に Sprite が描画されなくなり、
    // ライトも描画されず不自然になるため、別で追加する
    // https://github.com/jgayfer/bevy_light_2d/issues/26
    entity.commands().spawn((
        LightWithWitch { owner: index },
        PointLight2dBundle {
            transform: Transform::from_translation(position.extend(2.0)),
            point_light: PointLight2d {
                radius: 150.0,
                intensity: 3.0,
                falloff: 10.0,
                ..default()
            },
            ..default()
        },
    ));
}

fn follow_light(
    mut commands: Commands,
    mut light_query: Query<(Entity, &LightWithWitch, &mut Transform), With<PointLight2d>>,
    witch_query: Query<&Transform, (With<Actor>, Without<PointLight2d>)>,
) {
    for (entity, light, mut transform) in light_query.iter_mut() {
        if let Ok(witch_transform) = witch_query.get(light.owner) {
            transform.translation.x = witch_transform.translation.x;
            transform.translation.y = witch_transform.translation.y;
        } else {
            commands.entity(entity).despawn_recursive();
        }
    }
}

pub struct WitchPlugin;

impl Plugin for WitchPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, follow_light.run_if(in_state(GameState::InGame)));
    }
}
