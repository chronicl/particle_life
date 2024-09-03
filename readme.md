## Particle Life
![demo-image](https://github.com/chronicl/particle_life/blob/main/assets/screenshot.png)

A fully gpu driven implementation of particle life made in bevy.

The particles are simulated in compute shaders and drawn in a custom render pass.
On my RTX 2070 Super I achieve 60fps with 300k particles, bounds of 20k and max distance of 300 (these are the performance critical settings). However, the fps can drop lower based on how clustered the particles become, because a grid based spatial partitioning acceleration structure is used.
