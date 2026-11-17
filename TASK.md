# Task: robot link stiffness & deflection lab (Rust → WebAssembly, static web app)

Original open-source tool. Write all code yourself; do not copy anyone's code, tables or assets.
Use textbook methods only (Euler–Bernoulli beam theory, superposition, Rayleigh / closed-form first
bending mode, von Mises). Material values are generic and clearly labelled "typical" (no vendor names,
no copied datasheet tables).

## Product (SMALL scope — core + UI about 800-1200 lines of new Rust; renderer/math are reused)
A single-page web app (GitHub Pages, sub-path `/link-flex/`): "My robot arm / leg link is this tube or
printed beam with this payload — how much does the tip sag, is the stress safe, and how stiff / how
fast does it ring?"

1. **Inputs**: link length L (mm), up to 3 serial links (chain of cantilevered segments, each with
   its own section + material + length), cross-section per link: round tube (OD, wall), rectangular
   tube (W, H, wall), solid rectangle (W, H), I-beam-ish (W, H, flange t, web t); material per link:
   presets "aluminium 6061 (typical)", "steel (typical)", "CFRP tube (typical, quasi-isotropic)",
   "PLA printed (typical, 100 % infill)", "PETG printed (typical)", "nylon-CF printed (typical)",
   each with E (GPa), density (kg/m³), yield/strength (MPa) and an editable "print knock-down" factor
   for printed materials (default 0.6 on strength, 0.8 on E). Tip payload (kg), optional joint/actuator
   mass at each link end (kg), gravity, and a pose angle (link elevation 0° = horizontal, 90° = vertical)
   that projects gravity onto the bending direction. Dynamic factor for acceleration (default 1.5).
2. **Model (core)**: second moment of area I and section modulus for each section (both axes),
   cross-section area, link self-weight as distributed load; tip deflection and slope by superposition
   (point loads + uniform load) through the serial chain (each link's root rotation/displacement adds
   rigid-body motion to later links); max bending moment at each link root, max bending stress,
   von Mises with shear ignored (document it), safety factor vs yield (with print knock-down);
   end-effector stiffness k = F/δ (N/mm); first natural frequency of the whole chain via Rayleigh's
   method using the static deflection shape (f = (1/2π)·sqrt(g·Σmδ / Σmδ²)) and the closed-form single
   cantilever check (1.875² ...) for the single-link case; total mass. Mark "fails" when SF < 1.
3. **Extras (small)**: (a) sweep: tip deflection and mass vs wall thickness (or OD) for link 1, curve
   with the current value marked; (b) "same stiffness, lighter?" readout: for link 1, the mass of
   each other material/section needed to match the current I·E (keep section shape, scale wall), list
   sorted by mass.
4. **3D view (WebGL2, game-grade — see below)**: the chain drawn as real 3D extrusions of the chosen
   sections (tube as ring mesh, rectangles as boxes) on a base mount, deflected shape exaggerated by a
   scale slider (1×-200×, default auto so the max sag looks ~10 % of length) with the undeformed chain
   as translucent ghost with bright edges; surface coloured by local bending stress on a
   blue→yellow→red ramp (red = at yield); payload as a box at the tip, actuator masses as cylinders;
   gravity arrow; checker ground with shadow.
5. **2D plots** (canvas 2D, own small module): deflection along the chain (mm vs position), bending
   moment along the chain, and the sweep curve from extra (a).
6. Korean + English toggle (browser default). Mobile friendly. CSS prefers-color-scheme only.
Out of scope (do NOT build): torsion modelling beyond a note, FEM, buckling, fatigue, composite layup
analysis, URL share, JSON/CSV import/export.

## 3D view requirements (game-like, reuse the existing renderer)
- `crates/lf_web/src/gl.rs` and `crates/lf_web/src/math.rs` already exist: a tool-independent WebGL2
  renderer (lit meshes, checker/grid ground with fog, sky gradient, stencil planar shadows, translucent
  volumes, lines) and pure math (mat4, damped orbit camera with inertia, ray-plane/ray-sphere picking)
  with unit tests. Reuse them; extend only if needed and keep them tool-independent and tested.
- Scene code in `view3d.rs`. Orbit (left drag) / pan (right drag, two fingers) / zoom (wheel, pinch)
  with inertia, double-click reset, `touch-action:none`. View presets: Perspective / Top / Side /
  Section (camera looks at the root cross-section).
- Direct manipulation: dragging the tip payload box vertically sets the payload (kg, 0-20) and
  re-runs the model live; dragging the pose handle at the base rotates the elevation angle; hover
  highlight + cursor change.
- Toggle buttons over the view: ghost (undeformed), stress colour, gravity arrow, exaggeration.
- Optional "ring" animation: play/pause a decaying oscillation of the tip at the computed first
  natural frequency (slow-motion 0.1× default) to show how it rings after a stop.
- HUD over the canvas: tip deflection mm, stiffness N/mm, max stress MPa, min SF, f1 Hz, total mass.
- If WebGL2 is unavailable, show a message and keep the numeric readouts and 2D plots working.
- Release wasm ≲ 350 KB. No crates besides wasm-bindgen, web-sys, js-sys.

## Core crate (pure Rust, no web deps, unit tested) — `crates/lf_core`
- Sections, materials, chain, loads, solver, metrics, sweep, equivalent-stiffness search.
- Tests (hand-checkable, 6-10): round-tube I = π(D⁴−d⁴)/64; rectangle I = bh³/12; single cantilever
  tip point load δ = FL³/3EI within 0.5 %; uniform load δ = wL⁴/8EI within 0.5 %; vertical pose
  (90°) → bending deflection ~0; two links with identical properties equal one link of 2L for a tip
  load within 1 %; single-link Rayleigh f1 with only a tip mass matches sqrt(3EI/(mL³))/2π within 2 %;
  SF scales inversely with payload when self-weight is zero; print knock-down lowers SF.
- `crates/lf_web`: UI, plain web-sys. The existing math.rs tests must keep passing.

## Repo hygiene
- README (Korean first, then English): what it is, model and assumptions (Euler–Bernoulli, small
  deflection, linear elastic, shear and torsion ignored, joints rigid unless noted), controls, local
  build (`cargo test`, `trunk build --release`), MIT license, "educational estimates — validate with
  real tests / FEM before trusting a robot with it".
- LICENSE: MIT, copyright "link-flex contributors" (exists). `.gitignore` for target/ dist/ (exists).
- `index.html` with a CSP meta that blocks external requests:
  `default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; connect-src 'self'; object-src 'none'; base-uri 'self'`.
  `Trunk.toml` exists (`filehash = false`, `public_url = "/link-flex/"`, external `web/boot.js`
  copied via `<link data-trunk rel="copy-file" href="web/boot.js">`, which imports `./lf_web.js`).
- `.github/workflows/pages.yml` exists (trunk 0.22.1, privacy check, cargo test, deploy-pages). Keep it.
- `tools/privacy_check.sh` already exists; keep it and keep it clean.

## Done when
- `cargo test` green (workspace); `trunk build --release` succeeds with no warnings;
  `sh tools/privacy_check.sh` clean.
- Commit author `link-flex contributors <59640857+roscore@users.noreply.github.com>`.
- Commit everything with clear commits; open a PR. Do NOT create new GitHub repos or change repo settings.
