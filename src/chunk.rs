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
            for z in 0..CHUNK_SIZE {
                // Find the topmost solid OR water block
                let mut top_y = None;
                let mut top_block = BlockType::Air;

                for y in (0..CHUNK_HEIGHT).rev() {
                    let block = self.get_block(x, y, z);

                    // Water surface — show water on top
                    if matches!(block, BlockType::Water) {
                        // Only show water if there's air above it
                        let above = if y + 1 < CHUNK_HEIGHT {
                            self.get_block(x, y + 1, z)
                        } else {
                            BlockType::Air
                        };
                        if matches!(above, BlockType::Air) {
                            top_y = Some(y);
                            top_block = BlockType::Water;
                            break;
                        }
                    }

                    // Solid non-water block
                    if block.is_solid() && !matches!(block, BlockType::Water) {
                        let above = if y + 1 < CHUNK_HEIGHT {
                            self.get_block(x, y + 1, z)
                        } else {
                            BlockType::Air
                        };
                        if above.is_transparent() {
                            top_y = Some(y);
                            top_block = block;
                            break;
                        }
                    }
                }

                let Some(ty) = top_y else { continue };

                // Top surface block (grass, water, sand, etc.)
                surface.push((x, ty, z, top_block));

                // Only add subsurface layers under non-water blocks
                if !matches!(top_block, BlockType::Water) {
                    for depth in 1..=2usize {
                        if ty >= depth {
                            let by = ty - depth;
                            let b = self.get_block(x, by, z);
                            if b.is_solid() && !matches!(b, BlockType::Water) {
                                surface.push((x, by, z, b));
                            }
                        }
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
