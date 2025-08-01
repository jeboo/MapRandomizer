pub mod mosaic;
pub mod retiling;
pub mod room_palettes;
pub mod samus_sprite;
pub mod vanilla_music;

use anyhow::{Result, bail};
use std::cmp::min;
use std::path::Path;

use crate::patch::glowpatch_writer::write_glowpatch;
use crate::patch::{Rom, apply_ips_patch, snes2pc, write_credits_big_char};
use maprando_game::{GameData, Map};
use mosaic::MosaicTheme;
use retiling::apply_retiling;
use room_palettes::apply_area_themed_palettes;
use samus_sprite::SamusSpriteCategory;

struct AllocatorBlock {
    start_addr: usize,
    end_addr: usize,
    current_addr: usize,
}

pub struct Allocator {
    blocks: Vec<AllocatorBlock>,
}

impl Allocator {
    pub fn new(blocks: Vec<(usize, usize)>) -> Self {
        Allocator {
            blocks: blocks
                .into_iter()
                .map(|(start, end)| AllocatorBlock {
                    start_addr: start,
                    end_addr: end,
                    current_addr: start,
                })
                .collect(),
        }
    }

    pub fn allocate(&mut self, size: usize) -> Result<usize> {
        for block in &mut self.blocks {
            if block.end_addr - block.current_addr >= size {
                let addr = block.current_addr;
                block.current_addr += size;
                return Ok(addr);
            }
        }
        bail!("Failed to allocate {} bytes", size);
    }

    pub fn get_stats(&self) -> (usize, usize, usize) {
        let mut min_free = 0; // only count completely free blocks
        let mut max_free = 0; // include partially free blocks
        let mut total_capacity = 0;
        for block in &self.blocks {
            total_capacity += block.end_addr - block.start_addr;
            if block.current_addr == block.start_addr {
                min_free += block.end_addr - block.start_addr;
            }
            max_free += block.end_addr - block.current_addr;
        }
        (min_free, max_free, total_capacity)
    }
}

#[derive(Debug)]
pub enum MusicSettings {
    AreaThemed,
    Disabled,
}

#[derive(Debug)]
pub enum PaletteTheme {
    Vanilla,
    AreaThemed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TileTheme {
    Vanilla,
    AreaThemed,
    Scrambled,
    Constant(String),
}

#[derive(Default, Debug, Copy, Clone)]
pub enum DoorTheme {
    #[default]
    Vanilla,
    Alternate,
}

#[derive(Default, Debug, Copy, Clone)]
pub enum ControllerButton {
    #[default]
    Default,
    Left,
    Right,
    Up,
    Down,
    X,
    Y,
    A,
    B,
    L,
    R,
    Select,
    Start,
}

#[derive(Default, Debug)]
pub struct ControllerConfig {
    pub shot: ControllerButton,
    pub jump: ControllerButton,
    pub dash: ControllerButton,
    pub item_select: ControllerButton,
    pub item_cancel: ControllerButton,
    pub angle_up: ControllerButton,
    pub angle_down: ControllerButton,
    pub spin_lock_buttons: Vec<ControllerButton>,
    pub quick_reload_buttons: Vec<ControllerButton>,
    pub moonwalk: bool,
}

#[derive(Debug, Copy, Clone)]
pub enum ShakingSetting {
    Vanilla,
    Reduced,
    Disabled,
}

#[derive(Debug, Copy, Clone)]
pub enum FlashingSetting {
    Vanilla,
    Reduced,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ItemDotChange {
    Fade,
    Disappear,
}

#[derive(Debug)]
pub struct CustomizeSettings {
    pub samus_sprite: Option<String>,
    pub etank_color: Option<(u8, u8, u8)>,
    pub item_dot_change: ItemDotChange,
    pub transition_letters: bool,
    pub reserve_hud_style: bool,
    pub vanilla_screw_attack_animation: bool,
    pub palette_theme: PaletteTheme,
    pub tile_theme: TileTheme,
    pub door_theme: DoorTheme,
    pub music: MusicSettings,
    pub disable_beeping: bool,
    pub shaking: ShakingSetting,
    pub flashing: FlashingSetting,
    pub room_names: bool,
    pub controller_config: ControllerConfig,
}

impl Default for CustomizeSettings {
    fn default() -> Self {
        Self {
            samus_sprite: Some("samus_vanilla".to_string()),
            etank_color: None,
            item_dot_change: ItemDotChange::Fade,
            transition_letters: true,
            reserve_hud_style: true,
            vanilla_screw_attack_animation: true,
            room_names: true,
            palette_theme: PaletteTheme::Vanilla,
            tile_theme: TileTheme::AreaThemed,
            door_theme: DoorTheme::Vanilla,
            music: MusicSettings::AreaThemed,
            disable_beeping: false,
            shaking: ShakingSetting::Vanilla,
            flashing: FlashingSetting::Vanilla,
            controller_config: ControllerConfig::default(),
        }
    }
}

fn remove_mother_brain_flashing(rom: &mut Rom) -> Result<()> {
    // Disable start of flashing after Mother Brain 1:
    rom.write_u16(snes2pc(0xA9CFFE), 0)?;

    // Disable end of flashing (to prevent palette from getting overwritten)
    rom.write_u8(snes2pc(0xA9D00C), 0x60)?; // RTS

    Ok(())
}

fn apply_custom_samus_sprite(
    rom: &mut Rom,
    settings: &CustomizeSettings,
    samus_sprite_categories: &[SamusSpriteCategory],
) -> Result<()> {
    if settings.samus_sprite.is_some() || !settings.vanilla_screw_attack_animation {
        let sprite_name = settings
            .samus_sprite
            .clone()
            .unwrap_or("samus_vanilla".to_string());
        let patch_path_str = format!("../patches/samus_sprites/{sprite_name}.ips");
        apply_ips_patch(rom, Path::new(&patch_path_str))?;

        if settings.vanilla_screw_attack_animation {
            // Disable spin attack animation, to make it behave like vanilla: Screw attack animation will look like
            // you have Space Jump even if you don't:
            rom.write_u16(snes2pc(0x9B93FE), 0)?;
        }
    }

    // Patch credits to give credit to the sprite author:
    if let Some(sprite_name) = settings.samus_sprite.as_ref() {
        for category in samus_sprite_categories {
            for info in &category.sprites {
                if &info.name == sprite_name {
                    // Write the sprite name
                    let mut chars = vec![];
                    let credits_name = info
                        .credits_name
                        .clone()
                        .unwrap_or(info.display_name.clone());
                    for c in credits_name.chars() {
                        let c = c.to_ascii_uppercase();
                        if c.is_ascii_uppercase() || c == ' ' {
                            chars.push(c);
                        }
                    }
                    chars.extend(" SPRITE".chars());
                    let mut addr =
                        snes2pc(0xceb240 + (234 - 128) * 0x40) + 0x20 - chars.len().div_ceil(2) * 2;
                    for c in chars {
                        let color_palette = 0x0400;
                        if c.is_ascii_uppercase() {
                            rom.write_u16(addr, (c as isize - 'A' as isize) | color_palette)?;
                        }
                        addr += 2;
                    }

                    // Write the sprite author
                    let mut chars = vec![];
                    let author = info.authors.join(", ");
                    for c in author.chars() {
                        let c = c.to_ascii_uppercase();
                        if c.is_ascii_uppercase() || c.is_ascii_digit() || c == ' ' {
                            chars.push(c);
                        }
                    }
                    let mut addr =
                        snes2pc(0xceb240 + (235 - 128) * 0x40) + 0x20 - chars.len().div_ceil(2) * 2;
                    for c in chars {
                        write_credits_big_char(rom, c, addr)?;
                        addr += 2;
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn parse_controller_button(s: &str) -> Result<ControllerButton> {
    Ok(match s {
        "Left" => ControllerButton::Left,
        "Right" => ControllerButton::Right,
        "Up" => ControllerButton::Up,
        "Down" => ControllerButton::Down,
        "X" => ControllerButton::X,
        "Y" => ControllerButton::Y,
        "A" => ControllerButton::A,
        "B" => ControllerButton::B,
        "Select" => ControllerButton::Select,
        "L" => ControllerButton::L,
        "R" => ControllerButton::R,
        _ => bail!("Unexpected controller button: {}", s),
    })
}

fn get_button_mask(mut controller_button: ControllerButton, default: ControllerButton) -> isize {
    if let ControllerButton::Default = controller_button {
        controller_button = default;
    }
    match controller_button {
        ControllerButton::Left => 0x0200,
        ControllerButton::Right => 0x0100,
        ControllerButton::Up => 0x0800,
        ControllerButton::Down => 0x0400,
        ControllerButton::X => 0x0040,
        ControllerButton::Y => 0x4000,
        ControllerButton::A => 0x0080,
        ControllerButton::B => 0x8000,
        ControllerButton::L => 0x0020,
        ControllerButton::R => 0x0010,
        ControllerButton::Select => 0x2000,
        ControllerButton::Start => 0x1000,
        _ => panic!("Unexpected controller button: {controller_button:?}"),
    }
}

fn get_button_list_mask(buttons: &[ControllerButton]) -> isize {
    let mut mask = 0x0000;
    for &button in buttons {
        mask |= get_button_mask(button, ControllerButton::Default);
    }
    if mask == 0x0000 {
        // If no button are specified, assume this input combination (e.g. quick reload or spin lock)
        // is disabled, rather than being activated with no inputs held.
        mask = 0xFFFF;
    }
    mask
}

fn apply_controller_config(rom: &mut Rom, controller_config: &ControllerConfig) -> Result<()> {
    let control_data = vec![
        (0x81B325, controller_config.jump, ControllerButton::A),
        (0x81B32B, controller_config.dash, ControllerButton::B),
        (0x81B331, controller_config.shot, ControllerButton::X),
        (0x81B337, controller_config.item_cancel, ControllerButton::Y),
        (
            0x81B33D,
            controller_config.item_select,
            ControllerButton::Select,
        ),
        (0x81B343, controller_config.angle_up, ControllerButton::R),
        (0x81B349, controller_config.angle_down, ControllerButton::L),
    ];
    for (addr, button, default) in control_data {
        let mask = get_button_mask(button, default);
        rom.write_u16(snes2pc(addr), mask)?;
    }

    let spin_lock_mask = get_button_list_mask(&controller_config.spin_lock_buttons);
    rom.write_u16(snes2pc(0x82FE7C), spin_lock_mask)?;

    let quick_reload_mask = get_button_list_mask(&controller_config.quick_reload_buttons);
    rom.write_u16(snes2pc(0x82FE7E), quick_reload_mask)?;

    if controller_config.moonwalk {
        apply_ips_patch(rom, Path::new("../patches/ips/enable_moonwalk.ips"))?;
    }
    // $82FE7E

    Ok(())
}

pub fn customize_rom(
    rom: &mut Rom,
    orig_rom: &Rom,
    map: &Map,
    settings: &CustomizeSettings,
    game_data: &GameData,
    samus_sprite_categories: &[SamusSpriteCategory],
    mosaic_themes: &[MosaicTheme],
) -> Result<()> {
    rom.resize(0x400000);

    remove_mother_brain_flashing(rom)?;
    apply_retiling(
        rom,
        orig_rom,
        map,
        game_data,
        &settings.tile_theme,
        mosaic_themes,
    )?;

    match &settings.palette_theme {
        PaletteTheme::Vanilla => {}
        PaletteTheme::AreaThemed => {
            apply_area_themed_palettes(rom)?;
        }
    }

    match settings.door_theme {
        DoorTheme::Vanilla => {}
        DoorTheme::Alternate => {
            apply_ips_patch(rom, Path::new("../patches/ips/alternate_door_colors.ips"))?;
        }
    }

    // Fix Phantoon power-on sequence to not overwrite the first two palettes, since those contain
    // customized HUD colors which would get messed up.
    rom.write_u16(snes2pc(0xA7DC6E), 0x0040)?;

    apply_custom_samus_sprite(rom, settings, samus_sprite_categories)?;
    if let Some((r, g, b)) = settings.etank_color {
        let color = (r as isize) | ((g as isize) << 5) | ((b as isize) << 10);
        rom.write_u16(snes2pc(0x82FFFE), color)?; // Gameplay ETank color
        rom.write_u16(snes2pc(0x8EE416), color)?; // Main menu
        rom.write_u16(snes2pc(0xA7CA7B), color)?; // During Phantoon power-on
    }
    if settings.reserve_hud_style {
        apply_ips_patch(rom, Path::new("../patches/ips/reserve_hud.ips"))?;
    }
    if settings.room_names {
        rom.write_u16(snes2pc(0x82FFFA), 1)?;
    } else {
        rom.write_u16(snes2pc(0x82FFFA), 0)?;
    }
    match settings.music {
        MusicSettings::AreaThemed => {}
        MusicSettings::Disabled => {
            // We could call `override_music` here to restore the vanilla tracks: this would restore the correct sound effects
            // but at a cost of increasing room load times by almost 1 second per room.
            // override_music(rom)?;
            rom.write_u8(snes2pc(0xcf8413), 0x6F)?;
        }
    }
    if settings.disable_beeping {
        rom.write_n(snes2pc(0x90EA92), &[0xEA; 4])?;
        rom.write_n(snes2pc(0x90EAA0), &[0xEA; 4])?;
        rom.write_n(snes2pc(0x90F33C), &[0xEA; 4])?;
        rom.write_n(snes2pc(0x91E6DA), &[0xEA; 4])?;
    }
    match settings.shaking {
        ShakingSetting::Vanilla => {}
        ShakingSetting::Reduced => {
            // Limit BG shaking to 1-pixel displacements:
            for i in 0..144 {
                let x = rom.read_u16(snes2pc(0xA0872D + i * 2))?;
                rom.write_u16(snes2pc(0xA0872D + i * 2), min(x, 1))?;
            }
            // (Enemies already only shake up to 1 pixel)
            // Limit enemy projectile shaking to 1-pixel displacements:
            for i in 0..72 {
                let x = rom.read_u16(snes2pc(0x86846B + i * 2))?;
                rom.write_u16(snes2pc(0x86846B + i * 2), min(x, 1))?;
            }
        }
        ShakingSetting::Disabled => {
            // Disable BG shaking globally, by setting the shake displacements to zero (this should be timing-neutral?)
            rom.write_n(snes2pc(0xA0872D), &[0; 288])?;
            // Disable enemy shaking:
            rom.write_n(snes2pc(0xA09488), &[0xEA; 5])?; // 5 * NOP
            rom.write_n(snes2pc(0xA0948F), &[0xEA; 5])?; // 5 * NOP

            // Disable enemy projectile shaking, by setting the displacements to zero:
            rom.write_n(snes2pc(0x86846B), &[0; 144])?;
        }
    }
    match settings.flashing {
        FlashingSetting::Vanilla => {
            apply_ips_patch(rom, Path::new("../patches/ips/flashing_placebo.ips"))?;
        }
        FlashingSetting::Reduced => {
            apply_ips_patch(rom, Path::new("../patches/ips/flashing_placebo.ips"))?;
            write_glowpatch(rom, &game_data.reduced_flashing_patch)?;
        }
    }
    apply_controller_config(rom, &settings.controller_config)?;
    Ok(())
}
