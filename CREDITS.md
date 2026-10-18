# Credits

## Research

The shaders and sounds adapt techniques from these papers and articles.

| Used in                                             | Technique                                                                       | Source                                                                                                                                                                                                                         |
| --------------------------------------------------- | ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `packages/game/src/geometry/wood.wesl`              | Growth rings, pores and the fiber highlight of solid wood                       | Albert J. Liu, Stephen R. Marschner, Victoria E. Dye. [Procedural wood textures](https://arxiv.org/abs/1511.04224). 2015.                                                                                                      |
| `packages/game/src/geometry/bump.wesl`              | Bump mapping from screen space derivatives, without tangents                    | Morten S. Mikkelsen. [Bump Mapping Unparametrized Surfaces on the GPU](https://mmikk.github.io/papers3d/mm_sfgrad_bump.pdf). 2010.                                                                                             |
| `packages/game/src/geometry/noise.wesl`             | The PCG hash for random numbers                                                 | Mark Jarzynski, Marc Olano. [Hash Functions for GPU Rendering](https://jcgt.org/published/0009/03/02/). Journal of Computer Graphics Techniques, 2020.                                                                         |
| `packages/game/src/audio/synth.rs`                  | Contact sounds from short hits ringing banks of damped modes                    | Kees van den Doel, Paul G. Kry, Dinesh K. Pai. [FoleyAutomatic: Physically-based Sound Effects for Interactive Simulation and Animation](https://www.cs.mcgill.ca/~kry/pubs/foleyautomatic/foleyautomatic.pdf). SIGGRAPH 2001. |
| `packages/game/src/city/atmosphere/height_fog.wesl` | Haze that thins out with height, integrated along each view ray                 | Iñigo Quilez. [Better Fog](https://iquilezles.org/articles/fog/). 2010.                                                                                                                                                        |
| `packages/game/src/parts/liquid.rs`                 | Sloshing modes of a liquid in a round container, and how fast each one swings   | H. Norman Abramson (ed.). [The Dynamic Behavior of Liquids in Moving Containers](https://ntrs.nasa.gov/citations/19670006555). NASA SP-106, 1966.                                                                              |
| `packages/game/src/parts/steam.rs`                  | A plume that widens at a steady rate as it rises and takes in the air around it | B. R. Morton, G. I. Taylor, J. S. Turner. [Turbulent gravitational convection from maintained and instantaneous sources](https://doi.org/10.1098/rspa.1956.0011). Proceedings of the Royal Society A, 1956.                    |
| `packages/game/src/parts/steam.rs`                  | Wisps from noise looked up at a point that other noise pushes around            | Iñigo Quilez. [Domain Warping](https://iquilezles.org/articles/warp/). 2002.                                                                                                                                                   |

## Music

The radio plays these tracks, converted to mono Ogg Vorbis and filtered to sound
like a small speaker. They are in `packages/game/assets/music`.

| Files                   | Track                                 | Source                                                                     | License   |
| ----------------------- | ------------------------------------- | -------------------------------------------------------------------------- | --------- |
| `stargazer-*.ogg`       | Stargazer (album) by e s c p          | [itch.io](https://escpmusic.itch.io/stargazer)                             | CC BY 4.0 |
| `nostalgia.ogg`         | NOSTALGIA, audio by GLORYTOTHEMACHINE | [OpenGameArt](https://opengameart.org/content/nostalgia-gl0ryt0th3m4ch1n3) | CC BY 4.0 |
| `exploration-theme.ogg` | Exploration Theme by Cleyton Kauffman | [OpenGameArt](https://opengameart.org/content/exploration-theme)           | CC0       |

## Fonts

`packages/game/src/parts/dot_matrix/font.rs` holds the glyphs of the public
domain 5x7 font from X11's misc-fixed fonts.
