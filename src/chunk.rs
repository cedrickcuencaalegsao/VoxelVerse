use bevy::prelude::*;
use crate::block::BlockType;

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

    /// Returns all surface blocks — blocks that have air above them
    pub fn get_surface_blocks(&self) -> Vec<(usize, usize, usize, BlockType)> {
        let mut surface = Vec::new();
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for y in (0..CHUNK_HEIGHT).rev() {
                    let block = self.get_block(x, y, z);
                    if block.is_solid() && !matches!(block, BlockType::Water) {
                        // Check if the block above is air or water
                        let above = if y + 1 < CHUNK_HEIGHT {
                            self.get_block(x, y + 1, z)
                        } else {
                            BlockType::Air
                        };
                        if above.is_transparent() {
                            surface.push((x, y, z, block));
                        }
                        break;
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