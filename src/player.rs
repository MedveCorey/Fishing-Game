use bevy::prelude::*;
use crate::hud::HintDisplay;
use crate::inventory::PlayerInventory;
use crate::map::*;
use crate::button::*;
use crate::resources::*;
use crate::window::*;
use crate::fishing_view::*;
use std::time::Duration;

pub const PLAYER_WIDTH: f32 = 64.;
pub const PLAYER_HEIGHT: f32 = 128.;

const PLAYER_SPEED: f32 = 70.;
//240
const RUN_SPEED: f32 = 540.; // Added RUN_SPEED for running
pub const ANIM_TIME: f32 = 0.125; // 8 fps
pub const FISHING_ANIM_TIME: f32 = 0.25; // 4 frames per second for fishing animation

const UP: KeyCode = KeyCode::KeyW;
const LEFT: KeyCode = KeyCode::KeyA;
const DOWN: KeyCode = KeyCode::KeyS;
const RIGHT: KeyCode = KeyCode::KeyD;
const RUN: KeyCode = KeyCode::ShiftRight;

#[derive(Component)]
pub struct Player;

#[derive(Component, PartialEq, Clone)]
pub enum PlayerDirection {
    Front,
    Back,
    Left,
    Right,
}

#[derive(Component)]
pub struct Forageable;

#[derive(Default, Component)]
pub struct CanPickUp {
    pub isitem: bool,
}

#[derive(Default, Component)]
pub struct InputStack {
    stack: Vec<KeyCode>,
}

impl InputStack {
    fn push(&mut self, key: KeyCode) {
        if !self.stack.contains(&key) {
            self.stack.push(key);
        }
    }

    fn remove(&mut self, key: KeyCode) {
        self.stack.retain(|&k| k != key);
    }

    fn last(&self) -> Option<KeyCode> {
        self.stack.last().copied()
    }
}

pub fn move_player(
    state: Res<State<MapState>>,
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut player: Query<(&mut Transform, &mut PlayerDirection, &Location, &Animation, &mut InputStack, &mut CanPickUp), With<Player>>,
    inventory: Query<&PlayerInventory>,
    collision_query: Query<(&Transform, &Tile), (With<Collision>, Without<Player>)>,
    mut fish_button: Query<&mut Visibility, With<FishingButton>>,
    mut hint_display: Query<&mut Visibility, (With<HintDisplay>, Without<FishingButton>)>,
    mut next_fishing_area: ResMut<NextState<FishingLocal>>
) {
    let (mut pt, mut direction, location, animation, mut input_stack, mut CanPickUp) = player.single_mut();
    let mut fish_button_visibility = fish_button.single_mut();
    let mut hint_visibility = hint_display.single_mut();

    // Map transition
    if state.eq(&MapState::MapTransition) {
        let elapsed: f32 = time.elapsed_seconds() - animation.start_time;
        
        if elapsed < animation.duration {
            pt.translation = animation.start_position + elapsed / animation.duration * animation.motion;
        } else {
            pt.translation = animation.start_position + animation.motion;
        }

        return;
    }

    // Update key press list
    let is_running = input.pressed(RUN); 

    if input.pressed(UP) {
        input_stack.push(UP);
    } else {
        input_stack.remove(UP);
    }

    if input.pressed(LEFT) {
        input_stack.push(LEFT);
    } else {
        input_stack.remove(LEFT);
    }

    if input.pressed(DOWN) {
        input_stack.push(DOWN);
    } else {
        input_stack.remove(DOWN);
    }

    if input.pressed(RIGHT) {
        input_stack.push(RIGHT);
    } else {
        input_stack.remove(RIGHT);
    }

    // Determine velocity vector
    let mut change_direction = if let Some(last_key) = input_stack.last() {
        match last_key {
            KeyCode::KeyW => {
                *direction = PlayerDirection::Back;
                Vec2::new(0., 1.)
            }
            KeyCode::KeyS => {
                *direction = PlayerDirection::Front;
                Vec2::new(0., -1.)
            }
            KeyCode::KeyA => {
                *direction = PlayerDirection::Left;
                Vec2::new(-1., 0.)
            }
            KeyCode::KeyD => {
                *direction = PlayerDirection::Right;
                Vec2::new(1., 0.)
            }
            _ => Vec2::ZERO
        }
    } else {
        Vec2::ZERO
    };

    // Adjust movement speed based on running
    let speed = if is_running { RUN_SPEED } else { PLAYER_SPEED };

    if change_direction != Vec2::ZERO {
        change_direction = speed * time.delta_seconds() * change_direction
    }

    if change_direction.length() == 0. {
        return;
    }

    // Calculate new position
    // Snap to edge of screen
    let min_pos = Vec3::new(
        location.x as f32 * WIN_W - WIN_W / 2. + PLAYER_WIDTH / 2.,
        location.y as f32 * WIN_H - WIN_H / 2. + PLAYER_HEIGHT / 2.,
        pt.translation.z,
    );

    let max_pos = Vec3::new(
        location.x as f32 * WIN_W + WIN_W / 2. - PLAYER_WIDTH / 2.,
        location.y as f32 * WIN_H + WIN_H / 2. - PLAYER_HEIGHT / 2.,
        pt.translation.z,
    );

    let mut new_pos = (pt.translation + Vec3::new(change_direction.x, change_direction.y, pt.translation.z)).clamp(min_pos, max_pos);

    // Check for tile collisions
    for object in collision_query.iter() {
        let (transform, tile) = object;

        if new_pos.y - PLAYER_HEIGHT / 2. >= transform.translation.y + tile.hitbox.y / 2.
            || new_pos.y + PLAYER_HEIGHT / 2. <= transform.translation.y - tile.hitbox.y / 2. 
            || new_pos.x + PLAYER_WIDTH / 2. <= transform.translation.x - tile.hitbox.x / 2. 
            || new_pos.x - PLAYER_WIDTH / 2. >= transform.translation.x + tile.hitbox.x / 2.
        {
            CanPickUp.isitem = false;
            continue;
        }
        
        // Collision detected
        // Snap player to edge of tile
        match *direction {
            PlayerDirection::Back => {
                // Snap to bottom of tile
                pt.translation.y = transform.translation.y - (tile.hitbox.y + PLAYER_HEIGHT) / 2.;
            },
            PlayerDirection::Front => {
                pt.translation.y = transform.translation.y + (tile.hitbox.y + PLAYER_HEIGHT) / 2.;
            },
            PlayerDirection::Left => {
                pt.translation.x = transform.translation.x + (tile.hitbox.x + PLAYER_WIDTH) / 2.;
            },
            PlayerDirection::Right => {
                pt.translation.x = transform.translation.x - (tile.hitbox.x + PLAYER_WIDTH) / 2.;
            }
        }

        if tile.interactable {
            match tile {
                &Tile::GOLDLINE => {
                    CanPickUp.isitem = true;
                    *fish_button_visibility = Visibility::Visible;
                }
                &Tile::WATER => {
                    CanPickUp.isitem = false;
                    next_fishing_area.set(FishingLocal::Pond1);
                    *fish_button_visibility = Visibility::Visible;
                    *hint_visibility = Visibility::Hidden;
                }
                &Tile::WATER2 => {
                    CanPickUp.isitem = false;
                    next_fishing_area.set(FishingLocal::Pond2);
                    *fish_button_visibility = Visibility::Visible;
                    *hint_visibility = Visibility::Hidden;
                }
                &Tile::WATEROCEAN => {
                    // Requires surf rod
                    CanPickUp.isitem = false;
                    let inv = inventory.single();

                    for rod in inv.rods.iter() {
                        if rod.name.eq("Surf Rod") {
                            next_fishing_area.set(FishingLocal::Ocean);
                            *fish_button_visibility = Visibility::Visible;
                            *hint_visibility = Visibility::Hidden;
                            return;
                        }
                    }

                    *hint_visibility = Visibility::Visible;
                }
                &Tile::SHOP => {
                    CanPickUp.isitem = false;
                    *fish_button_visibility = Visibility::Hidden;
                    *hint_visibility = Visibility::Hidden;
                }
                _ => {
                    *fish_button_visibility = Visibility::Hidden;
                    *hint_visibility = Visibility::Hidden;
                }
            }
        }

        return;
    }

    // No collision
    *fish_button_visibility = Visibility::Hidden;
    *hint_visibility = Visibility::Hidden;
    pt.translation = new_pos;
}

pub fn animate_player(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut player: Query<(
        &mut Handle<Image>,
        &mut TextureAtlas,
        &mut AnimationTimer,
        &AnimationFrameCount,
        &PlayerDirection,
        &InputStack
    )>
) {
    let (_texture_handle, mut texture_atlas, mut timer, _frame_count, direction, input_stack) = player.single_mut();
    timer.set_duration(Duration::from_secs_f32(FISHING_ANIM_TIME));

    let is_moving = input_stack.stack.len() > 0;

    let dir_add = match *direction {
        PlayerDirection::Front => {
            if is_moving { 4 } else { 0 }
        }
        PlayerDirection::Back => {
            if is_moving { 12 } else { 2 }
        }
        PlayerDirection::Left => {
            if is_moving { 16 } else { 3 }
        }
        PlayerDirection::Right => {
            if is_moving { 8 } else { 1 }
        }
    };

    let is_running = input.pressed(KeyCode::ShiftRight); 
    let anim_speed = if is_running {time.delta()*3} else {time.delta()};

    timer.tick(anim_speed);
    if is_moving {
        if timer.just_finished() {
            texture_atlas.index = ((texture_atlas.index + 1) % 4) + dir_add;
        }
    } else {
        texture_atlas.index = dir_add;
    }
}