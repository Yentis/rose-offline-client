use bevy::{
    ecs::query::WorldQuery,
    prelude::{AssetServer, Commands, Entity, Local, Query, Res, ResMut, State, With},
};
use bevy_egui::{egui, EguiContexts};
use egui::{Align, ComboBox, Grid, Layout, TextEdit, Ui, Widget, Window};
use regex::Regex;
use rose_data::{AbilityType, EquipmentIndex, SkillData, SkillDatabase, SkillId};
use rose_game_common::{
    components::{CharacterGender, Equipment},
    messages::client::ClientMessage,
};
use std::sync::Arc;

use crate::{
    animation::SkeletalAnimation,
    components::{
        CharacterModel, Command, CommandCastSkill, CommandCastSkillState, CommandCastSkillTarget,
        NextCommand, PlayerCharacter,
    },
    resources::{
        AppState, GameConnection, GameData, SelectedTarget, UiResources, UiSpriteSheetType,
    },
    ui::{
        tooltips::{PlayerTooltipQuery, SkillTooltipType},
        ui_add_skill_tooltip, UiStateDebugWindows,
    },
};

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct QueryCommand<'w> {
    entity: Entity,
    command: &'w mut Command,
}

#[derive(WorldQuery)]
pub struct QueryCharacter<'w> {
    entity: Entity,
    character_model: &'w CharacterModel,
    equipment: &'w Equipment,
}

pub struct UiStateDebugSkillList {
    ability_types: Vec<AbilityType>,
    filter_name: String,
    filter_ability_type: Option<AbilityType>,
    filter_castable: bool,
    filter_base: bool,
    filter_named: bool,
    filtered_skills: Vec<SkillId>,
}

impl Default for UiStateDebugSkillList {
    fn default() -> Self {
        UiStateDebugSkillList {
            ability_types: Vec::default(),
            filter_name: String::default(),
            filter_ability_type: None,
            filter_castable: false,
            filter_base: true,
            filter_named: true,
            filtered_skills: Vec::default(),
        }
    }
}

fn create_controls(
    ui: &mut Ui,
    state: &mut Local<UiStateDebugSkillList>,
    game_data: &Res<GameData>,
) -> bool {
    let mut filter_changed = false;

    Grid::new("skill_list_controls_grid")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Name");
            filter_changed |= TextEdit::singleline(&mut state.filter_name)
                .ui(ui)
                .changed();
            ui.end_row();

            ui.label("Effect");
            ComboBox::from_id_source("skill_effect_filter")
                .selected_text(
                    state
                        .filter_ability_type
                        .map_or("All", |it| game_data.string_database.get_ability_type(it)),
                )
                .show_ui(ui, |ui| {
                    filter_changed |= ui
                        .selectable_value(&mut state.filter_ability_type, None, "All")
                        .changed();

                    for ability_type in state.ability_types.clone() {
                        let ability_text = game_data.string_database.get_ability_type(ability_type);
                        if ability_text.is_empty() {
                            continue;
                        }

                        filter_changed |= ui
                            .selectable_value(
                                &mut state.filter_ability_type,
                                Some(ability_type),
                                ability_text,
                            )
                            .changed();
                    }
                });
            ui.end_row();

            ui.horizontal(|ui| {
                filter_changed |= ui
                    .checkbox(&mut state.filter_castable, "Castable")
                    .changed();
                filter_changed |= ui.checkbox(&mut state.filter_base, "Base level").changed();
                filter_changed |= ui.checkbox(&mut state.filter_named, "Has name").changed();
            });
            ui.end_row();
        });

    if state.filter_name.is_empty() && state.filtered_skills.is_empty() {
        filter_changed = true;
    }

    filter_changed
}

fn set_filtered_skills(state: &mut Local<UiStateDebugSkillList>, game_data: &Res<GameData>) {
    let filter_name_re = if !state.filter_name.is_empty() {
        Some(Regex::new(&format!("(?i){}", regex::escape(&state.filter_name))).unwrap())
    } else {
        None
    };

    state.filtered_skills = game_data
        .skills
        .iter()
        .filter_map(|skill_data| {
            if (state.filter_castable && skill_data.casting_motion_id.is_none())
                || !filter_name_re
                    .as_ref()
                    .map_or(true, |re| re.is_match(skill_data.name))
            {
                return None;
            }

            if state.filter_base
                && skill_data
                    .base_skill_id
                    .is_some_and(|it| it != skill_data.id)
            {
                return None;
            }

            if state.filter_named && skill_data.name.is_empty() {
                return None;
            }

            if state
                .filter_ability_type
                .is_some_and(|it| !get_skill_ability_types(skill_data).contains(&it))
            {
                return None;
            }

            Some(skill_data.id)
        })
        .collect();
}

fn get_skill_ability_types(skill_data: &SkillData) -> Vec<AbilityType> {
    let mut ability_types: Vec<AbilityType> = Vec::new();

    for (use_ability_type, _) in &skill_data.use_ability {
        if ability_types.contains(use_ability_type) {
            continue;
        }

        ability_types.push(*use_ability_type);
    }

    for add_ability in &skill_data.add_ability {
        let Some(add_ability_type) = add_ability.as_ref().map(|it| it.ability_type) else {
            continue;
        };

        if ability_types.contains(&add_ability_type) {
            continue;
        }

        ability_types.push(add_ability_type);
    }

    ability_types
}

fn get_ability_types(skill_database: &Arc<SkillDatabase>) -> Vec<AbilityType> {
    let mut ability_types: Vec<AbilityType> = Vec::new();

    for skill_data in skill_database.iter() {
        for ability_type in get_skill_ability_types(skill_data).iter() {
            if ability_types.contains(ability_type) {
                continue;
            }

            ability_types.push(*ability_type);
        }
    }

    ability_types
}

pub fn ui_debug_skill_list_system(
    mut commands: Commands,
    mut egui_context: EguiContexts,
    mut ui_state: Local<UiStateDebugSkillList>,
    game_connection: Option<Res<GameConnection>>,
    app_state: Res<State<AppState>>,
    asset_server: Res<AssetServer>,
    game_data: Res<GameData>,
    ui_resources: Res<UiResources>,
    selected_target: Res<SelectedTarget>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut query_player_command: Query<QueryCommand, With<PlayerCharacter>>,
    query_character_models: Query<QueryCharacter, With<CharacterModel>>,
    query_player_tooltip: Query<PlayerTooltipQuery, With<PlayerCharacter>>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    if ui_state.ability_types.is_empty() {
        ui_state.ability_types = get_ability_types(&game_data.skills)
    }

    Window::new("Skill List")
        .resizable(true)
        .default_height(300.0)
        .open(&mut ui_state_debug_windows.skill_list_open)
        .show(egui_context.ctx_mut(), |ui| {
            let filter_changed = create_controls(ui, &mut ui_state, &game_data);
            if filter_changed {
                set_filtered_skills(&mut ui_state, &game_data);
            }

            let player_tooltip_data = query_player_tooltip.get_single().ok();

            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .cell_layout(Layout::left_to_right(Align::Center))
                .column(egui_extras::Column::exact(45.0))
                .column(egui_extras::Column::initial(50.0).at_least(50.0))
                .column(egui_extras::Column::remainder().at_least(80.0))
                .column(egui_extras::Column::initial(100.0).at_least(100.0))
                .column(egui_extras::Column::initial(100.0).at_least(100.0))
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("Icon");
                    });
                    header.col(|ui| {
                        ui.heading("ID");
                    });
                    header.col(|ui| {
                        ui.heading("Name");
                    });
                    header.col(|ui| {
                        ui.heading("Type");
                    });
                    header.col(|ui| {
                        ui.heading("Action");
                    });
                })
                .body(|body| {
                    body.rows(
                        45.0,
                        ui_state.filtered_skills.len(),
                        |row_index, mut row| {
                            let Some(skill_data) = ui_state
                                .filtered_skills
                                .get(row_index)
                                .and_then(|id| game_data.skills.get_skill(*id))
                            else {
                                return;
                            };

                            row.col(|ui| {
                                let Some(sprite) = ui_resources.get_sprite_by_index(
                                    UiSpriteSheetType::Skill,
                                    skill_data.icon_number as usize,
                                ) else {
                                    return;
                                };

                                ui.add(
                                    egui::Image::new(sprite.texture_id, [40.0, 40.0]).uv(sprite.uv),
                                )
                                .on_hover_ui(|ui| {
                                    ui_add_skill_tooltip(
                                        ui,
                                        SkillTooltipType::Extra,
                                        &game_data,
                                        player_tooltip_data.as_ref(),
                                        skill_data.id,
                                    );
                                });
                            });

                            row.col(|ui| {
                                ui.label(format!("{}", skill_data.id.get()));
                            });

                            row.col(|ui| {
                                ui.label(skill_data.name);
                            });

                            row.col(|ui| {
                                ui.label(format!("{:?}", skill_data.skill_type));
                            });

                            row.col(|ui| {
                                if matches!(app_state.get(), AppState::Game)
                                    && ui.button("Learn").clicked()
                                {
                                    if let Some(game_connection) = game_connection.as_ref() {
                                        game_connection
                                            .client_message_tx
                                            .send(ClientMessage::Chat {
                                                text: format!("/skill add {}", skill_data.id.get()),
                                            })
                                            .ok();
                                    }
                                }

                                if skill_data.casting_motion_id.is_none() {
                                    return;
                                }

                                if matches!(app_state.get(), AppState::Game) {
                                    if let Ok(mut player) = query_player_command.get_single_mut() {
                                        if let Command::CastSkill(command_cast_skill) =
                                            player.command.as_mut()
                                        {
                                            if command_cast_skill.skill_id == skill_data.id
                                                && !command_cast_skill.ready_action
                                            {
                                                if ui.button("Action").clicked() {
                                                    command_cast_skill.ready_action = true;
                                                }
                                            } else if ui.button("Stop").clicked() {
                                                *player.command = Command::with_stop();
                                            }
                                        } else if ui.button("Cast").clicked() {
                                            commands.entity(player.entity).insert(
                                                NextCommand::with_cast_skill(
                                                    skill_data.id,
                                                    selected_target.selected.map(|target_entity| {
                                                        CommandCastSkillTarget::Entity(
                                                            target_entity,
                                                        )
                                                    }),
                                                    None,
                                                    None,
                                                    None,
                                                ),
                                            );
                                        }
                                    };
                                } else if matches!(app_state.get(), AppState::ModelViewer) {
                                    if ui.button("Cast").clicked() {
                                        for character in query_character_models.iter() {
                                            let weapon_item_data = character
                                                .equipment
                                                .get_equipment_item(EquipmentIndex::Weapon)
                                                .and_then(|weapon_item| {
                                                    game_data.items.get_weapon_item(
                                                        weapon_item.item.item_number,
                                                    )
                                                });

                                            let weapon_motion_type = weapon_item_data
                                                .map(|weapon_item_data| {
                                                    weapon_item_data.motion_type as usize
                                                })
                                                .unwrap_or(0);

                                            let weapon_motion_gender =
                                                match character.character_model.gender {
                                                    CharacterGender::Male => 0,
                                                    CharacterGender::Female => 1,
                                                };

                                            let motion_data = skill_data
                                                .casting_motion_id
                                                .and_then(|motion_id| {
                                                    game_data
                                                        .character_motion_database
                                                        .find_first_character_motion(
                                                            motion_id,
                                                            weapon_motion_type,
                                                            weapon_motion_gender,
                                                        )
                                                });

                                            let Some(motion_data) = motion_data else {
                                                continue;
                                            };

                                            commands
                                                .entity(character.entity)
                                                .insert(Command::CastSkill(CommandCastSkill {
                                                    skill_id: skill_data.id,
                                                    skill_target: None,
                                                    action_motion_id: skill_data.action_motion_id,
                                                    cast_motion_id: skill_data.casting_motion_id,
                                                    cast_repeat_motion_id: skill_data
                                                        .casting_repeat_motion_id,
                                                    cast_skill_state:
                                                        CommandCastSkillState::Casting,
                                                    ready_action: true,
                                                }))
                                                .insert(
                                                    SkeletalAnimation::once(
                                                        asset_server.load(motion_data.path.path()),
                                                    )
                                                    .with_animation_speed(
                                                        skill_data.casting_motion_speed,
                                                    ),
                                                );
                                        }
                                    }

                                    if ui.button("Action").clicked() {
                                        for character in query_character_models.iter() {
                                            let weapon_item_data = character
                                                .equipment
                                                .get_equipment_item(EquipmentIndex::Weapon)
                                                .and_then(|weapon_item| {
                                                    game_data.items.get_weapon_item(
                                                        weapon_item.item.item_number,
                                                    )
                                                });
                                            let weapon_motion_type = weapon_item_data
                                                .map(|weapon_item_data| {
                                                    weapon_item_data.motion_type as usize
                                                })
                                                .unwrap_or(0);
                                            let weapon_motion_gender =
                                                match character.character_model.gender {
                                                    CharacterGender::Male => 0,
                                                    CharacterGender::Female => 1,
                                                };

                                            let motion_data =
                                                skill_data.action_motion_id.and_then(|motion_id| {
                                                    game_data
                                                        .character_motion_database
                                                        .find_first_character_motion(
                                                            motion_id,
                                                            weapon_motion_type,
                                                            weapon_motion_gender,
                                                        )
                                                });

                                            if let Some(motion_data) = motion_data {
                                                commands
                                                    .entity(character.entity)
                                                    .insert(Command::CastSkill(CommandCastSkill {
                                                        skill_id: skill_data.id,
                                                        skill_target: None,
                                                        action_motion_id: skill_data
                                                            .action_motion_id,
                                                        cast_motion_id: skill_data
                                                            .casting_motion_id,
                                                        cast_repeat_motion_id: skill_data
                                                            .casting_repeat_motion_id,
                                                        cast_skill_state:
                                                            CommandCastSkillState::Action,
                                                        ready_action: true,
                                                    }))
                                                    .insert(
                                                        SkeletalAnimation::once(
                                                            asset_server
                                                                .load(motion_data.path.path()),
                                                        )
                                                        .with_animation_speed(
                                                            skill_data.action_motion_speed,
                                                        ),
                                                    );
                                            }
                                        }
                                    }
                                }
                            });
                        },
                    );
                });
        });
}
