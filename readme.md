# Voxel Game - Minecraft-like Clone

A voxel-based game built with Rust and Bevy 0.14, featuring procedural terrain generation, chunk management, and first-person controls.

## Features

- 🌍 Procedural terrain generation using Perlin noise
- 🧊 Chunk-based world with dynamic loading/unloading
- 🌲 Tree generation
- 🎮 First-person camera controls
- 🏃 Flying movement (creative mode)
- 🎨 Colored voxel blocks (grass, dirt, stone, sand, wood, leaves)
- ⚡ Optimized mesh generation with face culling

## Controls

### Camera
- **Mouse Movement**: Look around
- **Left Click**: Grab cursor (lock mouse to window)
- **ESC**: Release cursor

### Movement
- **W**: Move forward
- **S**: Move backward
- **A**: Strafe left
- **D**: Strafe right
- **Space**: Move up
- **Left Shift**: Move down
- **Left Ctrl**: Sprint (2x speed)

## Building and Running

### Prerequisites
- Rust (latest stable version)
- Cargo

### Run the game
```bash
cargo run --release
```

For development (faster compile, slower runtime):
```bash
cargo run
```

## Project Structure

```
src/
├── main.rs        # Entry point and app setup
├── block.rs       # Block types and face definitions
├── chunk.rs       # Chunk data structure and meshing
├── world.rs       # World generation and chunk management
├── camera.rs      # First-person camera system
├── input.rs       # Keyboard input handling
└── physics.rs     # Basic physics and collision
```

## How It Works

### Chunk System
- World is divided into 16x64x16 chunks
- Chunks are dynamically loaded based on camera position
- Render distance: 4 chunks in each direction
- Chunks outside render distance are automatically unloaded

### Terrain Generation
- Uses Perlin noise for natural-looking terrain
- Base height: 32 blocks
- Height variation: ±15 blocks
- Different biomes based on height:
  - **Below 30**: Sand (beach/desert)
  - **30-35**: Grass (plains)
  - **Above 35**: Stone (mountains)

### Mesh Generation
- Greedy face culling: Only visible block faces are rendered
- Blocks hidden underground are not meshed
- Vertex colors for different block types
- Optimized for performance

### Tree Generation
- Procedurally placed using noise
- 5-block tall trunks
- Leaf canopy with natural shape
- Only spawns on grass blocks at appropriate heights

## Performance Tips

1. **Render Distance**: Decrease `render_distance` in `world.rs` for better FPS
2. **Chunk Size**: Modify `CHUNK_SIZE` in `chunk.rs` (default: 16)
3. **Chunk Height**: Adjust `CHUNK_HEIGHT` in `chunk.rs` (default: 64)

## Future Enhancements

Potential features to add:
- [ ] Block breaking and placing
- [ ] Inventory system
- [ ] More block types
- [ ] Water and lava
- [ ] Caves and underground generation
- [ ] Better collision detection
- [ ] Gravity and jumping
- [ ] Day/night cycle
- [ ] Mob spawning
- [ ] Save/load world data
- [ ] Multiplayer support

## Technical Details

- **Engine**: Bevy 0.14
- **Noise Generation**: Perlin noise (noise crate)
- **Language**: Rust 2021 edition
- **Graphics**: PBR rendering with vertex colors

## Troubleshooting

### Low FPS
- Try running in release mode: `cargo run --release`
- Reduce render distance in `world.rs`
- Close other applications

### Cursor not locking
- Click the left mouse button in the game window
- Press ESC to unlock the cursor

## License

This project is open source and available for educational purposes.

## Credits

Built with:
- [Bevy](https://bevyengine.org/) - Game engine
- [Noise](https://crates.io/crates/noise) - Procedural generation
- [Rand](https://crates.io/crates/rand) - Random number generation