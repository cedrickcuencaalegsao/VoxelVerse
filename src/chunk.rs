use crate::block::BlockType;
use bevy::prelude::*;

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
    pub fn get_surface_blocks(&self) -> Vec<(usize, usize, usize, BlockType)> {
        let mut surface = Vec::new();

        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_HEIGHT {
                for z in 0..CHUNK_SIZE {
                    let b = self.blocks[x][y][z];
                    if b == BlockType::Air || b == BlockType::Stone {
                        continue;
                    }

                    let mut exposed = false;

                    if x == 0
                        || x == CHUNK_SIZE - 1
                        || z == 0
                        || z == CHUNK_SIZE - 1
                        || y == 0
                        || y == CHUNK_HEIGHT - 1
                    {
                        exposed = true;
                    } else {
                        if self.blocks[x][y + 1][z] == BlockType::Air
                            || self.blocks[x][y - 1][z] == BlockType::Air
                            || self.blocks[x + 1][y][z] == BlockType::Air
                            || self.blocks[x - 1][y][z] == BlockType::Air
                            || self.blocks[x][y][z + 1] == BlockType::Air
                            || self.blocks[x][y][z - 1] == BlockType::Air
                        {
                            exposed = true;
                        }
                    }

                    if exposed {
                        surface.push((x, y, z, b));
                    }
                }
            }
        }

        surface
    }
}

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, _app: &mut App) {}
}
