use crate::{
    components::{Command, PlayerCharacter},
    resources::{GameConnection, GameData, UiResources},
    ui::{
        tooltips::POSITIVE_EFFECT_COLOR,
        widgets::{DataBindings, Dialog, DrawText},
        UiSoundEvent, UiStateWindows,
    },
};
use bevy::{
    ecs::query::WorldQuery,
    prelude::{Assets, EventWriter, Local, Query, Res, ResMut, With},
};
use bevy_egui::{egui, EguiContexts};
use egui::{pos2, vec2, Color32, Frame, Grid, Response, RichText, Window};
use rose_file_readers::QsdEquation;
use rose_game_common::{
    components::{
        AbilityValues, BasicStatType, BasicStats, CharacterInfo, Equipment, ExperiencePoints,
        Level, MoveMode, MoveSpeed, SkillList, Stamina, StatPoints, StatusEffects, MAX_STAMINA,
    },
    data::PassiveRecoveryState,
    messages::client::ClientMessage,
};

const IID_BTN_CLOSE: i32 = 10;
// const IID_BTN_DIALOG2ICON: i32 = 11;
const IID_TABBEDPANE: i32 = 20;
const IID_TAB_BASICINFO: i32 = 21;
// const IID_TAB_BASICINFO_BG: i32 = 22;
// const IID_TAB_BASICINFO_BTN: i32 = 23;
const IID_GUAGE_STAMINA: i32 = 24;
const IID_TAB_ABILITY: i32 = 31;
// const IID_TAB_ABILITY_BG: i32 = 32;
// const IID_TAB_ABILITY_BTN: i32 = 33;
const IID_BTN_UP_STR: i32 = 34;
const IID_BTN_UP_DEX: i32 = 35;
const IID_BTN_UP_INT: i32 = 36;
const IID_BTN_UP_CON: i32 = 37;
const IID_BTN_UP_CHARM: i32 = 38;
const IID_BTN_UP_SENSE: i32 = 39;
const IID_TAB_UNION: i32 = 41;
// const IID_TAB_UNION_BG: i32 = 42;
// const IID_TAB_UNION_BTN: i32 = 43;

pub struct UiStateCharacterInfo {
    current_tab: i32,
    simulated_ability_values: Option<AbilityValues>,
    simulate_amount: i32,
}

impl Default for UiStateCharacterInfo {
    fn default() -> Self {
        Self {
            current_tab: IID_TAB_BASICINFO,
            simulated_ability_values: None,
            simulate_amount: 1,
        }
    }
}

#[derive(WorldQuery)]
pub struct PlayerQuery<'w> {
    ability_values: &'w AbilityValues,
    basic_stats: &'w BasicStats,
    character_info: &'w CharacterInfo,
    experience_points: &'w ExperiencePoints,
    level: &'w Level,
    move_speed: &'w MoveSpeed,
    stamina: &'w Stamina,
    stat_points: &'w StatPoints,
    equipment: &'w Equipment,
    skill_list: &'w SkillList,
    status_effects: &'w StatusEffects,
    command: &'w Command,
}

fn calculate_reward_percentage(
    game_data: &Res<GameData>,
    equation: &QsdEquation,
    ability_values: &AbilityValues,
    character_info: &CharacterInfo,
) -> f32 {
    let calculator = &game_data.ability_value_calculator;
    let base = calculator.calculate_reward_value(
        equation,
        100,
        0,
        ability_values.level,
        BasicStats::default().charm,
        character_info.fame as i32,
        100,
    );

    let adjusted = calculator.calculate_reward_value(
        equation,
        100,
        0,
        ability_values.level,
        ability_values.get_charm(),
        character_info.fame as i32,
        100,
    );

    let divisor = adjusted as f32 / 100.0;
    (100.0 - (base as f32 / divisor)) + 100.0
}

fn get_exp_reward_rate(
    game_data: &Res<GameData>,
    ability_values: &AbilityValues,
    character_info: &CharacterInfo,
) -> (f32, f32) {
    let exp_leveled = calculate_reward_percentage(
        game_data,
        &QsdEquation::ExpLeveled,
        ability_values,
        character_info,
    );

    let exp_unleveled = calculate_reward_percentage(
        game_data,
        &QsdEquation::ExpUnleveled,
        ability_values,
        character_info,
    );

    if exp_leveled > exp_unleveled {
        (exp_unleveled, exp_leveled)
    } else {
        (exp_leveled, exp_unleveled)
    }
}

fn get_money_reward_rate(
    game_data: &Res<GameData>,
    ability_values: &AbilityValues,
    character_info: &CharacterInfo,
) -> f32 {
    calculate_reward_percentage(
        game_data,
        &QsdEquation::MoneyScaled,
        ability_values,
        character_info,
    )
}

fn get_item_reward_rate(
    game_data: &Res<GameData>,
    ability_values: &AbilityValues,
    character_info: &CharacterInfo,
) -> f32 {
    calculate_reward_percentage(
        game_data,
        &QsdEquation::Item,
        ability_values,
        character_info,
    )
}

fn get_drop_gem_rate(ability_values: &AbilityValues) -> f32 {
    let mut ability_values = ability_values.clone();
    let charm = ability_values.charm.clone();

    ability_values.charm = BasicStats::default().charm;
    // Assume monster is same lv as player
    let base = ability_values.get_drop_gem(ability_values.level, 100, Some(50));

    ability_values.charm = charm;
    let adjusted = ability_values.get_drop_gem(ability_values.level, 100, Some(50));

    let divisor = adjusted as f32 / 100.0;
    (100.0 - (base as f32 / divisor)) + 100.0
}

fn get_drop_grade_rate(ability_values: &AbilityValues) -> f32 {
    let mut ability_values = ability_values.clone();
    let charm = ability_values.charm.clone();

    ability_values.charm = BasicStats::default().charm;
    // Assume monster is same lv as player
    let base = ability_values.get_drop_grade(ability_values.level, 100, Some(50));

    ability_values.charm = charm;
    let adjusted = ability_values.get_drop_grade(ability_values.level, 100, Some(50));

    let divisor = adjusted / 100.0;
    (100.0 - (base / divisor)) + 100.0
}

fn get_simulated_stat<T: ToString + PartialEq>(simulated_value: T, value: T) -> (String, Color32) {
    let (raw_value, color) = get_simulated_stat_raw(simulated_value, value);
    (raw_value.to_string(), color)
}

fn get_simulated_stat_raw<T: PartialEq>(simulated_value: T, value: T) -> (T, Color32) {
    let color = if simulated_value != value {
        POSITIVE_EFFECT_COLOR
    } else {
        Color32::WHITE
    };

    (simulated_value, color)
}

pub fn ui_character_info_system(
    mut egui_context: EguiContexts,
    query_player: Query<PlayerQuery, With<PlayerCharacter>>,
    mut ui_state: Local<UiStateCharacterInfo>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    mut ui_sound_events: EventWriter<UiSoundEvent>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
) {
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_character_info) {
        dialog
    } else {
        return;
    };

    let player = if let Ok(player) = query_player.get_single() {
        player
    } else {
        return;
    };

    let ui_state = &mut *ui_state;
    let mut response_close_button = None;
    let mut response_raise_str_button = None;
    let mut response_raise_dex_button = None;
    let mut response_raise_int_button = None;
    let mut response_raise_con_button = None;
    let mut response_raise_cha_button = None;
    let mut response_raise_sen_button = None;

    Window::new("Character Info")
        .frame(Frame::none())
        .open(&mut ui_state_windows.character_info_open)
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .show(egui_context.ctx_mut(), |ui| {
            let modifiers = ui.input(|ui| ui.modifiers);
            if modifiers.alt {
                ui_state.simulate_amount = 100;
            } else if modifiers.shift {
                ui_state.simulate_amount = 10;
            } else {
                ui_state.simulate_amount = 1;
            }

            let need_xp = game_data
                .ability_value_calculator
                .calculate_levelup_require_xp(player.level.level);
            let stamina = player.stamina.stamina as f32 / MAX_STAMINA as f32;

            dialog.draw(
                ui,
                DataBindings {
                    sound_events: Some(&mut ui_sound_events),
                    response: &mut [
                        (IID_BTN_CLOSE, &mut response_close_button),
                        (IID_BTN_UP_STR, &mut response_raise_str_button),
                        (IID_BTN_UP_DEX, &mut response_raise_dex_button),
                        (IID_BTN_UP_INT, &mut response_raise_int_button),
                        (IID_BTN_UP_CON, &mut response_raise_con_button),
                        (IID_BTN_UP_CHARM, &mut response_raise_cha_button),
                        (IID_BTN_UP_SENSE, &mut response_raise_sen_button),
                    ],
                    gauge: &mut [(
                        IID_GUAGE_STAMINA,
                        &stamina,
                        &format!("{} / {}", player.stamina.stamina, MAX_STAMINA),
                    )],
                    tabs: &mut [(IID_TABBEDPANE, &mut ui_state.current_tab)],
                    ..Default::default()
                },
                |ui, bindings| match bindings.get_tab(IID_TABBEDPANE) {
                    Some(&mut IID_TAB_BASICINFO) => {
                        ui.add_label_at(pos2(59.0, 67.0), &player.character_info.name);
                        ui.add_label_at(
                            pos2(59.0, 88.0),
                            game_data
                                .string_database
                                .get_job_name(player.character_info.job),
                        );
                        // ui.add_label_at(pos2(59.0, 109.0), ""); // TODO: Clan name
                        ui.add_label_at(pos2(59.0, 172.0), &format!("{}", player.level.level));
                        ui.add_label_at(
                            pos2(59.0, 193.0),
                            &format!("{} / {}", player.experience_points.xp, need_xp),
                        );
                    }
                    Some(&mut IID_TAB_ABILITY) => {
                        let ability_values = ui_state
                            .simulated_ability_values
                            .as_ref()
                            .unwrap_or(player.ability_values);

                        let (value, color) = get_simulated_stat(
                            ability_values.get_strength(),
                            player.ability_values.get_strength(),
                        );
                        ui.add_label_at(pos2(58.0, 67.0), RichText::from(value).color(color));

                        let (value, color) = get_simulated_stat(
                            ability_values.get_dexterity(),
                            player.ability_values.get_dexterity(),
                        );
                        ui.add_label_at(pos2(58.0, 88.0), RichText::from(value).color(color));

                        let (value, color) = get_simulated_stat(
                            ability_values.get_intelligence(),
                            player.ability_values.get_intelligence(),
                        );

                        ui.add_label_at(pos2(58.0, 109.0), RichText::from(value).color(color))
                            .on_hover_ui_at_pointer(|ui| {
                                Grid::new("intelligence_info")
                                    .num_columns(2)
                                    .show(ui, |ui| {
                                        ui.label("Increases buff power");
                                        ui.end_row();
                                        ui.label("Increases magical skill power");
                                        ui.end_row();
                                    });
                            });

                        let (value, color) = get_simulated_stat(
                            ability_values.get_concentration(),
                            player.ability_values.get_concentration(),
                        );
                        ui.add_label_at(pos2(58.0, 130.0), RichText::from(value).color(color));

                        let (value, color) = get_simulated_stat(
                            ability_values.get_charm(),
                            player.ability_values.get_charm(),
                        );
                        ui.add_label_at(pos2(58.0, 151.0), RichText::from(value).color(color));

                        let (value, color) = get_simulated_stat(
                            ability_values.get_sense(),
                            player.ability_values.get_sense(),
                        );

                        ui.add_label_at(pos2(58.0, 172.0), RichText::from(value).color(color))
                            .on_hover_ui_at_pointer(|ui| {
                                Grid::new("sense_info").num_columns(2).show(ui, |ui| {
                                    ui.label("Increases skill power");
                                    ui.end_row();
                                    ui.end_row();
                                });
                            });

                        ui.add_label_at(
                            pos2(69.0, 211.0),
                            &format!("{}", player.stat_points.points),
                        );

                        let (value, color) = get_simulated_stat(
                            ability_values.get_attack_power(),
                            player.ability_values.get_attack_power(),
                        );
                        ui.add_label_at(pos2(171.0, 67.0), RichText::from(value).color(color));

                        let (value, color) = get_simulated_stat(
                            ability_values.get_defence(),
                            player.ability_values.get_defence(),
                        );
                        ui.add_label_at(pos2(171.0, 88.0), RichText::from(value).color(color));

                        let (value, color) = get_simulated_stat(
                            ability_values.get_resistance(),
                            player.ability_values.get_resistance(),
                        );
                        ui.add_label_at(pos2(171.0, 109.0), RichText::from(value).color(color));

                        let (value, color) = get_simulated_stat(
                            ability_values.get_hit(),
                            player.ability_values.get_hit(),
                        );
                        ui.add_label_at(pos2(171.0, 130.0), RichText::from(value).color(color));

                        let (value, color) = get_simulated_stat(
                            ability_values.get_critical(),
                            player.ability_values.get_critical(),
                        );
                        ui.add_label_at(pos2(171.0, 151.0), RichText::from(value).color(color));

                        let (value, color) = get_simulated_stat(
                            ability_values.get_avoid(),
                            player.ability_values.get_avoid(),
                        );
                        ui.add_label_at(pos2(171.0, 172.0), RichText::from(value).color(color));

                        let (value, color) = get_simulated_stat(
                            ability_values.get_attack_speed(),
                            player.ability_values.get_attack_speed(),
                        );
                        ui.add_label_at(pos2(171.0, 193.0), RichText::from(value).color(color));

                        let (value, color) = get_simulated_stat(
                            ability_values.get_move_speed(&MoveMode::Run) as i32,
                            player.ability_values.get_move_speed(&MoveMode::Run) as i32,
                        );
                        ui.add_label_at(pos2(171.0, 214.0), RichText::from(value).color(color));

                        let parent = ui.min_rect();
                        let offset = vec2(parent.width(), 0.0);

                        Window::new("Extra Stats")
                            .fixed_pos(parent.left_top() + offset)
                            .resizable(false)
                            .show(ui.ctx(), |ui| {
                                Grid::new("extra_stats").num_columns(2).show(ui, |ui| {
                                    let (value, color) = get_simulated_stat(
                                        ability_values.get_max_health(),
                                        player.ability_values.get_max_health(),
                                    );

                                    ui.label("Max HP");
                                    ui.colored_label(color, value);
                                    ui.end_row();

                                    let recovery_state = if player.command.is_sit() {
                                        PassiveRecoveryState::Sitting
                                    } else {
                                        PassiveRecoveryState::Normal
                                    };

                                    ui.label("HP Recovery");

                                    let (value, color) = get_simulated_stat(
                                        ability_values.get_health_recovery(recovery_state),
                                        player.ability_values.get_health_recovery(recovery_state),
                                    );
                                    ui.colored_label(color, value);
                                    ui.end_row();

                                    let (value, color) = get_simulated_stat(
                                        ability_values.get_max_mana(),
                                        player.ability_values.get_max_mana(),
                                    );

                                    ui.label("Max MP");
                                    ui.colored_label(color, value);
                                    ui.end_row();

                                    let (value, color) = get_simulated_stat(
                                        ability_values.get_mana_recovery(recovery_state),
                                        player.ability_values.get_mana_recovery(recovery_state),
                                    );

                                    ui.label("MP Recovery");
                                    ui.colored_label(color, value);
                                    ui.end_row();

                                    let (value, color) = get_simulated_stat(
                                        ability_values.max_weight(),
                                        player.ability_values.max_weight(),
                                    );

                                    ui.label("Capacity");
                                    ui.colored_label(color, value);
                                    ui.end_row();

                                    let (value, color) = get_simulated_stat_raw(
                                        get_drop_grade_rate(ability_values),
                                        get_drop_grade_rate(player.ability_values),
                                    );

                                    ui.label("Drop Grade");
                                    ui.colored_label(color, format!("{:.2}%", value));
                                    ui.end_row();

                                    let (value, color) = get_simulated_stat_raw(
                                        get_drop_gem_rate(ability_values),
                                        get_drop_gem_rate(player.ability_values),
                                    );

                                    ui.label("Drop Bonus Stats");
                                    ui.colored_label(color, format!("{:.2}%", value));
                                    ui.end_row();

                                    let (value, color) = get_simulated_stat_raw(
                                        get_money_reward_rate(
                                            &game_data,
                                            ability_values,
                                            player.character_info,
                                        ),
                                        get_money_reward_rate(
                                            &game_data,
                                            player.ability_values,
                                            player.character_info,
                                        ),
                                    );

                                    ui.label("Quest Zuly");
                                    ui.colored_label(color, format!("{:.2}%", value));
                                    ui.end_row();

                                    let (value, color) = get_simulated_stat_raw(
                                        get_item_reward_rate(
                                            &game_data,
                                            ability_values,
                                            player.character_info,
                                        ),
                                        get_item_reward_rate(
                                            &game_data,
                                            player.ability_values,
                                            player.character_info,
                                        ),
                                    );

                                    ui.label("Quest Items");
                                    ui.colored_label(color, format!("{:.2}%", value));
                                    ui.end_row();

                                    let (value, color) = get_simulated_stat_raw(
                                        get_exp_reward_rate(
                                            &game_data,
                                            ability_values,
                                            player.character_info,
                                        ),
                                        get_exp_reward_rate(
                                            &game_data,
                                            player.ability_values,
                                            player.character_info,
                                        ),
                                    );

                                    ui.label("Quest EXP");
                                    ui.colored_label(
                                        color,
                                        format!("{:.2}% - {:.2}%", value.0, value.1,),
                                    );
                                    ui.end_row();
                                });
                            });
                    }
                    Some(&mut IID_TAB_UNION) => {}
                    _ => {}
                },
            );
        });

    if response_close_button.map_or(false, |r| r.clicked()) {
        ui_state_windows.character_info_open = false;
    }

    let mut some_hovered = false;
    let mut stat_button_response = |basic_stat_type, response: Option<Response>| {
        let Some(response) = response else {
            return;
        };

        if response.hovered() {
            some_hovered = true;

            let mut basic_stats = player.basic_stats.clone();
            basic_stats.set(
                basic_stat_type,
                basic_stats.get(basic_stat_type) + ui_state.simulate_amount,
            );

            ui_state.simulated_ability_values = Some(game_data.ability_value_calculator.calculate(
                player.character_info,
                player.level,
                player.equipment,
                &basic_stats,
                player.skill_list,
                player.status_effects,
            ));
        }

        let Some(cost) = game_data
            .ability_value_calculator
            .calculate_basic_stat_increase_cost(player.basic_stats, basic_stat_type)
        else {
            return;
        };

        if response.hovered() {
            let position = response.rect.left_top() - pos2(242.0, 0.0);
            egui::show_tooltip_at(
                &response.ctx,
                egui::Id::from("points"),
                Some(position.to_pos2()),
                |ui| {
                    ui.label(format!(
                        "Required Points: {}\nShift: Simulate 10 points\nAlt: Simulate 100 points",
                        cost
                    ));
                },
            );
        }

        if !response.clicked() || cost > player.stat_points.points {
            return;
        }

        let Some(game_connection) = game_connection.as_ref() else {
            return;
        };

        game_connection
            .client_message_tx
            .send(ClientMessage::IncreaseBasicStat { basic_stat_type })
            .ok();
    };

    stat_button_response(BasicStatType::Strength, response_raise_str_button);
    stat_button_response(BasicStatType::Dexterity, response_raise_dex_button);
    stat_button_response(BasicStatType::Intelligence, response_raise_int_button);
    stat_button_response(BasicStatType::Concentration, response_raise_con_button);
    stat_button_response(BasicStatType::Charm, response_raise_cha_button);
    stat_button_response(BasicStatType::Sense, response_raise_sen_button);

    if !some_hovered {
        ui_state.simulated_ability_values = None;
    }
}
