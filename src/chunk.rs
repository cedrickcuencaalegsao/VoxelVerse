use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use crate::block::{BlockType, Face};

pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_HEIGHT: usize = 64;

#[derive(Component)]
pub struct Chunk {
    pub position: IVec3,
    pub blocks: [[[BlockType; CHUNK_SIZE]; CHUNK_HEIGHT]; CHUNK_SIZE],
}

impl Chunk {
    pub fn new(position: IVec3) -> Self {
        Self {
            position,
            blocks: [[[BlockType::Air; CHUNK_SIZE]; CHUNK_HEIGHT]; CHUNK_SIZE],
        }
    }

    pub fn get_block(&self, x: usize, y: usize, z: usize) -> BlockType {
        if x >= CHUNK_SIZE || y >= CHUNK_HEIGHT || z >= CHUNK_SIZE {
            return BlockType::Air;
        }
        self.blocks[x][y][z]
    }

    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block_type: BlockType) {
        if x < CHUNK_SIZE && y < CHUNK_HEIGHT && z < CHUNK_SIZE {
            self.blocks[x][y][z] = block_type;
        }
    }

    pub fn is_face_visible(&self, x: usize, y: usize, z: usize, face: Face) -> bool {
        let block = self.get_block(x, y, z);
        if !block.is_solid() {
            return false;
        }

        let (nx, ny, nz) = match face {
            Face::Top => (x, y + 1, z),
            Face::Bottom => {
                if y == 0 {
                    return false;
                }
                (x, y - 1, z)
            }
            Face::North => (x, y, z + 1),
            Face::South => (x, y, z.wrapping_sub(1)),
            Face::East => (x + 1, y, z),
            Face::West => (x.wrapping_sub(1), y, z),
        };

        if nx >= CHUNK_SIZE || ny >= CHUNK_HEIGHT || nz >= CHUNK_SIZE {
            return true;
        }

        let neighbor = self.get_block(nx, ny, nz);
        neighbor.is_transparent()
    }

    pub fn generate_mesh(&self) -> Mesh {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();

        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_HEIGHT {
                for z in 0..CHUNK_SIZE {
                    let block = self.get_block(x, y, z);
                    if !block.is_solid() {
                        continue;
                    }

                    for face in Face::all() {
                        if !self.is_face_visible(x, y, z, face) {
                            continue;
                        }

                        let world_pos = Vec3::new(
                            (self.position.x * CHUNK_SIZE as i32 + x as i32) as f32,
                            y as f32,
                            (self.position.z * CHUNK_SIZE as i32 + z as i32) as f32,
                        );

                        let vertices = face.get_vertices(world_pos);
                        let normal = face.normal();
                        let start_index = positions.len() as u32;

                        for vertex in vertices.iter() {
                            positions.push([vertex.x, vertex.y, vertex.z]);
                            normals.push([normal.x, normal.y, normal.z]);
                        }

                        indices.extend_from_slice(&[
                            start_index,
                            start_index + 1,
                            start_index + 2,
                            start_index,
                            start_index + 2,
                            start_index + 3,
                        ]);
                    }
                }
            }
        }

        Mesh::new(PrimitiveTopology::TriangleList, Default::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_indices(Indices::U32(indices))
    }

    pub fn generate_lod_mesh(&self) -> Mesh {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();

        const STEP: usize = 2;

        let mut x = 0;
        while x + STEP <= CHUNK_SIZE {
            let mut z = 0;
            while z + STEP <= CHUNK_SIZE {
                let h00 = self.top_solid_y(x,        z       );
                let h10 = self.top_solid_y(x + STEP, z       );
                let h01 = self.top_solid_y(x,        z + STEP);
                let h11 = self.top_solid_y(x + STEP, z + STEP);

                let wx = (self.position.x * CHUNK_SIZE as i32 + x as i32) as f32;
                let wz = (self.position.z * CHUNK_SIZE as i32 + z as i32) as f32;
                let step = STEP as f32;

                let base = positions.len() as u32;

                positions.push([wx,        h00, wz       ]);
                positions.push([wx + step, h10, wz       ]);
                positions.push([wx + step, h11, wz + step]);
                positions.push([wx,        h01, wz + step]);

                let normal = [0.0f32, 1.0, 0.0];
                for _ in 0..4 {
                    normals.push(normal);
                }

                indices.extend_from_slice(&[
                    base, base + 1, base + 2,
                    base, base + 2, base + 3,
                ]);

                z += STEP;
            }
            x += STEP;
        }

        Mesh::new(PrimitiveTopology::TriangleList, Default::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_indices(Indices::U32(indices))
    }

    fn top_solid_y(&self, x: usize, z: usize) -> f32 {
        let x = x.min(CHUNK_SIZE - 1);
        let z = z.min(CHUNK_SIZE - 1);
        for y in (0..CHUNK_HEIGHT).rev() {
            let b = self.get_block(x, y, z);
            if b.is_solid() && !matches!(b, BlockType::Water) {
                return y as f32 + 1.0;
            }
        }
        0.0
    }
}

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, _app: &mut App) {}
}