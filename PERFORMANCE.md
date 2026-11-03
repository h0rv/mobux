# Rendering performance

The original native renderer was slow in the development build. It redrew while idle, rebuilt icon rasters on every frame, and drew the wallpaper as thousands of tiny rectangles. It also calculated a cell fallback for surfaces that already had a native image.

The current renderer caches icon assets and batches wallpaper pixels into complete rows. Native surfaces skip fallback sampling. Idle screens do not redraw. Unchanged backgrounds do not get PNG-encoded or uploaded. Development builds optimize this crate at level 1 and dependencies at level 2.

## Measurements

CPU-side measurements on this development machine used an Apple M5, one initial frame, and 100 warm frames. The initial baseline used 10 warm frames. Values below are rounded medians for drawing plus PNG encoding, in milliseconds.

| Terminal cells | Pixels per cell | Original development build | Optimized development build |
| --- | --- | --- | --- |
| 40 by 48 | 8 by 16 | 85.4 | 1.6 |
| 40 by 48 | 16 by 32 | 144.8 | 5.4 |
| 100 by 40 | 8 by 16 | 122.0 | 2.8 |
| 100 by 40 | 16 by 32 | 256.4 | 10.1 |

These improvements include both code changes and compiler optimization. They do not isolate either contribution. The current measurements include bundled icon masks and the shared background canvas. Later changes can move these numbers.

Unchanged backgrounds and clock-only changes uploaded zero image bytes in the benchmark. The event loop also avoids drawing idle frames, which this table does not measure.

## Reproduce

```sh
mise exec -- cargo run --bin bench
```

The command prints separate warm medians for drawing and PNG encoding, plus the unchanged-image upload count. It uses the normal development profile and 100 measured frames per size after one warmup frame. It does not measure input handling, terminal GPU work, SSH transfer, background text search, or phone hardware. A fast benchmark is not proof of a responsive end-to-end phone experience.

Live Ghostty captures are in `shots/live/`. Ghostty's native background and app switching have been checked on macOS. Phone performance and the Kitty application itself still need separate verification.

## Minimal layout pass

The current benchmark follows the default Minimal layout and prints that layout name. The earlier comparison used the Android layout. These are different workloads, so the following numbers are not a direct comparison with the original baseline.

| Terminal cells | Pixels per cell | Paint median, ms | PNG median, ms |
| --- | --- | --- | --- |
| 40 by 48 | 8 by 16 | 0.760 | 0.664 |
| 40 by 48 | 16 by 32 | 2.523 | 2.465 |
| 100 by 40 | 8 by 16 | 1.285 | 1.404 |
| 100 by 40 | 16 by 32 | 4.567 | 5.193 |

All four cases uploaded zero image bytes for an unchanged background. These measurements exclude terminal display and do not establish phone performance.
