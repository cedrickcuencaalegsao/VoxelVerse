use bevy::prelude::*;
use bevy::utils::HashMap;

use crate::block::BlockType;

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

#[derive(Resource, Default)]
pub struct InventoryOpen(pub bool);

#[derive(Component)]
pub struct InventoryPanel;

#[derive(Component)]
pub struct InventoryIcon {
    pub kind: ItemKind,
}

#[derive(Component)]
pub struct InventoryCountText {
    pub kind: ItemKind,
}

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Inventory>()
            .init_resource::<InventoryOpen>()
            .add_systems(Startup, setup_inventory_ui)
            .add_systems(
                Update,
                (toggle_inventory, update_inventory_ui, apply_preview_images),
            );
    }
}

fn setup_inventory_ui(mut commands: Commands) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.88)), // dark "blurry" overlay
                visibility: Visibility::Hidden,
                z_index: ZIndex::Global(100),
                ..default()
            },
            InventoryPanel,
        ))
        .with_children(|overlay| {
            overlay
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(640.0),
                        height: Val::Px(460.0),
                        margin: UiRect::all(Val::Auto),
                        align_self: AlignSelf::Center,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::FlexStart,
                        padding: UiRect::all(Val::Px(24.0)),
                        border: UiRect::all(Val::Px(6.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.98)),
                    border_color: BorderColor(Color::srgba(0.9, 0.85, 0.7, 0.6)),
                    ..default()
                })
                .with_children(|panel| {
                    // Title
                    panel.spawn(TextBundle::from_section(
                        "INVENTORY",
                        TextStyle {
                            font_size: 36.0,
                            color: Color::srgb(1.0, 0.95, 0.4),
                            ..default()
                        },
                    ));

                    panel
                        .spawn(NodeBundle {
                            style: Style {
                                width: Val::Px(580.0),
                                height: Val::Px(320.0),
                                flex_wrap: FlexWrap::Wrap,
                                justify_content: JustifyContent::Center,
                                align_content: AlignContent::Center,
                                column_gap: Val::Px(14.0),
                                row_gap: Val::Px(14.0),
                                margin: UiRect::top(Val::Px(30.0)),
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|slots| {
                            for &kind in ItemKind::ALL {
                                slots
                                    .spawn(NodeBundle {
                                        style: Style {
                                            width: Val::Px(78.0),
                                            height: Val::Px(98.0),
                                            flex_direction: FlexDirection::Column,
                                            align_items: AlignItems::Center,
                                            justify_content: JustifyContent::FlexStart,
                                            padding: UiRect::all(Val::Px(8.0)),
                                            border: UiRect::all(Val::Px(3.0)),
                                            ..default()
                                        },
                                        background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
                                        border_color: BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.35)),
                                        ..default()
                                    })
                                    .with_children(|slot| {
                                        // Icon (starts with solid color, gets replaced by preview)
                                        slot.spawn((
                                            ImageBundle {
                                                style: Style {
                                                    width: Val::Px(52.0),
                                                    height: Val::Px(52.0),
                                                    ..default()
                                                },
                                                background_color: BackgroundColor(kind.color()),
                                                ..default()
                                            },
                                            InventoryIcon { kind },
                                        ));

                                        // Count text
                                        slot.spawn((
                                            TextBundle::from_section(
                                                "0",
                                                TextStyle {
                                                    font_size: 19.0,
                                                    color: Color::srgb(1.0, 1.0, 0.4),
                                                    ..default()
                                                },
                                            ),
                                            InventoryCountText { kind },
                                        ));
                                    });
                            }
                        });
                });
        });

    commands.spawn(NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(24.0),
            left: Val::Px(24.0),
            width: Val::Px(72.0),
            height: Val::Px(72.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(4.0)),
            ..default()
        },
        background_color: BackgroundColor(Color::srgba(0.25, 0.18, 0.12, 0.95)),
        border_color: BorderColor(Color::srgba(0.85, 0.7, 0.45, 0.9)),
        ..default()
    }).with_children(|bag| {
        bag.spawn(TextBundle::from_section(
            "👜", // will be replace 
            TextStyle {
                font_size: 48.0,
                color: Color::srgb(1.0, 0.9, 0.65),
                ..default()
            },
        ));
    });
}

fn toggle_inventory(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut open: ResMut<InventoryOpen>,
    mut panel_query: Query<&mut Visibility, With<InventoryPanel>>,
) {
    if keyboard.just_pressed(KeyCode::Tab) {
        open.0 = !open.0;

        for mut vis in &mut panel_query {
            *vis = if open.0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn update_inventory_ui(
    inventory: Res<Inventory>,
    mut count_query: Query<(&mut Text, &InventoryCountText)>,
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
            Color::srgba(1.0, 1.0, 1.0, 0.4)
        };
    }
}

fn apply_preview_images(
    mut commands: Commands,
    previews: Res<crate::item_preview::ItemPreviews>,
    mut icon_query: Query<(Entity, &InventoryIcon, &mut UiImage)>,
) {
    for (entity, icon, mut ui_image) in icon_query.iter_mut() {
        if let Some(handle) = previews.images.get(&icon.kind) {
            ui_image.texture = handle.clone();
            commands.entity(entity).remove::<InventoryIcon>();
        }
    }
}