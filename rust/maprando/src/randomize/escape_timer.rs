use anyhow::{Context, Result};
use hashbrown::HashMap;
use maprando_game::TECH_ID_CAN_HERO_SHOT;
use maprando_game::TECH_ID_CAN_HYPER_GATE_SHOT;
use maprando_game::TECH_ID_CAN_KAGO;
use maprando_game::TECH_ID_CAN_MID_AIR_MORPH;
use maprando_game::TECH_ID_CAN_MOONFALL;
use maprando_game::TECH_ID_CAN_OFF_SCREEN_SUPER_SHOT;
use maprando_game::TECH_ID_CAN_SUITLESS_LAVA_DIVE;
use maprando_game::TechId;
use pathfinding;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::settings::RandomizerSettings;
use maprando_game::{
    DoorPtrPair, EscapeConditionRequirement, GameData, IndexedVec, Map, RoomGeometryDoorIdx,
    RoomGeometryRoomIdx,
};

use super::DifficultyConfig;
use super::MotherBrainFight;

pub type RoomName = &'static str;
pub type VertexId = usize;
pub type Cost = usize;
pub type VertexKey = (RoomGeometryRoomIdx, RoomGeometryDoorIdx);

#[derive(Clone, Default)]
pub struct RoomDoorGraph {
    pub vertices: IndexedVec<VertexKey>,
    pub successors: Vec<Vec<(VertexId, Cost)>>,
    pub mother_brain_vertex_id: VertexId,
    pub ship_vertex_id: VertexId,
    pub animals_vertex_id: VertexId,
}

// Ideally this would contain coords, but whatever
#[derive(Serialize, Deserialize)]
pub struct SpoilerEscapeRouteNode {
    room: String,
    node: String,
    pub x: usize,
    pub y: usize,
}

#[derive(Serialize, Deserialize)]
pub struct SpoilerEscapeRouteEntry {
    pub from: SpoilerEscapeRouteNode,
    pub to: SpoilerEscapeRouteNode,
    pub base_igt_frames: usize,
}

#[derive(Serialize, Deserialize)]
pub struct SpoilerEscape {
    pub base_igt_frames: usize,
    pub base_igt_seconds: f32,
    pub base_leniency_factor: f32,
    pub difficulty_multiplier: f32,
    pub raw_time_seconds: f32,
    pub final_time_seconds: f32,
    pub animals_route: Option<Vec<SpoilerEscapeRouteEntry>>,
    pub ship_route: Vec<SpoilerEscapeRouteEntry>,
}

// In game times are recorded in the format X.YY where X is integer seconds and YY is frames
// (as in the Practice Hack). This function converts this to frames.
fn parse_in_game_time(raw_time: f32) -> usize {
    let int_part = raw_time.floor() as usize;
    let frac_part = raw_time.fract();
    let frames = (frac_part * 100.0).round() as usize;
    assert!(frames < 60);
    int_part * 60 + frames
}

fn is_requirement_satisfied(
    req: EscapeConditionRequirement,
    settings: &RandomizerSettings,
    difficulty: &DifficultyConfig,
    game_data: &GameData,
) -> bool {
    let has_tech = |tech_id: TechId| {
        let tech_idx = game_data.tech_isv.index_by_key[&tech_id];
        difficulty.tech[tech_idx]
    };
    match req {
        EscapeConditionRequirement::EnemiesCleared => {
            settings.quality_of_life_settings.escape_enemies_cleared
        }
        EscapeConditionRequirement::CanMidAirMorph => has_tech(TECH_ID_CAN_MID_AIR_MORPH),
        EscapeConditionRequirement::CanUsePowerBombs => {
            settings.quality_of_life_settings.mother_brain_fight == MotherBrainFight::Skip
        }
        EscapeConditionRequirement::CanAcidDive => has_tech(TECH_ID_CAN_SUITLESS_LAVA_DIVE),
        EscapeConditionRequirement::CanKago => has_tech(TECH_ID_CAN_KAGO),
        EscapeConditionRequirement::CanMoonfall => has_tech(TECH_ID_CAN_MOONFALL),
        EscapeConditionRequirement::CanOffCameraShot => has_tech(TECH_ID_CAN_OFF_SCREEN_SUPER_SHOT),
        EscapeConditionRequirement::CanReverseGate => has_tech(TECH_ID_CAN_HYPER_GATE_SHOT),
        EscapeConditionRequirement::CanHeroShot => has_tech(TECH_ID_CAN_HERO_SHOT),
        EscapeConditionRequirement::CanOneTapShortcharge => difficulty.shine_charge_tiles <= 25.0,
    }
}

pub fn get_base_room_door_graph(
    game_data: &GameData,
    settings: &RandomizerSettings,
    difficulty: &DifficultyConfig,
) -> RoomDoorGraph {
    let mut vertices: IndexedVec<VertexKey> = IndexedVec::default();
    let mut successors: Vec<Vec<(VertexId, Cost)>> = Vec::new();
    let mut ship_vertex_id = VertexId::MAX;
    let mut animals_vertex_id = VertexId::MAX;
    let mut mother_brain_vertex_id = VertexId::MAX;

    for escape_timing_room in &game_data.escape_timings {
        let room_id = escape_timing_room.room_id;
        for escape_timing_group in &escape_timing_room.timings {
            let from_key = (room_id, escape_timing_group.from_door.door_idx);
            let from_idx = vertices.add(&from_key);
            successors.push(vec![]);
            if escape_timing_room.room_id == 8 && escape_timing_group.from_door.name == "Ship" {
                // "Landing Site"
                ship_vertex_id = from_idx;
            }
            if escape_timing_room.room_id == 19 {
                // "Bomb Torizo Room"
                animals_vertex_id = from_idx;
            }
            if escape_timing_room.room_id == 238
                && escape_timing_group.from_door.direction == "left"
            {
                // "Mother Brain Room"
                mother_brain_vertex_id = from_idx;
            }
        }
        for escape_timing_group in &escape_timing_room.timings {
            let from_key = (room_id, escape_timing_group.from_door.door_idx);
            let from_idx = vertices.index_by_key[&from_key];
            for escape_timing in &escape_timing_group.to {
                let to_key = (room_id, escape_timing.to_door.door_idx);
                let to_idx = vertices.index_by_key[&to_key];
                if let Some(in_game_time) = escape_timing.in_game_time {
                    let cost = parse_in_game_time(in_game_time);
                    successors[from_idx].push((to_idx, cost));
                }
                for condition in &escape_timing.conditions {
                    if condition
                        .requires
                        .iter()
                        .all(|&req| is_requirement_satisfied(req, settings, difficulty, game_data))
                    {
                        let cost = parse_in_game_time(condition.in_game_time);
                        successors[from_idx].push((to_idx, cost));
                    }
                }
            }
        }
    }

    RoomDoorGraph {
        vertices,
        successors,
        mother_brain_vertex_id,
        ship_vertex_id,
        animals_vertex_id,
    }
}

pub fn get_full_room_door_graph(
    game_data: &GameData,
    map: &Map,
    settings: &RandomizerSettings,
    difficulty: &DifficultyConfig,
) -> RoomDoorGraph {
    let base = get_base_room_door_graph(game_data, settings, difficulty);
    let mut door_ptr_pair_to_vertex: HashMap<DoorPtrPair, VertexId> = HashMap::new();
    for room in &game_data.room_geometry {
        let room_id = room.room_id;
        for (door_idx, door) in room.doors.iter().enumerate() {
            let vertex_id = *base
                .vertices
                .index_by_key
                .get(&(room_id, door_idx))
                .context(format!(
                    "base.vertices.index_by_key missing entry: ({room_id}, {door_idx})"
                ))
                .unwrap();
            let door_ptr_pair = (door.exit_ptr, door.entrance_ptr);
            door_ptr_pair_to_vertex.insert(door_ptr_pair, vertex_id);
        }
    }
    let mut room_door_graph = base.clone();
    for &(src_door_ptr_pair, dst_door_ptr_pair, bidirectional) in &map.doors {
        let src_vertex_id = door_ptr_pair_to_vertex[&src_door_ptr_pair];
        let dst_vertex_id = door_ptr_pair_to_vertex[&dst_door_ptr_pair];
        room_door_graph.successors[src_vertex_id].push((dst_vertex_id, 0));
        if bidirectional {
            room_door_graph.successors[dst_vertex_id].push((src_vertex_id, 0));
        }
    }
    room_door_graph
}

fn get_vertex_name(
    vertex_id: VertexId,
    room_door_graph: &RoomDoorGraph,
    game_data: &GameData,
    map: &Map,
) -> SpoilerEscapeRouteNode {
    let (room_id, door_idx) = room_door_graph.vertices.keys[vertex_id];
    let room_ptr = game_data.room_ptr_by_id[&room_id];
    let room_idx = game_data.room_idx_by_ptr[&room_ptr];
    let room = &game_data.room_geometry[room_idx];
    if vertex_id == room_door_graph.ship_vertex_id {
        return SpoilerEscapeRouteNode {
            room: "Landing Site".to_string(),
            node: "Ship".to_string(),
            x: map.rooms[room_idx].0 + 4,
            y: map.rooms[room_idx].1 + 4,
        };
    }
    let door = &room.doors[door_idx];
    let door_ptr_pair = (door.exit_ptr, door.entrance_ptr);
    let (room_id, door_id) = game_data.door_ptr_pair_map[&door_ptr_pair];
    let x = map.rooms[room_idx].0 + door.x;
    let y = map.rooms[room_idx].1 + door.y;
    let room_json = &game_data.room_json_map[&room_id];
    let node_json = &game_data.node_json_map[&(room_id, door_id)];
    SpoilerEscapeRouteNode {
        room: room_json["name"].as_str().unwrap().to_string(),
        node: node_json["name"].as_str().unwrap().to_string(),
        x,
        y,
    }
}

fn get_spoiler_escape_route(
    path: &[(VertexId, Cost)],
    room_door_graph: &RoomDoorGraph,
    game_data: &GameData,
    map: &Map,
) -> Vec<SpoilerEscapeRouteEntry> {
    let mut out: Vec<SpoilerEscapeRouteEntry> = Vec::new();
    for slice in path.windows(2) {
        if let &[(src_vertex_id, src_cost), (dst_vertex_id, dst_cost)] = slice {
            if src_cost == dst_cost {
                continue;
            }
            out.push(SpoilerEscapeRouteEntry {
                from: get_vertex_name(src_vertex_id, room_door_graph, game_data, map),
                to: get_vertex_name(dst_vertex_id, room_door_graph, game_data, map),
                base_igt_frames: dst_cost,
            });
        } else {
            panic!("Internal error");
        }
    }
    out
}

fn get_shortest_path(
    src: VertexId,
    dst: VertexId,
    room_door_graph: &RoomDoorGraph,
) -> Result<Vec<(VertexId, Cost)>> {
    let successors = |&src: &VertexId| room_door_graph.successors[src].iter().copied();
    let parents: std::collections::HashMap<VertexId, (VertexId, Cost)> =
        pathfinding::directed::dijkstra::dijkstra_all(&src, successors);
    let mut path: Vec<(VertexId, Cost)> = Vec::new();
    let mut v = dst;
    while v != src {
        let (new_v, cost) = *parents.get(&v).context("No escape path")?;
        path.push((v, cost));
        v = new_v;
    }
    path.reverse();
    Ok(path)
}

pub fn compute_escape_data(
    game_data: &GameData,
    map: &Map,
    settings: &RandomizerSettings,
    save_animals: bool,
    difficulty: &DifficultyConfig,
) -> Result<SpoilerEscape> {
    let graph = get_full_room_door_graph(game_data, map, settings, difficulty);
    let animals_spoiler: Option<Vec<SpoilerEscapeRouteEntry>>;
    let ship_spoiler: Vec<SpoilerEscapeRouteEntry>;
    let base_igt_frames: usize;
    if save_animals {
        let animals_path = get_shortest_path(
            graph.mother_brain_vertex_id,
            graph.animals_vertex_id,
            &graph,
        )?;
        animals_spoiler = Some(get_spoiler_escape_route(
            &animals_path,
            &graph,
            game_data,
            map,
        ));
        let ship_path = get_shortest_path(graph.animals_vertex_id, graph.ship_vertex_id, &graph)?;
        ship_spoiler = get_spoiler_escape_route(&ship_path, &graph, game_data, map);
        base_igt_frames = animals_path.last().unwrap().1 + ship_path.last().unwrap().1;
    } else {
        animals_spoiler = None;
        let ship_path =
            get_shortest_path(graph.mother_brain_vertex_id, graph.ship_vertex_id, &graph)?;
        ship_spoiler = get_spoiler_escape_route(&ship_path, &graph, game_data, map);
        base_igt_frames = ship_path.last().unwrap().1;
    }

    let base_igt_seconds: f32 = (base_igt_frames as f32) / 60.0;
    let base_leniency_factor: f32 = 1.0;
    let raw_time_seconds =
        base_igt_seconds * base_leniency_factor * difficulty.escape_timer_multiplier;

    let mut final_time_seconds = if difficulty.escape_timer_multiplier <= 1.0 {
        f32::ceil(raw_time_seconds)
    } else {
        f32::ceil(raw_time_seconds / 5.0) * 5.0
    };
    if final_time_seconds > 5995.0 {
        final_time_seconds = 5995.0;
    }
    println!("escape time: {final_time_seconds}");

    Ok(SpoilerEscape {
        base_igt_frames,
        base_igt_seconds,
        base_leniency_factor,
        difficulty_multiplier: difficulty.escape_timer_multiplier,
        raw_time_seconds,
        final_time_seconds,
        animals_route: animals_spoiler,
        ship_route: ship_spoiler,
    })
}
