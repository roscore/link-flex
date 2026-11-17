//! link-flex core: stiffness, deflection, stress and first bending mode of a serial chain of
//! cantilevered robot links. Pure Rust, SI units (m, kg, N, Pa), no web dependencies.
//!
//! Model: Euler–Bernoulli beams, small deflection, linear elastic, rigid joints. Each link is a
//! cantilever clamped to the (moving) tip of the previous one; its root displacement and rotation
//! add rigid-body motion to every later link (superposition). Shear deflection, shear stress and
//! torsion are ignored, so von Mises reduces to the bending stress |M|·c/I.

pub const G: f64 = 9.80665;

/// Cross-section, dimensions in metres. "h" is the dimension in the bending (gravity) direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Section {
    RoundTube { od: f64, wall: f64 },
    RectTube { w: f64, h: f64, wall: f64 },
    SolidRect { w: f64, h: f64 },
    /// Symmetric I-beam: flange width w, total height h, flange thickness tf, web thickness tw.
    IBeam { w: f64, h: f64, tf: f64, tw: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Props {
    pub area: f64,
    /// Second moment of area for bending in the h direction (about the horizontal axis).
    pub i_strong: f64,
    /// Second moment of area for bending in the w direction.
    pub i_weak: f64,
    /// Extreme fibre distances (h/2, w/2).
    pub c_strong: f64,
    pub c_weak: f64,
}

impl Props {
    pub fn z_strong(&self) -> f64 {
        self.i_strong / self.c_strong
    }
    pub fn z_weak(&self) -> f64 {
        self.i_weak / self.c_weak
    }
}

const TINY: f64 = 1e-6;

impl Section {
    /// Clamp dimensions to something physical (walls no thicker than solid).
    pub fn sanitized(self) -> Self {
        let p = |v: f64| v.max(TINY);
        match self {
            Section::RoundTube { od, wall } => Section::RoundTube { od: p(od), wall: p(wall).min(p(od) / 2.0) },
            Section::RectTube { w, h, wall } => {
                let (w, h) = (p(w), p(h));
                Section::RectTube { w, h, wall: p(wall).min(w.min(h) / 2.0) }
            }
            Section::SolidRect { w, h } => Section::SolidRect { w: p(w), h: p(h) },
            Section::IBeam { w, h, tf, tw } => {
                let (w, h) = (p(w), p(h));
                Section::IBeam { w, h, tf: p(tf).min(h / 2.0), tw: p(tw).min(w) }
            }
        }
    }

    /// Outer envelope (w, h).
    pub fn envelope(&self) -> (f64, f64) {
        match *self {
            Section::RoundTube { od, .. } => (od, od),
            Section::RectTube { w, h, .. } | Section::SolidRect { w, h } | Section::IBeam { w, h, .. } => (w, h),
        }
    }

    pub fn props(&self) -> Props {
        match self.sanitized() {
            Section::RoundTube { od, wall } => {
                let id = od - 2.0 * wall;
                let i = std::f64::consts::PI * (od.powi(4) - id.powi(4)) / 64.0;
                let a = std::f64::consts::PI * (od * od - id * id) / 4.0;
                Props { area: a, i_strong: i, i_weak: i, c_strong: od / 2.0, c_weak: od / 2.0 }
            }
            Section::RectTube { w, h, wall } => {
                let (wi, hi) = (w - 2.0 * wall, h - 2.0 * wall);
                Props {
                    area: w * h - wi * hi,
                    i_strong: (w * h.powi(3) - wi * hi.powi(3)) / 12.0,
                    i_weak: (h * w.powi(3) - hi * wi.powi(3)) / 12.0,
                    c_strong: h / 2.0,
                    c_weak: w / 2.0,
                }
            }
            Section::SolidRect { w, h } => Props {
                area: w * h,
                i_strong: w * h.powi(3) / 12.0,
                i_weak: h * w.powi(3) / 12.0,
                c_strong: h / 2.0,
                c_weak: w / 2.0,
            },
            Section::IBeam { w, h, tf, tw } => {
                let hw = h - 2.0 * tf;
                Props {
                    area: 2.0 * w * tf + hw * tw,
                    i_strong: (w * h.powi(3) - (w - tw) * hw.powi(3)) / 12.0,
                    i_weak: (2.0 * tf * w.powi(3) + hw * tw.powi(3)) / 12.0,
                    c_strong: h / 2.0,
                    c_weak: w / 2.0,
                }
            }
        }
    }
}

/// Material with typical (not datasheet) values and print knock-down factors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    /// Preset index into `PRESETS` (for labels only).
    pub preset: usize,
    pub e: f64,
    pub density: f64,
    /// Yield (metals) or strength (composites, printed polymers).
    pub strength: f64,
    pub printed: bool,
    /// Multipliers applied to strength and E for printed parts (1 for others).
    pub kd_strength: f64,
    pub kd_e: f64,
}

impl Material {
    pub fn e_eff(&self) -> f64 {
        self.e * if self.printed { self.kd_e } else { 1.0 }
    }
    pub fn strength_eff(&self) -> f64 {
        self.strength * if self.printed { self.kd_strength } else { 1.0 }
    }
}

/// Generic typical values. Ids are stable keys for UI labels.
pub const PRESET_IDS: [&str; 6] = ["al6061", "steel", "cfrp", "pla", "petg", "pacf"];

pub fn preset(i: usize) -> Material {
    // (E GPa, density kg/m³, strength MPa, printed)
    let t = [
        (69.0, 2700.0, 275.0, false),
        (200.0, 7850.0, 350.0, false),
        (50.0, 1550.0, 450.0, false),
        (3.5, 1240.0, 50.0, true),
        (2.1, 1270.0, 45.0, true),
        (7.0, 1150.0, 75.0, true),
    ][i.min(5)];
    Material {
        preset: i.min(5),
        e: t.0 * 1e9,
        density: t.1,
        strength: t.2 * 1e6,
        printed: t.3,
        kd_strength: if t.3 { 0.6 } else { 1.0 },
        kd_e: if t.3 { 0.8 } else { 1.0 },
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Link {
    pub length: f64,
    pub section: Section,
    pub material: Material,
    /// Bend about the weak axis (section turned 90°).
    pub rotated: bool,
    /// Joint / actuator mass lumped at the outer end of this link (kg).
    pub end_mass: f64,
}

impl Link {
    pub fn props(&self) -> Props {
        self.section.props()
    }
    /// Second moment and extreme fibre distance in the bending direction.
    pub fn bending(&self) -> (f64, f64) {
        let p = self.props();
        if self.rotated { (p.i_weak, p.c_weak) } else { (p.i_strong, p.c_strong) }
    }
    pub fn ei(&self) -> f64 {
        self.material.e_eff() * self.bending().0
    }
    pub fn mass(&self) -> f64 {
        self.material.density * self.props().area * self.length
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Loads {
    /// Tip payload (kg).
    pub payload: f64,
    /// Gravity (m/s²).
    pub g: f64,
    /// Link elevation (rad): 0 = horizontal, π/2 = vertical.
    pub elevation: f64,
    /// Multiplier on loads for stress / safety factor (acceleration allowance).
    pub dyn_factor: f64,
}

impl Default for Loads {
    fn default() -> Self {
        Loads { payload: 1.0, g: G, elevation: 0.0, dyn_factor: 1.5 }
    }
}

/// A point along the chain (static loads, no dynamic factor).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    /// Distance from the chain root (m).
    pub s: f64,
    pub link: usize,
    /// Local position within the link (m).
    pub x: f64,
    /// Transverse deflection (m, positive = sag along gravity).
    pub defl: f64,
    pub slope: f64,
    /// Bending moment (N·m, positive = sagging / hogging per load sign).
    pub moment: f64,
}

pub const SAMPLES_PER_LINK: usize = 40;

/// Transverse solve: `q[i]` uniform load on link i (N/m), `p[i]` point load at the end of link i (N).
pub fn solve(chain: &[Link], q: &[f64], p: &[f64], n: usize) -> Vec<Sample> {
    let k = chain.len();
    // Shear force and moment at each link's outer end from everything outboard.
    let (mut v_end, mut m_end) = (vec![0.0; k], vec![0.0; k]);
    let (mut v, mut m) = (0.0, 0.0);
    for i in (0..k).rev() {
        v += p[i];
        v_end[i] = v;
        m_end[i] = m;
        let l = chain[i].length;
        m += v * l + q[i] * l * l / 2.0;
        v += q[i] * l;
    }
    let mut out = Vec::with_capacity(k * (n + 1));
    let (mut y0, mut th0, mut s0) = (0.0, 0.0, 0.0);
    for (i, link) in chain.iter().enumerate() {
        let (l, ei) = (link.length, link.ei().max(1e-12));
        let (mm, pp, qq) = (m_end[i], v_end[i], q[i]);
        let local = |x: f64| {
            let d = mm * x * x / 2.0 + pp * x * x * (3.0 * l - x) / 6.0 + qq * x * x * (6.0 * l * l - 4.0 * l * x + x * x) / 24.0;
            let t = mm * x + pp * x * (2.0 * l - x) / 2.0 + qq * x * (3.0 * l * l - 3.0 * l * x + x * x) / 6.0;
            (d / ei, t / ei)
        };
        for j in 0..=n {
            let x = l * j as f64 / n as f64;
            let (d, t) = local(x);
            out.push(Sample {
                s: s0 + x,
                link: i,
                x,
                defl: y0 + th0 * x + d,
                slope: th0 + t,
                moment: mm + pp * (l - x) + qq * (l - x) * (l - x) / 2.0,
            });
        }
        let (d, t) = local(l);
        y0 += th0 * l + d;
        th0 += t;
        s0 += l;
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinkResult {
    pub mass: f64,
    /// Static bending moment at the link root (N·m).
    pub root_moment: f64,
    /// Peak design bending stress = von Mises with shear ignored, × dynamic factor (Pa).
    pub max_stress: f64,
    pub sf: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Results {
    pub samples: Vec<Sample>,
    /// Static tip deflection (m) and slope (rad).
    pub tip_defl: f64,
    pub tip_slope: f64,
    pub links: Vec<LinkResult>,
    /// End-effector stiffness F/δ for a transverse tip force (N/m).
    pub stiffness: f64,
    /// First bending frequency, Rayleigh with the static deflection shape (Hz).
    pub f1: f64,
    /// Closed-form single cantilever check (single-link chains only).
    pub f1_closed: Option<f64>,
    /// Links + joint masses, payload excluded (kg).
    pub total_mass: f64,
    pub min_sf: f64,
    pub fails: bool,
}

fn transverse_loads(chain: &[Link], payload: f64, gt: f64) -> (Vec<f64>, Vec<f64>) {
    let q = chain.iter().map(|l| l.material.density * l.props().area * gt).collect();
    let mut p: Vec<f64> = chain.iter().map(|l| l.end_mass * gt).collect();
    if let Some(last) = p.last_mut() {
        *last += payload * gt;
    }
    (q, p)
}

/// Rayleigh: f = (1/2π)·sqrt(g·Σmδ / Σmδ²) with δ the static deflection under the masses' weight.
pub fn rayleigh_f1(chain: &[Link], payload: f64) -> f64 {
    let n = SAMPLES_PER_LINK;
    let (q, p) = transverse_loads(chain, payload, G);
    let smp = solve(chain, &q, &p, n);
    let (mut a, mut b) = (0.0, 0.0);
    let mut add = |m: f64, d: f64| {
        a += m * d;
        b += m * d * d;
    };
    for (i, link) in chain.iter().enumerate() {
        let seg = &smp[i * (n + 1)..(i + 1) * (n + 1)];
        let dm = link.mass() / n as f64;
        for w in seg.windows(2) {
            add(dm, (w[0].defl + w[1].defl) / 2.0);
        }
        add(p[i] / G, seg[n].defl);
    }
    if b <= 0.0 { 0.0 } else { (G * a / b).sqrt() / std::f64::consts::TAU }
}

pub fn analyze(chain: &[Link], loads: &Loads) -> Results {
    let n = SAMPLES_PER_LINK;
    let gt = loads.g * loads.elevation.cos();
    let (q, p) = transverse_loads(chain, loads.payload, gt);
    let samples = solve(chain, &q, &p, n);
    let tip = *samples.last().expect("at least one link");

    let links: Vec<LinkResult> = chain
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let (ib, c) = l.bending();
            let mmax = samples.iter().filter(|s| s.link == i).map(|s| s.moment.abs()).fold(0.0, f64::max);
            let stress = mmax * c / ib * loads.dyn_factor;
            let sf = if stress > 0.0 { l.material.strength_eff() / stress } else { f64::INFINITY };
            LinkResult { mass: l.mass(), root_moment: samples[i * (n + 1)].moment, max_stress: stress, sf }
        })
        .collect();

    let mut unit = vec![0.0; chain.len()];
    *unit.last_mut().unwrap() = 1.0;
    let d1 = solve(chain, &vec![0.0; chain.len()], &unit, 4).last().unwrap().defl;

    let f1_closed = (chain.len() == 1).then(|| {
        let l = &chain[0];
        let tip_mass = l.end_mass + loads.payload;
        let ei = l.ei();
        if tip_mass <= 0.0 {
            let mu = l.material.density * l.props().area;
            1.875104f64.powi(2) / std::f64::consts::TAU * (ei / (mu * l.length.powi(4))).sqrt()
        } else {
            // Tip mass plus 33/140 of the beam mass (classic Rayleigh cantilever result).
            (3.0 * ei / ((tip_mass + 0.2357 * l.mass()) * l.length.powi(3))).sqrt() / std::f64::consts::TAU
        }
    });

    let min_sf = links.iter().map(|r| r.sf).fold(f64::INFINITY, f64::min);
    Results {
        tip_defl: tip.defl,
        tip_slope: tip.slope,
        samples,
        stiffness: if d1 > 0.0 { 1.0 / d1 } else { f64::INFINITY },
        f1: rayleigh_f1(chain, loads.payload),
        f1_closed,
        total_mass: chain.iter().map(|l| l.mass() + l.end_mass).sum(),
        min_sf,
        fails: min_sf < 1.0,
        links,
    }
}

/// The sweep parameter of a section: wall thickness for tubes / I-beams (flange), height for solids.
pub fn sweep_param(s: &Section) -> f64 {
    match *s {
        Section::RoundTube { wall, .. } | Section::RectTube { wall, .. } => wall,
        Section::IBeam { tf, .. } => tf,
        Section::SolidRect { h, .. } => h,
    }
}

/// Section with its sweep parameter set to `v` (I-beam web scales with the flange).
pub fn with_param(s: &Section, v: f64) -> Section {
    match *s {
        Section::RoundTube { od, .. } => Section::RoundTube { od, wall: v },
        Section::RectTube { w, h, .. } => Section::RectTube { w, h, wall: v },
        Section::IBeam { w, h, tf, tw } => Section::IBeam { w, h, tf: v, tw: tw * v / tf.max(TINY) },
        Section::SolidRect { w, .. } => Section::SolidRect { w, h: v },
    }
    .sanitized()
}

fn param_max(s: &Section) -> f64 {
    match *s {
        Section::RoundTube { od, .. } => od / 2.0,
        Section::RectTube { w, h, .. } => w.min(h) / 2.0,
        Section::IBeam { h, .. } => h / 2.0,
        Section::SolidRect { h, .. } => h * 2.0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SweepPoint {
    pub x: f64,
    pub tip_defl: f64,
    pub mass: f64,
}

/// Tip deflection and chain mass vs link 1's sweep parameter. Returns points and the current value.
pub fn sweep(chain: &[Link], loads: &Loads, n: usize) -> (Vec<SweepPoint>, f64) {
    let s0 = chain[0].section.sanitized();
    let cur = sweep_param(&s0);
    let lo = if matches!(s0, Section::SolidRect { .. }) { cur * 0.5 } else { (cur * 0.25).max(2e-4) };
    let hi = (cur * 3.0).min(param_max(&s0)).max(lo * 1.001);
    let mut c = chain.to_vec();
    let pts = (0..n)
        .map(|k| {
            let x = lo + (hi - lo) * k as f64 / (n - 1).max(1) as f64;
            c[0].section = with_param(&s0, x);
            let r = analyze(&c, loads);
            SweepPoint { x, tip_defl: r.tip_defl, mass: r.total_mass }
        })
        .collect();
    (pts, cur)
}

pub const KIND_COUNT: usize = 4;

pub fn kind(s: &Section) -> usize {
    match s {
        Section::RoundTube { .. } => 0,
        Section::RectTube { .. } => 1,
        Section::SolidRect { .. } => 2,
        Section::IBeam { .. } => 3,
    }
}

/// Section of `kind` inside the envelope (w, h) with wall parameter t (ignored for solids).
pub fn make_section(kind: usize, w: f64, h: f64, t: f64) -> Section {
    match kind {
        0 => Section::RoundTube { od: h, wall: t },
        1 => Section::RectTube { w, h, wall: t },
        2 => Section::SolidRect { w, h },
        _ => Section::IBeam { w, h, tf: t, tw: t },
    }
    .sanitized()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Equivalent {
    pub material: Material,
    pub section: Section,
    /// Mass of the replacement link (kg).
    pub mass: f64,
}

/// For each material × section shape: the section matching `link`'s E·I in its bending direction,
/// keeping the outer envelope and scaling the wall (or, if even a solid is too soft / for solids,
/// scaling the whole section). Sorted by mass, lightest first.
pub fn equivalents(link: &Link, materials: &[Material]) -> Vec<Equivalent> {
    let target = link.ei();
    let (mut w, mut h) = link.section.sanitized().envelope();
    if link.rotated {
        std::mem::swap(&mut w, &mut h);
    }
    let bend_i = |s: &Section| s.props().i_strong;
    let mut out = vec![];
    for m in materials {
        let need = target / m.e_eff();
        for k in 0..KIND_COUNT {
            let tmax = w.min(h) / 2.0;
            let mut s = make_section(k, w, h, tmax);
            if k != 2 && bend_i(&s) >= need {
                let (mut a, mut b) = (0.0, tmax);
                for _ in 0..60 {
                    let mid = (a + b) / 2.0;
                    if bend_i(&make_section(k, w, h, mid)) < need { a = mid } else { b = mid }
                }
                s = make_section(k, w, h, b);
            } else {
                let f = (need / bend_i(&s)).powf(0.25);
                s = make_section(k, w * f, h * f, tmax * f);
            }
            let mass = m.density * s.props().area * link.length;
            out.push(Equivalent { material: *m, section: s, mass });
        }
    }
    out.sort_by(|a, b| a.mass.total_cmp(&b.mass));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn rel(a: f64, b: f64) -> f64 {
        ((a - b) / b).abs()
    }
    fn massless(mut m: Material) -> Material {
        m.density = 0.0;
        m
    }
    fn link(len: f64, sec: Section, mat: Material) -> Link {
        Link { length: len, section: sec, material: mat, rotated: false, end_mass: 0.0 }
    }
    fn tube() -> Section {
        Section::RoundTube { od: 0.03, wall: 0.002 }
    }

    #[test]
    fn round_tube_inertia() {
        let (d, di) = (0.03f64, 0.026f64);
        let p = tube().props();
        assert!(rel(p.i_strong, PI * (d.powi(4) - di.powi(4)) / 64.0) < 1e-12);
        assert!(rel(p.area, PI * (d * d - di * di) / 4.0) < 1e-12);
        assert_eq!(p.i_strong, p.i_weak);
    }

    #[test]
    fn rectangle_inertia_both_axes() {
        let p = Section::SolidRect { w: 0.02, h: 0.05 }.props();
        assert!(rel(p.i_strong, 0.02 * 0.05f64.powi(3) / 12.0) < 1e-12);
        assert!(rel(p.i_weak, 0.05 * 0.02f64.powi(3) / 12.0) < 1e-12);
        assert!(rel(p.z_strong(), 0.02 * 0.05 * 0.05 / 6.0) < 1e-12);
        // A rectangular tube with a wall of half the size is the solid rectangle.
        let t = Section::RectTube { w: 0.02, h: 0.05, wall: 0.01 }.props();
        assert!(rel(t.i_strong, p.i_strong) < 1e-12);
    }

    #[test]
    fn tip_point_load_matches_fl3_3ei() {
        let l = link(0.5, tube(), massless(preset(0)));
        let loads = Loads { payload: 2.0, ..Loads::default() };
        let r = analyze(&[l], &loads);
        let f = 2.0 * G;
        assert!(rel(r.tip_defl, f * 0.125 / (3.0 * l.ei())) < 0.005);
        assert!(rel(r.stiffness, 3.0 * l.ei() / 0.125) < 0.005);
        assert!(rel(r.links[0].root_moment, f * 0.5) < 1e-9);
    }

    #[test]
    fn uniform_load_matches_wl4_8ei() {
        let l = link(0.8, tube(), preset(1));
        let r = analyze(&[l], &Loads { payload: 0.0, ..Loads::default() });
        let w = l.material.density * l.props().area * G;
        assert!(rel(r.tip_defl, w * 0.8f64.powi(4) / (8.0 * l.ei())) < 0.005);
    }

    #[test]
    fn vertical_pose_has_no_bending() {
        let l = Link { end_mass: 0.5, ..link(0.4, tube(), preset(3)) };
        let r = analyze(&[l, l], &Loads { elevation: PI / 2.0, ..Loads::default() });
        assert!(r.tip_defl.abs() < 1e-12);
        assert!(r.min_sf > 1e9);
    }

    #[test]
    fn two_identical_links_equal_one_long_link() {
        let l = link(0.3, tube(), massless(preset(0)));
        let loads = Loads::default();
        let two = analyze(&[l, l], &loads);
        let one = analyze(&[Link { length: 0.6, ..l }], &loads);
        assert!(rel(two.tip_defl, one.tip_defl) < 0.01);
        assert!(rel(two.f1, one.f1) < 0.01);
        // With self-weight too.
        let lw = link(0.3, tube(), preset(2));
        let a = analyze(&[lw, lw], &loads).tip_defl;
        let b = analyze(&[Link { length: 0.6, ..lw }], &loads).tip_defl;
        assert!(rel(a, b) < 0.01);
    }

    #[test]
    fn rayleigh_single_tip_mass() {
        let l = link(0.5, tube(), massless(preset(0)));
        let m = 1.5;
        let r = analyze(&[l], &Loads { payload: m, ..Loads::default() });
        let exact = (3.0 * l.ei() / (m * 0.125)).sqrt() / (2.0 * PI);
        assert!(rel(r.f1, exact) < 0.02);
        assert!(rel(r.f1_closed.unwrap(), exact) < 1e-9);
        // Distributed mass only: Rayleigh with the static shape is within ~1 % of 1.875².
        let lw = link(0.5, tube(), preset(0));
        let r = analyze(&[lw], &Loads { payload: 0.0, ..Loads::default() });
        assert!(rel(r.f1, r.f1_closed.unwrap()) < 0.02);
    }

    #[test]
    fn sf_inverse_with_payload_and_knockdown_lowers_it() {
        let l = link(0.4, Section::SolidRect { w: 0.01, h: 0.02 }, massless(preset(3)));
        let sf = |kg: f64, l: Link| analyze(&[l], &Loads { payload: kg, ..Loads::default() }).min_sf;
        assert!(rel(sf(1.0, l) / sf(3.0, l), 3.0) < 1e-9);
        let mut strong = l;
        strong.material.kd_strength = 1.0;
        assert!(sf(1.0, l) < sf(1.0, strong));
        assert!(rel(sf(1.0, strong) * 0.6, sf(1.0, l)) < 1e-9);
        assert!(analyze(&[l], &Loads { payload: 200.0, ..Loads::default() }).fails);
    }

    #[test]
    fn equivalents_match_stiffness() {
        let l = link(0.5, tube(), preset(0));
        let mats: Vec<Material> = (0..6).map(preset).collect();
        let eq = equivalents(&l, &mats);
        assert_eq!(eq.len(), 24);
        for e in &eq {
            let ei = e.material.e_eff() * e.section.props().i_strong;
            assert!(rel(ei, l.ei()) < 1e-3, "{e:?}");
        }
        assert!(eq.windows(2).all(|w| w[0].mass <= w[1].mass));
        // The same material and shape needs the same wall.
        let same = eq.iter().find(|e| e.material.preset == 0 && kind(&e.section) == 0).unwrap();
        assert!(rel(same.mass, l.mass()) < 1e-3);
    }

    #[test]
    fn sweep_thicker_wall_is_stiffer_and_heavier() {
        let l = link(0.5, tube(), preset(0));
        let (pts, cur) = sweep(&[l], &Loads::default(), 12);
        assert!((cur - 0.002).abs() < 1e-12);
        assert!(pts.first().unwrap().x <= cur && pts.last().unwrap().x >= cur);
        assert!(pts.windows(2).all(|w| w[1].mass > w[0].mass));
        assert!(pts.first().unwrap().tip_defl > pts.last().unwrap().tip_defl);
    }
}
