use std::path::Path;

use super::mosaic::MosaicTheme;
use crate::patch::{Rom, apply_ips_patch, bps::BPSPatch, get_room_state_ptrs, snes2pc};
use anyhow::{Context, Result};
use hashbrown::HashMap;
use maprando_game::{DoorPtr, GameData, Map, RoomPtr, RoomStateIdx};
use rand::{Rng, SeedableRng};

use super::TileTheme;

const BPS_PATCH_PATH: &str = "../patches/mosaic";

fn apply_bps_patch(rom: &mut Rom, orig_rom: &Rom, filename: &str) -> Result<()> {
    let path = Path::new(BPS_PATCH_PATH).join(filename);
    let patch_bytes = std::fs::read(path).with_context(|| format!("Loading {filename}"))?;
    let patch = BPSPatch::new(patch_bytes)?;
    patch.apply(&orig_rom.data, &mut rom.data);
    Ok(())
}

fn apply_toilet(rom: &mut Rom, orig_rom: &Rom, theme_name: &str) -> Result<()> {
    let toilet_intersecting_room_ptr_addr = snes2pc(0xB5FE70);
    let toilet_rel_x_addr = snes2pc(0xB5FE72);
    let toilet_rel_y_addr = snes2pc(0xB5FE73);

    let room_ptr = rom.read_u16(toilet_intersecting_room_ptr_addr)? + 0x70000;
    let patch_filename = if room_ptr == 0x7FFFF {
        // Unspecified room means this is vanilla map, so leave the Toilet alone.
        format!("{theme_name}-VanillaMapTransit.bps")
    } else {
        let x = rom.read_u8(toilet_rel_x_addr)? as i8 as isize;
        let y = rom.read_u8(toilet_rel_y_addr)? as i8 as isize;
        format!("{theme_name}-{room_ptr:X}-Transit-{x}-{y}.bps")
    };
    println!("toilet patch: {patch_filename}");
    apply_bps_patch(rom, orig_rom, &patch_filename)
        .context(format!("Applying Toilet patch: {patch_filename}"))?;

    Ok(())
}

pub fn apply_retiling(
    rom: &mut Rom,
    orig_rom: &Rom,
    map: &Map,
    game_data: &GameData,
    theme: &TileTheme,
    mosaic_themes: &[MosaicTheme],
) -> Result<()> {
    let patch_names = vec![
        "Scrolling Sky v1.6",
        "Area FX",
        "Area Palettes",
        "Area Palette Glows",
        "Bowling",
        "Item Loading",
        "Fake Lava",
        "in_place_level_data",
    ];
    for name in &patch_names {
        let patch_path_str = format!("../patches/ips/{name}.ips");
        apply_ips_patch(rom, Path::new(&patch_path_str))?;
    }

    let mut fx_door_ptr_map: HashMap<(RoomPtr, RoomStateIdx, DoorPtr), DoorPtr> = HashMap::new();
    for &room_ptr in game_data.raw_room_id_by_ptr.keys() {
        let state_ptrs = get_room_state_ptrs(rom, room_ptr)?;
        for (state_idx, (_event_ptr, state_ptr)) in state_ptrs.iter().enumerate() {
            let orig_fx_ptr = orig_rom.read_u16(state_ptr + 6)? as usize;
            let fx_ptr = rom.read_u16(state_ptr + 6)? as usize;
            assert_eq!(orig_fx_ptr, fx_ptr);
            for i in 0..4 {
                let orig_door_ptr =
                    orig_rom.read_u16(snes2pc(0x830000 + fx_ptr + i * 16))? as DoorPtr;
                let door_ptr = rom.read_u16(snes2pc(0x830000 + fx_ptr + i * 16))? as DoorPtr;
                if orig_door_ptr == 0 || orig_door_ptr == 0xFFFF {
                    break;
                }
                fx_door_ptr_map.insert((room_ptr, state_idx, orig_door_ptr), door_ptr);
            }
        }
    }

    let random_seed = u32::from_le_bytes(rom.read_n(snes2pc(0xdfff00), 4)?.try_into()?);

    apply_bps_patch(rom, orig_rom, "tilesets.bps")?;

    let mut theme_name_map: HashMap<RoomPtr, String> = HashMap::new();
    for (room_idx, room) in game_data.room_geometry.iter().enumerate() {
        let room_ptr = room.rom_address;
        let theme_name = match theme {
            TileTheme::Vanilla => "Base".to_string(),
            TileTheme::Constant(s) => s.clone(),
            TileTheme::AreaThemed => {
                let area = map.area[room_idx];
                let sub_area = map.subarea[room_idx];
                let sub_sub_area = if !map.subsubarea.is_empty() {
                    map.subsubarea[room_idx]
                } else {
                    // For backward compatibility, use subsubarea 0 for old maps that didn't have a subsubarea.
                    0
                };
                match (area, sub_area, sub_sub_area) {
                    (0, 0, _) => "OuterCrateria",
                    (0, 1, 0) => "InnerCrateria",
                    (0, 1, 1) => "BlueBrinstar",
                    (1, 0, 0) => "GreenBrinstar",
                    (1, 0, 1) => "PinkBrinstar",
                    (1, 1, _) => "RedBrinstar",
                    (2, 0, _) => "UpperNorfair",
                    (2, 1, _) => "LowerNorfair",
                    (3, _, _) => "WreckedShip",
                    (4, 0, _) => "WestMaridia",
                    (4, 1, _) => "YellowMaridia",
                    (5, 0, _) => "MetroidHabitat",
                    (5, 1, _) => "MechaTourian",
                    _ => panic!(
                        "unexpected area/subarea/subsubarea combination: ({area}, {sub_area}, {sub_sub_area})"
                    ),
                }
                .to_string()
            }
            TileTheme::Scrambled => {
                let seed = random_seed ^ (room_ptr as u32);
                let mut rng_seed = [0u8; 32];
                rng_seed[..4].copy_from_slice(&seed.to_le_bytes());
                let mut rng = rand::rngs::StdRng::from_seed(rng_seed);
                let theme_idx = rng.gen_range(0..mosaic_themes.len());
                mosaic_themes[theme_idx].name.clone()
            }
        };
        theme_name_map.insert(room_ptr, theme_name);
    }

    if *theme == TileTheme::AreaThemed {
        apply_ips_patch(rom, Path::new("../patches/ips/mosaic_fx_fix.ips"))?;
    }

    if *theme != TileTheme::Vanilla {
        // Enable tileset swap based on map area
        rom.write_u16(snes2pc(0x8AB500), 0xF0F0)?;
    }

    // Enable handling of always-on NMI (for scrolling sky)
    rom.write_u16(snes2pc(0x8AB180), 0xFFFF)?;

    // Make the Toilet's intersecting room use the same tile theme as the Toilet.
    // Likewise for East Pants Room and Homing Geemer Room.
    // This only matters in case of Scrambled tile theme, since otherwise this should already be true.
    let toilet_room_ptr = 0x7D408;
    let toilet_intersecting_room_ptr_addr = snes2pc(0xB5FE70);
    let toilet_intersection_room_ptr =
        (rom.read_u16(toilet_intersecting_room_ptr_addr)? + 0x70000) as usize;
    if toilet_intersection_room_ptr == 0x7FFFF {
        // Unspecified room means this is vanilla map, so the Toilet intersects Aqueduct and Botwoon Hallway.
        theme_name_map.insert(0x7D5A7, theme_name_map[&toilet_room_ptr].clone()); // Aqueduct
        theme_name_map.insert(0x7D617, theme_name_map[&toilet_room_ptr].clone());
    // Botwoon Hallway
    } else {
        theme_name_map.insert(
            toilet_intersection_room_ptr,
            theme_name_map[&toilet_room_ptr].clone(),
        );
    }
    theme_name_map.insert(0x7D69A, theme_name_map[&0x7D646].clone()); // East Pants Room
    theme_name_map.insert(0x7968F, theme_name_map[&0x793FE].clone()); // Homing Geemer Room

    for &room_ptr in game_data.raw_room_id_by_ptr.keys() {
        let theme_name = &theme_name_map[&room_ptr];
        let state_ptrs = get_room_state_ptrs(rom, room_ptr)?;
        for (state_idx, (_event_ptr, state_ptr)) in state_ptrs.iter().enumerate() {
            let patch_filename = format!("{theme_name}-{room_ptr:X}-{state_idx}.bps");
            apply_bps_patch(rom, orig_rom, &patch_filename)?;

            let fx_ptr = rom.read_u16(state_ptr + 6)? as usize;
            for i in 0..4 {
                let door_ptr_addr = snes2pc(0x830000 + fx_ptr + i * 16);
                let door_ptr = rom.read_u16(door_ptr_addr)? as DoorPtr;
                if door_ptr == 0 || door_ptr == 0xffff {
                    break;
                }
                let new_door_ptr = fx_door_ptr_map[&(room_ptr, state_idx, door_ptr)];
                rom.write_u16(door_ptr_addr, new_door_ptr as isize)?;
            }
        }
    }

    apply_toilet(rom, orig_rom, &theme_name_map[&toilet_room_ptr])?;

    Ok(())
}
