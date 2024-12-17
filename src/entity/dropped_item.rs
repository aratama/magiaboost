use crate::controller::player::Player;
use crate::entity::EntityDepth;
use crate::inventory::InventoryItem;
use crate::inventory_item::InventoryItemType;
use crate::se::{SECommand, SE};
use crate::{asset::GameAssets, constant::*, states::GameState};
use bevy::core::FrameCount;
use bevy::prelude::*;
use bevy::text::FontSmoothing;
use bevy_aseprite_ultra::prelude::*;
use bevy_rapier2d::prelude::*;

use super::actor::Actor;
use super::life::Life;

#[derive(Component)]
pub struct DroppedItemEntity {
    item: InventoryItem,
}

#[derive(Component)]
struct SpellSprites {
    swing: f32,
    frame_count_offset: u32,
}

pub fn spawn_dropped_item(
    commands: &mut Commands,
    assets: &Res<GameAssets>,
    position: Vec2,
    item: InventoryItem,
) {
    let item_type = item.item_type;
    let icon = match item_type {
        InventoryItemType::Spell(spell) => spell.to_props().icon,
        InventoryItemType::Wand(wand) => wand.to_props().icon,
        InventoryItemType::Equipment(equipment) => equipment.to_props().icon,
    };
    let name = match item_type {
        InventoryItemType::Spell(spell) => spell.to_props().name.en,
        InventoryItemType::Wand(wand) => wand.to_props().name.en,
        InventoryItemType::Equipment(equipment) => equipment.to_props().name.en,
    };
    let frame_slice = match item_type {
        InventoryItemType::Wand(_) => "empty", //"wand_frame",
        InventoryItemType::Spell(_) if 0 < item.price => "spell_frame_yellow",
        InventoryItemType::Spell(_) => "spell_frame",
        InventoryItemType::Equipment(_) => "empty",
    };
    let collider_width = match item_type {
        InventoryItemType::Wand(_) => 16.0,
        _ => 8.0,
    };
    let swing = match item_type {
        InventoryItemType::Spell(_) => 2.0,
        InventoryItemType::Wand(_) => 0.0,
        InventoryItemType::Equipment(_) => 0.0,
    };
    commands
        .spawn((
            Name::new(format!("dropped item {}", name)),
            StateScoped(GameState::InGame),
            DroppedItemEntity { item },
            EntityDepth,
            InheritedVisibility::default(),
            Transform::from_translation(Vec3::new(position.x, position.y, 0.0)),
            GlobalTransform::default(),
            Life {
                life: 300,
                max_life: 300,
                amplitude: 0.0,
            },
            (
                RigidBody::Dynamic,
                LockedAxes::ROTATION_LOCKED,
                Damping {
                    linear_damping: 1.0,
                    angular_damping: 1.0,
                },
                Collider::cuboid(collider_width, 8.0),
                CollisionGroups::new(
                    ENTITY_GROUP,
                    ENTITY_GROUP
                        | WITCH_GROUP
                        | WITCH_BULLET_GROUP
                        | ENEMY_GROUP
                        | ENEMY_BULLET_GROUP
                        | WALL_GROUP,
                ),
                ActiveEvents::COLLISION_EVENTS,
                ExternalForce::default(),
                ExternalImpulse::default(),
            ),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    SpellSprites {
                        swing,
                        frame_count_offset: rand::random::<u32>() % 360,
                    },
                    Transform::from_xyz(0.0, 0.0, 0.0),
                    GlobalTransform::default(),
                    InheritedVisibility::default(),
                ))
                .with_children(|parent| {
                    if 0 < item.price {
                        parent.spawn((
                            Text2d(format!("{}", item.price)),
                            TextFont {
                                font: assets.dotgothic.clone(),
                                font_size: 24.0,
                                font_smoothing: FontSmoothing::None,
                            },
                            TextColor(Color::WHITE),
                            Transform::from_xyz(0.0, 14.0, 1.0)
                                .with_scale(Vec3::new(0.3, 0.3, 1.0)),
                        ));
                    }

                    parent.spawn((
                        AseSpriteSlice {
                            aseprite: assets.atlas.clone(),
                            name: frame_slice.into(),
                        },
                        Transform::from_xyz(0.0, 0.0, 0.0),
                    ));

                    parent.spawn((
                        AseSpriteSlice {
                            aseprite: assets.atlas.clone(),
                            name: icon.into(),
                        },
                        Transform::from_xyz(0.0, 0.0, 0.0001),
                    ));
                });
        });
}

fn swing(mut query: Query<(&mut Transform, &SpellSprites)>, frame_count: Res<FrameCount>) {
    for (mut transform, sprite) in query.iter_mut() {
        transform.translation.y =
            ((sprite.frame_count_offset + frame_count.0) as f32 * 0.05).sin() * sprite.swing;
    }
}

fn collision(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    item_query: Query<&DroppedItemEntity>,
    mut player_query: Query<&mut Actor, With<Player>>,
    mut global: EventWriter<SECommand>,
) {
    for collision_event in collision_events.read() {
        match collision_event {
            CollisionEvent::Started(a, b, _option) => {
                let _ = chat_start(
                    &mut commands,
                    a,
                    b,
                    &item_query,
                    &mut player_query,
                    &mut global,
                ) || chat_start(
                    &mut commands,
                    b,
                    a,
                    &item_query,
                    &mut player_query,
                    &mut global,
                );
            }
            CollisionEvent::Stopped(..) => {}
        }
    }
}

fn chat_start(
    commands: &mut Commands,
    a: &Entity,
    b: &Entity,
    item_query: &Query<&DroppedItemEntity>,
    player_query: &mut Query<&mut Actor, With<Player>>,
    global: &mut EventWriter<SECommand>,
) -> bool {
    match (item_query.get(*a), player_query.get_mut(*b)) {
        (Ok(item), Ok(mut actor)) => {
            if actor.inventory.insert(item.item) {
                commands.entity(*a).despawn_recursive();
                global.send(SECommand::new(SE::PickUp));
                return true;
            }
        }
        _ => return false,
    }
    return false;
}

pub struct SpellEntityPlugin;

impl Plugin for SpellEntityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            collision
                .run_if(in_state(GameState::InGame))
                .before(PhysicsSet::SyncBackend),
        );
        app.add_systems(Update, swing.run_if(in_state(GameState::InGame)));
    }
}
