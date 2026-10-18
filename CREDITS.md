# Credits

## Research

The shaders adapt techniques from these papers.

| Used in               | Technique                                                    | Source                                                                                                                                                 |
| --------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `packages/game/src/geometry/wood.wesl`  | Growth rings, pores and the fiber highlight of solid wood    | Albert J. Liu, Stephen R. Marschner, Victoria E. Dye. [Procedural wood textures](https://arxiv.org/abs/1511.04224). 2015.                              |
| `packages/game/src/geometry/bump.wesl`  | Bump mapping from screen space derivatives, without tangents | Morten S. Mikkelsen. [Bump Mapping Unparametrized Surfaces on the GPU](https://mmikk.github.io/papers3d/mm_sfgrad_bump.pdf). 2010.                     |
| `packages/game/src/geometry/noise.wesl` | The PCG hash for random numbers                              | Mark Jarzynski, Marc Olano. [Hash Functions for GPU Rendering](https://jcgt.org/published/0009/03/02/). Journal of Computer Graphics Techniques, 2020. |
