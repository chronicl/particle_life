## Particle Life
![demo-image](https://github.com/chronicl/particle_life/blob/main/assets/screenshot.png)

A fully gpu driven implementation of particle life made in bevy.

Particle Life is a simulation that models interactions between particles, where each particle is assigned a color. The colors determine the interactions: a color may either attract, repel or be neutral towards each of the other colors. However, this relation is asymmetric, so for example blue may attract red, but red may be neutral or repel blue.

### Performance
The particles are simulated in compute shaders and drawn in a custom render pass.
On my rtx 2070 super I achieve 60fps with 300k particles, bounds of 20k and max distance of 300 (these are the performance critical settings). However, the fps can drop lower based on how clustered the particles become, because a grid based spatial partitioning acceleration structure is used.

### Installation
If you are on windows (64 bit) you can simply download the latest release from the [releases page](https://github.com/chronicl/particle_life/releases).

Otherwise you will have to build it yourself. Simply clone this repo, install [rust](https://www.rust-lang.org/tools/install) and run `cargo run --release --no-default-features`.

### Sharing
The settings include a "Copy settings to clipboard" button. Simply click that and you can share your amazing settings with other people (`ctrl + v`). They can then copy it and press the "Paste settings from clipboard" button to get the same settings as you.

If "Paste settings from clipboard" doesn't do anything, you are either using different versions of the app or the settings are invalid in some other way.
