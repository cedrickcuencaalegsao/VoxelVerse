use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::block::BlockType;
use crate::item_preview::ItemPreviews;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemKind {
    Grass,
    Dirt,
    Stone,
    Sand,
    Wood,
    Leaves,
    Weed,
}

impl ItemKind {
    pub const ALL: &'static [ItemKind] = &[
        ItemKind::Grass,
        ItemKind::Dirt,
        ItemKind::Stone,
        ItemKind::Sand,
        ItemKind::Wood,
        ItemKind::Leaves,
        ItemKind::Weed,
    ];

    // Kept for potential debug logging, but no longer rendered in the UI
    pub fn short(&self) -> &'static str {
        match self {
            ItemKind::Grass => "Grass",
            ItemKind::Dirt => "Dirt",
            ItemKind::Stone => "Stone",
            ItemKind::Sand => "Sand",
            ItemKind::Wood => "Wood",
            ItemKind::Leaves => "Leaves",
            ItemKind::Weed => "Weed",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            ItemKind::Grass => Color::srgb(0.25, 0.65, 0.15),
            ItemKind::Dirt => Color::srgb(0.45, 0.28, 0.12),
            ItemKind::Stone => Color::srgb(0.55, 0.55, 0.55),
            ItemKind::Sand => Color::srgb(0.85, 0.78, 0.50),
            ItemKind::Wood => Color::srgb(0.50, 0.32, 0.14),
            ItemKind::Leaves => Color::srgb(0.18, 0.55, 0.12),
            ItemKind::Weed => Color::srgb(0.35, 0.80, 0.30),
        }
    }

    pub fn from_block(b: BlockType) -> Option<ItemKind> {
        Some(match b {
            BlockType::Grass => ItemKind::Grass,
            BlockType::Dirt => ItemKind::Dirt,
            BlockType::Stone => ItemKind::Stone,
            BlockType::Sand => ItemKind::Sand,
            BlockType::Wood => ItemKind::Wood,
            BlockType::Leaves => ItemKind::Leaves,
            BlockType::Water | BlockType::Air => return None,
        })
    }
}

/// Tag placed on any ground-drop entity so the pickup systems know which
/// item to credit to the inventory when the player walks over it.
#[derive(Component, Clone, Copy)]
pub struct Pickup {
    pub kind: ItemKind,
}

#[derive(Resource, Default)]
pub struct Inventory {
    pub counts: HashMap<ItemKind, u32>,
}

impl Inventory {
    pub fn add(&mut self, kind: ItemKind, n: u32) {
        *self.counts.entry(kind).or_insert(0) += n;
    }

    pub fn get(&self, kind: ItemKind) -> u32 {
        self.counts.get(&kind).copied().unwrap_or(0)
    }
}

#[derive(Component)]
pub struct HotbarSlot {
    pub kind: ItemKind,
}

#[derive(Component)]
pub struct HotbarCountText {
    pub kind: ItemKind,
}

#[derive(Component)]
pub struct HotbarBar {
    pub kind: ItemKind,
}

/// Marks the `ImageBundle` icon node so we can swap its image once previews load.
#[derive(Component)]
pub struct HotbarIcon {
    pub kind: ItemKind,
}

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Inventory>()
            .add_systems(Startup, setup_hotbar)
            .add_systems(Update, (update_hotbar, apply_preview_images));
    }
}

// ── hotbar setup ─────────────────────────────────────────────────────────────
fn setup_hotbar(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(18.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                height: Val::Px(82.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                column_gap: Val::Px(6.0),
                ..default()
            },
            z_index: ZIndex::Global(20),
            ..default()
        })
        .with_children(|parent| {
            for &kind in ItemKind::ALL {
                parent
                    .spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Px(58.0),
                                height: Val::Px(78.0),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::FlexStart,
                                padding: UiRect::all(Val::Px(4.0)),
                                border: UiRect::all(Val::Px(2.0)),
                                row_gap: Val::Px(5.0), // increased slightly to compensate for removed text gap
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                            border_color: BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.35)),
                            ..default()
                        },
                        HotbarSlot { kind },
                    ))
                    .with_children(|slot| {
                        // ── icon (ImageBundle, filled later by apply_preview_images) ──
                        slot.spawn((
                            ImageBundle {
                                style: Style {
                                    width: Val::Px(36.0),
                                    height: Val::Px(36.0),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                background_color: BackgroundColor(kind.color()),
                                ..default()
                            },
                            HotbarIcon { kind },
                        ));

                        // ── progress bar background ───────────────────────────
                        slot.spawn(NodeBundle {
                            style: Style {
                                width: Val::Px(44.0),
                                height: Val::Px(6.0),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.10)),
                            border_color: BorderColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                            ..default()
                        })
                        .with_children(|bar_bg| {
                            bar_bg.spawn((
                                NodeBundle {
                                    style: Style {
                                        width: Val::Percent(0.0),
                                        height: Val::Percent(100.0),
                                        ..default()
                                    },
                                    background_color: BackgroundColor(kind.color()),
                                    ..default()
                                },
                                HotbarBar { kind },
                            ));
                        });

                        // ── count text ────────────────────────────────────────
                        slot.spawn((
                            TextBundle::from_section(
                                "0",
                                TextStyle {
                                    font_size: 13.0,
                                    color: Color::srgba(1.0, 1.0, 1.0, 0.4),
                                    ..default()
                                },
                            ),
                            HotbarCountText { kind },
                        ));
                    });
            }
        });
}

// ── apply render-target images once they are available ───────────────────────
fn apply_preview_images(
    mut commands: Commands,
    previews: Res<ItemPreviews>,
    mut icon_query: Query<(Entity, &HotbarIcon, &mut UiImage)>,
) {
    for (entity, icon, mut ui_image) in icon_query.iter_mut() {
        if let Some(handle) = previews.images.get(&icon.kind) {
            let h: Handle<Image> = handle.clone();
            ui_image.texture = h;
            commands.entity(entity).remove::<HotbarIcon>();
        }
    }
}

// ── live inventory updates ────────────────────────────────────────────────────
const HOTBAR_FULL_AT: f32 = 64.0;
fn update_hotbar(
    inventory: Res<Inventory>,
    mut count_query: Query<(&mut Text, &HotbarCountText)>,
    mut bar_query: Query<(&mut Style, &HotbarBar)>,
) {
    if !inventory.is_changed() {
        return;
    }
    for (mut text, tag) in count_query.iter_mut() {
        let n = inventory.get(tag.kind);
        text.sections[0].value = format!("{}", n);
        text.sections[0].style.color = if n > 0 {
            Color::srgb(1.0, 1.0, 0.4)
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.35)
        };
    }
    for (mut style, bar) in bar_query.iter_mut() {
        let n = inventory.get(bar.kind) as f32;
        let pct = (n / HOTBAR_FULL_AT * 100.0).clamp(0.0, 100.0);
        style.width = Val::Percent(pct);
    }
}