use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockType {
    Air,
    Grass,
    Dirt,
    Stone,
    Sand,
    Wood,
    Leaves,
    Water,
}

#[allow(dead_code)]
impl BlockType {
    pub fn is_solid(&self) -> bool {
        // Water is "solid" for the mesher (it has a mesh), 
        // but physically you'd handle it differently in physics.rs
        !matches!(self, BlockType::Air)
    }

    pub fn is_transparent(&self) -> bool {
        // Water and Leaves allow light/faces to be seen through them
        matches!(self, BlockType::Air | BlockType::Leaves | BlockType::Water)
    }

    pub fn get_color(&self) -> Color {
        match self {
            BlockType::Air => Color::NONE,
            BlockType::Grass => Color::srgb(0.2, 0.8, 0.2),
            BlockType::Dirt => Color::srgb(0.45, 0.3, 0.15),
            BlockType::Stone => Color::srgb(0.5, 0.5, 0.5),
            BlockType::Sand => Color::srgb(0.9, 0.85, 0.6),
            BlockType::Wood => Color::srgb(0.35, 0.2, 0.1),
            BlockType::Leaves => Color::srgb(0.1, 0.5, 0.1),
            BlockType::Water => Color::srgba(0.0, 0.3, 0.8, 0.8), // Deep Blue with some alpha
        }
    }

    pub fn get_top_color(&self) -> Color {
        match self {
            BlockType::Grass => Color::srgb(0.3, 0.7, 0.3),
            BlockType::Water => Color::srgb(0.1, 0.4, 0.9), // Slightly lighter surface
            _ => self.get_color(),
        }
    }

    pub fn get_side_color(&self) -> Color {
        match self {
            BlockType::Grass => Color::srgb(0.4, 0.5, 0.2),
            _ => self.get_color(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Top,
    Bottom,
    North,
    South,
    East,
    West,
}

impl Face {
    pub fn normal(&self) -> Vec3 {
        match self {
            Face::Top => Vec3::Y,
            Face::Bottom => Vec3::NEG_Y,
            Face::North => Vec3::Z,
            Face::South => Vec3::NEG_Z,
            Face::East => Vec3::X,
            Face::West => Vec3::NEG_X,
        }
    }

    pub fn get_vertices(&self, pos: Vec3) -> [Vec3; 4] {
        let x = pos.x;
        let y = pos.y;
        let z = pos.z;

        match self {
            Face::Top => [
                Vec3::new(x, y + 1.0, z),
                Vec3::new(x + 1.0, y + 1.0, z),
                Vec3::new(x + 1.0, y + 1.0, z + 1.0),
                Vec3::new(x, y + 1.0, z + 1.0),
            ],
            Face::Bottom => [
                Vec3::new(x, y, z + 1.0),
                Vec3::new(x + 1.0, y, z + 1.0),
                Vec3::new(x + 1.0, y, z),
                Vec3::new(x, y, z),
            ],
            Face::North => [
                Vec3::new(x, y, z + 1.0),
                Vec3::new(x, y + 1.0, z + 1.0),
                Vec3::new(x + 1.0, y + 1.0, z + 1.0),
                Vec3::new(x + 1.0, y, z + 1.0),
            ],
            Face::South => [
                Vec3::new(x + 1.0, y, z),
                Vec3::new(x + 1.0, y + 1.0, z),
                Vec3::new(x, y + 1.0, z),
                Vec3::new(x, y, z),
            ],
            Face::East => [
                Vec3::new(x + 1.0, y, z + 1.0),
                Vec3::new(x + 1.0, y + 1.0, z + 1.0),
                Vec3::new(x + 1.0, y + 1.0, z),
                Vec3::new(x + 1.0, y, z),
            ],
            Face::West => [
                Vec3::new(x, y, z),
                Vec3::new(x, y + 1.0, z),
                Vec3::new(x, y + 1.0, z + 1.0),
                Vec3::new(x, y, z + 1.0),
            ],
        }
    }

    pub fn all() -> [Face; 6] {
        [
            Face::Top,
            Face::Bottom,
            Face::North,
            Face::South,
            Face::East,
            Face::West,
        ]
    }
}