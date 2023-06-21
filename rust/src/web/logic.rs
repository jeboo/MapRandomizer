use glob::glob;
use hashbrown::{HashMap, HashSet};
use json::JsonValue;
use sailfish::TemplateOnce;

use crate::game_data::{GameData, NodeId, Requirement, RoomId};
use crate::web::VERSION;

#[derive(Clone)]
struct RoomStrat {
    room_name: String,
    room_name_stripped: String,
    strat_name: String,
    notable: bool,
    from_node_id: usize,
    from_node_name: String,
    to_node_id: usize,
    to_node_name: String,
    note: String,
    requires: String,                         // new-line separated requirements
    obstacles: Vec<(String, String, String)>, // list of (obstacle name, obstacle requires, additional obstacles)
}

#[derive(TemplateOnce, Clone)]
#[template(path = "logic/room.stpl")]
struct RoomTemplate {
    version: usize,
    room_id: usize,
    room_name: String,
    room_name_stripped: String,
    area: String,
    sub_area: String,
    room_diagram_path: String,
    nodes: Vec<(usize, String)>,
    strats: Vec<RoomStrat>,
    room_json: String,
}

#[derive(TemplateOnce, Clone)]
#[template(path = "logic/tech.stpl")]
struct TechTemplate {
    version: usize,
    tech_name: String,
    tech_note: String,
    tech_dependencies: String,
    strats: Vec<RoomStrat>,
    tech_gif_listing: HashSet<String>,
}

#[derive(TemplateOnce)]
#[template(path = "logic/logic.stpl")]
struct LogicIndexTemplate<'a> {
    version: usize,
    rooms: &'a [RoomTemplate],
    tech: &'a [TechTemplate],
}

#[derive(Default)]
pub struct LogicData {
    pub index_html: String,  // Logic index page
    pub room_html: HashMap<String, String>, // Map from room name (alphanumeric characters only) to rendered HTML.
    pub tech_html: HashMap<String, String>, // Map from tech name to rendered HTML.
    pub tech_strat_counts: HashMap<String, usize>, // Map from tech name to strat count using that tech.
}

fn list_room_diagram_files() -> HashMap<usize, String> {
    let mut out: HashMap<usize, String> = HashMap::new();
    for entry in glob("static/sm-json-data/region/*/roomDiagrams/*.png").unwrap() {
        match entry {
            Ok(path) => {
                let path_string = path.to_str().unwrap().to_string();
                let segments: Vec<&str> = path_string.split("_").collect();
                let subregion = segments[0];
                if subregion == "ceres" {
                    continue;
                }
                let room_id: usize = str::parse(segments[1]).unwrap();
                // let img = image::open(path).unwrap();
                // println!("{:?}", img.dimensions());
                out.insert(room_id, path_string);
            }
            Err(e) => panic!("Failure reading room diagrams: {:?}", e),
        }
    }
    out
}

fn make_requires(requires_json: &JsonValue) -> String {
    let mut out: Vec<String> = vec![];
    for req in requires_json.members() {
        out.push(req.pretty(2));
    }
    out.join("\n")
}

fn extract_tech_rec(req: &JsonValue, tech: &mut HashSet<usize>, game_data: &GameData) {
    if req.is_string() {
        let value = req.as_str().unwrap();
        if let Some(idx) = game_data.tech_isv.index_by_key.get(value) {
            // Skipping tech dependencies, so that only techs that explicitly appear in a strat (or via a helper)
            // will show up under the corresponding tech page.
            tech.insert(*idx);
        } else if let Some(helper_json) = game_data.helper_json_map.get(value) {
            for r in helper_json["requires"].members() {
                extract_tech_rec(r, tech, game_data);
            }
        } 
    } else if req.is_object() && req.len() == 1 {
        let (key, value) = req.entries().next().unwrap();
        if key == "and" || key == "or" {
            for x in value.members() {
                extract_tech_rec(x, tech, game_data);
            }
        } else if key == "canShineCharge" && value["shinesparkFrames"].as_i32().unwrap() > 0 {
            tech.insert(game_data.tech_isv.index_by_key["canShinespark"]);
        } else if key == "canComeInCharged" && value["shinesparkFrames"].as_i32().unwrap() > 0 {
            tech.insert(game_data.tech_isv.index_by_key["canShinespark"]);
        } else if key == "comeInWithRMode" {
            tech.insert(game_data.tech_isv.index_by_key["canEnterRMode"]);
        } else if key == "comeInWithGMode" {
            tech.insert(game_data.tech_isv.index_by_key["canEnterGMode"]);
            if value["artificialMorph"].as_bool().unwrap() {
                tech.insert(game_data.tech_isv.index_by_key["canArtificialMorph"]);                    
            }
        }
    }
}

fn make_tech_templates<'a>(
    game_data: &GameData,
    room_templates: &[RoomTemplate],
    tech_gif_listing: &'a HashSet<String>,
) -> Vec<TechTemplate> {
    let mut tech_strat_ids: Vec<HashSet<(RoomId, NodeId, NodeId, String)>> =
        vec![HashSet::new(); game_data.tech_isv.keys.len()];
    for room_json in game_data.room_json_map.values() {
        let room_id = room_json["id"].as_usize().unwrap();
        for link_json in room_json["links"].members() {
            for link_to_json in link_json["to"].members() {
                for strat_json in link_to_json["strats"].members() {
                    let from_node_id = link_json["from"].as_usize().unwrap();
                    let to_node_id = link_to_json["id"].as_usize().unwrap();
                    let strat_name = strat_json["name"].as_str().unwrap().to_string();
                    let ids = (room_id, from_node_id, to_node_id, strat_name);
                    let mut tech_set: HashSet<usize> = HashSet::new();
                    for req in strat_json["requires"].members() {
                        extract_tech_rec(req, &mut tech_set, game_data);
                    }
                    for tech_idx in tech_set {
                        tech_strat_ids[tech_idx].insert(ids.clone());
                    }
                }
            }
        }
    }

    let mut room_strat_map: HashMap<(RoomId, NodeId, NodeId, String), &RoomStrat> = HashMap::new();
    for template in room_templates {
        for strat in &template.strats {
            room_strat_map.insert(
                (
                    template.room_id,
                    strat.from_node_id,
                    strat.to_node_id,
                    strat.strat_name.to_string(),
                ),
                strat,
            );
        }
    }

    let mut tech_templates: Vec<TechTemplate> = vec![];
    for (tech_idx, tech_ids) in tech_strat_ids.iter().enumerate() {
        let tech_name = game_data.tech_isv.keys[tech_idx].clone();
        let tech_note = game_data.tech_description[&tech_name].clone();
        let tech_dependencies = game_data.tech_dependencies[&tech_name].join(", ");
        let mut strats: Vec<RoomStrat> = vec![];
        for strat_ids in tech_ids {
            // Infinitely-spawning farm strats aren't included (TODO: fix that?)
            if room_strat_map.contains_key(strat_ids) {
                strats.push(room_strat_map[strat_ids].clone());
            }
        }
        let template = TechTemplate {
            version: VERSION,
            tech_name: tech_name.clone(),
            tech_note,
            tech_dependencies,
            strats,
            tech_gif_listing: tech_gif_listing.clone(),
        };
        tech_templates.push(template);
    }
    tech_templates
}

fn strip_name(s: &str) -> String {
    s.chars().filter(|x| x.is_ascii_alphanumeric()).collect()
}

fn make_room_template(
    room_json: &JsonValue,
    room_diagram_listing: &HashMap<usize, String>,
    game_data: &GameData,
) -> RoomTemplate {
    let mut room_strats: Vec<RoomStrat> = vec![];
    let room_id = room_json["id"].as_usize().unwrap();
    let room_name = room_json["name"].as_str().unwrap().to_string();
    let room_name_stripped = strip_name(&room_name);
    let mut node_name_map: HashMap<usize, String> = HashMap::new();
    let mut nodes: Vec<(usize, String)> = vec![];
    for node_json in room_json["nodes"].members() {
        let node_id = node_json["id"].as_usize().unwrap();
        let node_name = node_json["name"].as_str().unwrap();
        node_name_map.insert(node_id, node_name.to_string());
        nodes.push((node_id, node_name.to_string()));
    }
    for link_json in room_json["links"].members() {
        for link_to_json in link_json["to"].members() {
            for strat_json in link_to_json["strats"].members() {
                let from_node_id = link_json["from"].as_usize().unwrap();
                let to_node_id = link_to_json["id"].as_usize().unwrap();
                let mut obstacles: Vec<(String, String, String)> = vec![];
                for obstacle_json in strat_json["obstacles"].members() {
                    let obstacle_id = obstacle_json["id"].as_str().unwrap().to_string();
                    let obstacle_requires = make_requires(&obstacle_json["requires"]);
                    let mut additional: Vec<String> = vec![];
                    for x in obstacle_json["additionalObstacles"].members() {
                        additional.push(x.as_str().unwrap().to_string());
                    }
                    obstacles.push((obstacle_id, obstacle_requires, additional.join(", ")));
                }
                let strat = RoomStrat {
                    room_name: room_name.clone(),
                    room_name_stripped: room_name_stripped.clone(),
                    strat_name: strat_json["name"].as_str().unwrap().to_string(),
                    notable: strat_json["notable"].as_bool().unwrap_or(false),
                    from_node_id,
                    from_node_name: node_name_map[&from_node_id].clone(),
                    to_node_id,
                    to_node_name: node_name_map[&to_node_id].clone(),
                    note: game_data.parse_note(&strat_json["note"]).join(" "),
                    requires: make_requires(&strat_json["requires"]),
                    obstacles,
                };
                room_strats.push(strat);
            }
        }
    }
    // let shape = *game_data.room_shape.get(&room_id).unwrap_or(&(1, 1));
    RoomTemplate {
        version: VERSION,
        room_id,
        room_name,
        room_name_stripped,
        area: room_json["area"].as_str().unwrap().to_string(),
        sub_area: room_json["subarea"].as_str().unwrap().to_string(),
        room_diagram_path: room_diagram_listing[&room_id].clone(),
        nodes,
        strats: room_strats,
        room_json: room_json.pretty(2),
    }
}

impl LogicData {
    pub fn new(game_data: &GameData, tech_gif_listing: &HashSet<String>) -> LogicData {
        let mut out = LogicData::default();
        let room_diagram_listing = list_room_diagram_files();
        let mut room_templates: Vec<RoomTemplate> = vec![];
        for (_, room_json) in game_data.room_json_map.iter() {
            let template = make_room_template(room_json, &room_diagram_listing, &game_data);
            let html = template.clone().render_once().unwrap();
            out.room_html
                .insert(strip_name(&template.room_name), html);
            room_templates.push(template);
        }
        room_templates.sort_by_key(|x| (x.area.clone(), x.sub_area.clone(), x.room_name.clone()));

        let tech_templates = make_tech_templates(game_data, &room_templates, tech_gif_listing);
        for template in &tech_templates {
            let html = template.clone().render_once().unwrap();
            out.tech_strat_counts.insert(template.tech_name.clone(), template.strats.len());
            out.tech_html.insert(template.tech_name.clone(), html);
        }

        let index_template = LogicIndexTemplate {
            version: VERSION,
            rooms: &room_templates,
            tech: &tech_templates,
        };
        out.index_html = index_template.render_once().unwrap();
        out
    }
}
