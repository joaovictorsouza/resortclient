# Mundo 3D, Tiles e Detecção de Clique

## Conversão de Coordenadas

```rust
pub fn tile_to_world(x: i32, z: i32) -> Vec3 {
    Vec3::new(x as f32 + 0.5, 0.1, z as f32 + 0.5)
}

pub fn world_to_tile(pos: Vec3) -> (i32, i32) {
    (pos.x.floor() as i32, pos.z.floor() as i32)
}
```

Cada tile possui dimensão de `1.0m x 0.2m x 1.0m`.
Ao receber o snapshot da sala, o cliente spawna apenas os tiles caminháveis (onde o heightmap não possui `'x'`).
Cada tile recebe um `avian3d::prelude::Collider::cuboid(1.0, 0.2, 1.0)` para suporte à física e raycasting.
