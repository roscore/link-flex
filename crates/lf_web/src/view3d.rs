//! 3D scene: the link chain as extruded sections on a base mount, deflected shape with stress
//! colours, ghost of the undeformed chain, payload / actuators, gravity arrow, picking and drags.
use crate::gl::{self, Geo, Mesh, Paint, Renderer};
use crate::math::{self, Orbit, OrbitCam, M4, V};
use lf_core::{Link, Loads, Results, Section};
use web_sys::HtmlCanvasElement;

pub struct Scene<'a> {
    pub chain: &'a [Link],
    pub res: &'a Results,
    pub loads: &'a Loads,
}

pub enum Edit {
    Payload(f64),
    Elevation(f64),
}

#[derive(Clone, Copy, PartialEq)]
enum Drag {
    None,
    Orbit,
    Pan,
    Pinch,
    Payload { kg0: f64, y0: f32, n: V },
    Pose,
}

const FOV: f32 = 0.8;
const ELEV_MIN: f64 = -30.0;

struct Prof {
    loops: Vec<(Vec<[f32; 2]>, bool)>,
    caps: Vec<[[f32; 2]; 4]>,
    c: f32,
}

fn circle(r: f32, n: usize) -> Vec<[f32; 2]> {
    (0..n).map(|i| {
        let (s, c) = (i as f32 / n as f32 * std::f32::consts::TAU).sin_cos();
        [r * c, r * s]
    }).collect()
}
fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<[f32; 2]> {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}
fn ring_caps(o: &[[f32; 2]], i: &[[f32; 2]]) -> Vec<[[f32; 2]; 4]> {
    (0..o.len()).map(|k| {
        let j = (k + 1) % o.len();
        [o[k], o[j], i[j], i[k]]
    }).collect()
}

fn profile(s: &Section, rotated: bool) -> Prof {
    let mut p = match s.sanitized() {
        Section::RoundTube { od, wall } => {
            let (ro, ri) = (od as f32 / 2.0, (od / 2.0 - wall) as f32);
            let (o, i) = (circle(ro, 28), circle(ri, 28));
            let caps = ring_caps(&o, &i);
            let mut loops = vec![(o, true)];
            if ri > 1e-6 {
                loops.push((i.into_iter().rev().collect(), true));
            }
            Prof { loops, caps, c: 0.0 }
        }
        Section::RectTube { w, h, wall } => {
            let (w, h, t) = (w as f32 / 2.0, h as f32 / 2.0, wall as f32);
            let (o, i) = (rect(-w, -h, w, h), rect(-w + t, -h + t, w - t, h - t));
            let caps = ring_caps(&o, &i);
            let mut loops = vec![(o, false)];
            if w - t > 1e-6 && h - t > 1e-6 {
                loops.push((i.into_iter().rev().collect(), false));
            }
            Prof { loops, caps, c: 0.0 }
        }
        Section::SolidRect { w, h } => {
            let o = rect(-w as f32 / 2.0, -h as f32 / 2.0, w as f32 / 2.0, h as f32 / 2.0);
            Prof { caps: vec![[o[0], o[1], o[2], o[3]]], loops: vec![(o, false)], c: 0.0 }
        }
        Section::IBeam { w, h, tf, tw } => {
            let (w, h, f, t) = (w as f32 / 2.0, h as f32 / 2.0, tf as f32, tw as f32 / 2.0);
            let o = vec![[-w, -h], [w, -h], [w, -h + f], [t, -h + f], [t, h - f], [w, h - f], [w, h], [-w, h], [-w, h - f], [-t, h - f], [-t, -h + f], [-w, -h + f]];
            let caps = [rect(-w, -h, w, -h + f), rect(-w, h - f, w, h), rect(-t, -h + f, t, h - f)].map(|r| [r[0], r[1], r[2], r[3]]).to_vec();
            Prof { loops: vec![(o, false)], caps, c: 0.0 }
        }
    };
    if rotated {
        let sw = |q: &mut [f32; 2]| q.swap(0, 1);
        p.loops.iter_mut().for_each(|(l, _)| l.iter_mut().for_each(sw));
        p.caps.iter_mut().for_each(|c| c.iter_mut().for_each(sw));
    }
    p.c = p.loops[0].0.iter().map(|q| q[1].abs()).fold(1e-9, f32::max);
    p
}

/// blue → yellow → red for 0 … 1 (red = at the knocked-down strength with the dynamic factor).
pub fn ramp(r: f32) -> V {
    let r = r.clamp(0.0, 1.0);
    let (a, b, t) = if r < 0.5 { ([0.15, 0.35, 0.95], [1.0, 0.9, 0.15], r * 2.0) } else { ([1.0, 0.9, 0.15], [0.95, 0.1, 0.1], r * 2.0 - 1.0) };
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

#[derive(Clone, Copy)]
struct Frame {
    p: V,
    t: V,
    n: V,
    ratio: f32,
}

const Z: V = [0.0, 0.0, 1.0];

fn push(g: &mut Geo, p: V, n: V, c: V) -> u16 {
    g.pos.extend(p);
    g.nrm.extend(n);
    g.col.extend(c);
    (g.pos.len() / 3 - 1) as u16
}

fn at(f: &Frame, q: [f32; 2]) -> V {
    math::add(f.p, math::add(math::scale(Z, q[0]), math::scale(f.n, q[1])))
}

/// Extrude the profile along the frames; colour by fibre stress if `stress`.
fn extrude(prof: &Prof, frames: &[Frame], stress: bool) -> Geo {
    let mut g = Geo::default();
    let col = |f: &Frame, q: [f32; 2]| if stress { ramp(f.ratio * q[1].abs() / prof.c) } else { [0.72, 0.75, 0.80] };
    let en = |a: [f32; 2], b: [f32; 2]| {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let l = (dx * dx + dy * dy).sqrt().max(1e-9);
        [dy / l, -dx / l]
    };
    for (lp, smooth) in &prof.loops {
        let m = lp.len();
        for k in 0..m {
            let (a, b) = (lp[k], lp[(k + 1) % m]);
            let ne = en(a, b);
            let vn = |i: usize| {
                let (p0, p1, p2) = (lp[(i + m - 1) % m], lp[i], lp[(i + 1) % m]);
                let (e0, e1) = (en(p0, p1), en(p1, p2));
                let s = [e0[0] + e1[0], e0[1] + e1[1]];
                let l = (s[0] * s[0] + s[1] * s[1]).sqrt().max(1e-9);
                [s[0] / l, s[1] / l]
            };
            let (na, nb) = if *smooth { (vn(k), vn((k + 1) % m)) } else { (ne, ne) };
            let n3 = |f: &Frame, q: [f32; 2]| math::add(math::scale(Z, q[0]), math::scale(f.n, q[1]));
            let mut prev: Option<(u16, u16)> = None;
            for f in frames {
                let i0 = push(&mut g, at(f, a), n3(f, na), col(f, a));
                let i1 = push(&mut g, at(f, b), n3(f, nb), col(f, b));
                if let Some((p0, p1)) = prev {
                    g.idx.extend([p0, p1, i1, p0, i1, i0]);
                }
                prev = Some((i0, i1));
            }
        }
    }
    for (f, s) in [(frames[0], -1.0), (frames[frames.len() - 1], 1.0)] {
        for q in &prof.caps {
            let i = q.map(|c| push(&mut g, at(&f, c), math::scale(f.t, s), col(&f, c)));
            g.idx.extend([i[0], i[1], i[2], i[0], i[2], i[3]]);
        }
    }
    g
}

pub struct View {
    r: Renderer,
    canvas: HtmlCanvasElement,
    pub cam: OrbitCam,
    cube: Mesh,
    cyl: Mesh,
    sph: Mesh,
    head: Mesh,
    links: Vec<Mesh>,
    ghost: Vec<Mesh>,
    edges: Option<Mesh>,
    pub ghost_on: bool,
    pub stress_on: bool,
    pub grav_on: bool,
    pub exag_on: bool,
    pub ex_auto: bool,
    pub ex_val: f32,
    /// Exaggeration actually used for the last build.
    pub ex_used: f32,
    pub ring: bool,
    ring_t: f32,
    section: bool,
    pub dirty: bool,
    drag: Drag,
    ptrs: Vec<(i32, f32, f32)>,
    hover: u8,
    inv_vp: M4,
    // Layout of the last build (world units = metres).
    root: V,
    ltot: f32,
    hub: f32,
    tip_box: (V, f32),
    handle: (V, f32),
    joints: Vec<(V, f32)>,
    theta: f32,
}

impl View {
    pub fn new(canvas: HtmlCanvasElement) -> Option<Self> {
        let r = Renderer::new(&canvas)?;
        let head = gl::frustum([0.0, -0.5, 0.0], &[[-0.5, 0.5, -0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5]]);
        Some(View {
            cube: r.mesh(&gl::cube())?,
            cyl: r.mesh(&gl::cylinder(28))?,
            sph: r.mesh(&gl::sphere(20))?,
            head: r.mesh(&head)?,
            r,
            canvas,
            cam: OrbitCam::new(Orbit { target: [0.0; 3], yaw: 0.6, pitch: 0.3, dist: 2.0 }),
            links: vec![],
            ghost: vec![],
            edges: None,
            ghost_on: true,
            stress_on: true,
            grav_on: true,
            exag_on: true,
            ex_auto: true,
            ex_val: 50.0,
            ex_used: 1.0,
            ring: false,
            ring_t: 0.0,
            section: false,
            dirty: true,
            drag: Drag::None,
            ptrs: vec![],
            hover: 0,
            inv_vp: math::IDENTITY,
            root: [0.0; 3],
            ltot: 1.0,
            hub: 0.02,
            tip_box: ([0.0; 3], 0.0),
            handle: ([0.0; 3], 0.0),
            joints: vec![],
            theta: 0.0,
        })
    }

    fn layout(&mut self, sc: &Scene) {
        let ltot: f64 = sc.chain.iter().map(|l| l.length).sum();
        self.ltot = ltot.max(1e-3) as f32;
        let (w, h) = sc.chain[0].section.envelope();
        self.hub = (w.max(h) as f32 * 0.8).max(0.012 * self.ltot);
        self.root = [0.0, 0.6 * self.ltot + 2.0 * self.hub, 0.0];
        self.theta = sc.loads.elevation as f32;
    }

    /// Camera presets: 0 perspective, 1 top, 2 side, 3 section (looks at the root cross-section).
    pub fn preset(&mut self, k: usize, sc: &Scene) {
        self.layout(sc);
        let (s, c) = self.theta.sin_cos();
        let mid = math::add(self.root, [c * self.ltot / 2.0, s * self.ltot / 2.0 - 0.2 * self.ltot, 0.0]);
        let d = 1.6 * self.ltot;
        self.section = k == 3;
        let o = match k {
            1 => Orbit { target: mid, yaw: 0.0, pitch: 1.5, dist: d * 1.1 },
            2 => Orbit { target: mid, yaw: 0.0, pitch: 0.02, dist: d },
            3 => {
                let (w, h) = sc.chain[0].section.envelope();
                let cut = self.cut_x(sc) as f32;
                let t = math::add(self.root, [c * cut, s * cut, 0.0]);
                Orbit { target: t, yaw: std::f32::consts::FRAC_PI_2, pitch: self.theta.clamp(math::PITCH_MIN, math::PITCH_MAX), dist: 5.0 * w.max(h) as f32 }
            }
            _ => Orbit { target: mid, yaw: 0.55, pitch: 0.3, dist: d },
        };
        self.cam.set_goal(o);
        self.dirty = true;
    }

    fn cut_x(&self, sc: &Scene) -> f64 {
        sc.chain[0].length * 3.0 / lf_core::SAMPLES_PER_LINK as f64
    }

    fn free_all(&mut self) {
        for m in self.links.drain(..).chain(self.ghost.drain(..)).chain(self.edges.take()) {
            self.r.free(m);
        }
    }

    fn build(&mut self, sc: &Scene) {
        self.layout(sc);
        self.free_all();
        let res = sc.res;
        let maxd = res.samples.iter().map(|s| s.defl.abs()).fold(0.0, f64::max) as f32;
        let auto = if maxd > 1e-12 { (0.1 * self.ltot / maxd).clamp(1.0, 200.0) } else { 200.0 };
        self.ex_used = if !self.exag_on { 1.0 } else if self.ex_auto { auto } else { self.ex_val.clamp(1.0, 200.0) };
        let f1 = res.f1 as f32;
        let mut amp = 1.0;
        if self.ring && f1.is_finite() && f1 > 0.0 {
            let w = std::f32::consts::TAU * f1;
            let env = (-0.03 * w * self.ring_t).exp();
            if env < 0.01 {
                self.ring_t = 0.0;
            }
            amp = 1.0 + (sc.loads.dyn_factor as f32 - 1.0).max(0.5) * env * (w * self.ring_t).cos();
        }
        let (s, c) = self.theta.sin_cos();
        let (u, n0) = ([c, s, 0.0], [-s, c, 0.0]);
        let root = self.root;
        let frames = |ex: f32, i: usize| -> Vec<Frame> {
            let l = &sc.chain[i];
            let (ib, cc) = l.bending();
            let k = (cc / ib * sc.loads.dyn_factor / l.material.strength_eff()) as f32;
            res.samples.iter().filter(|q| q.link == i).map(|q| {
                let p = math::add(root, math::add(math::scale(u, q.s as f32), math::scale(n0, -(q.defl as f32) * ex)));
                let t = math::norm(math::sub(u, math::scale(n0, q.slope as f32 * ex)));
                Frame { p, t, n: math::cross(Z, t), ratio: (q.moment.abs() as f32 * k).min(1e3) }
            }).collect()
        };
        let ex = self.ex_used * amp;
        let mut edges = vec![];
        self.joints.clear();
        for (i, l) in sc.chain.iter().enumerate() {
            let prof = profile(&l.section, l.rotated);
            let mut fr = frames(ex, i);
            let (w, h) = l.section.envelope();
            let end = fr[fr.len() - 1];
            self.joints.push((end.p, (w.max(h) as f32 * 0.75).max(0.01 * self.ltot)));
            if self.section {
                if i == 0 {
                    fr.truncate(4);
                    if let Some(m) = self.r.mesh(&extrude(&prof, &fr, self.stress_on)) {
                        self.links.push(m);
                    }
                }
                continue;
            }
            if let Some(m) = self.r.mesh(&extrude(&prof, &fr, self.stress_on)) {
                self.links.push(m);
            }
            let gf = frames(0.0, i);
            if let Some(m) = self.r.mesh(&extrude(&prof, &gf, false)) {
                self.ghost.push(m);
            }
            let (a, b) = (gf[0], gf[gf.len() - 1]);
            let o = &prof.loops[0].0;
            let step = if o.len() > 12 { 7 } else { 1 };
            for k in 0..o.len() {
                let q = (o[k], o[(k + 1) % o.len()]);
                edges.extend([at(&a, q.0), at(&a, q.1), at(&b, q.0), at(&b, q.1)]);
                if k % step == 0 {
                    edges.extend([at(&a, q.0), at(&b, q.0)]);
                }
            }
        }
        self.edges = self.r.mesh(&gl::lines(&edges));
        let tip = self.joints.last().map(|j| j.0).unwrap_or(self.root);
        let b = self.ltot * (0.05 + 0.035 * (sc.loads.payload.max(0.0) as f32).cbrt());
        let cs = self.joints.last().map(|j| j.1).unwrap_or(0.0);
        self.tip_box = (math::sub(tip, [0.0, b / 2.0 + cs * 0.7, 0.0]), b);
        self.handle = (math::add(self.root, math::scale(n0, self.hub * 3.2)), self.hub * 0.8);
        self.dirty = false;
    }

    fn ray(&self, x: f32, y: f32) -> (V, V) {
        let (w, h) = (self.canvas.client_width().max(1) as f32, self.canvas.client_height().max(1) as f32);
        math::ndc_ray(&self.inv_vp, 2.0 * x / w - 1.0, 1.0 - 2.0 * y / h)
    }

    fn pick(&self, x: f32, y: f32) -> u8 {
        let (o, d) = self.ray(x, y);
        let a = math::ray_sphere(o, d, self.tip_box.0, self.tip_box.1 * 0.8);
        let b = if self.section { None } else { math::ray_sphere(o, d, self.handle.0, self.handle.1 * 1.3) };
        match (a, b) {
            (Some(a), Some(b)) => if a < b { 1 } else { 2 },
            (Some(_), None) => 1,
            (None, Some(_)) => 2,
            _ => 0,
        }
    }

    pub fn cursor(&self) -> &'static str {
        match (self.drag, self.hover) {
            (Drag::Payload { .. }, _) => "ns-resize",
            (Drag::Pose | Drag::Orbit | Drag::Pan, _) => "grabbing",
            (_, 1) => "ns-resize",
            (_, 2) => "grab",
            _ => "default",
        }
    }

    pub fn pointer_down(&mut self, id: i32, x: f32, y: f32, button: i16, payload: f64) {
        self.ptrs.retain(|p| p.0 != id);
        self.ptrs.push((id, x, y));
        if self.ptrs.len() >= 2 {
            self.drag = Drag::Pinch;
        } else if button == 2 {
            self.drag = Drag::Pan;
        } else {
            self.drag = match self.pick(x, y) {
                1 => {
                    let eye = self.cam.cur.eye();
                    let mut n = math::sub(eye, self.tip_box.0);
                    n[1] = 0.0;
                    let n = if math::dot(n, n) < 1e-9 { Z } else { math::norm(n) };
                    let (o, d) = self.ray(x, y);
                    let y0 = math::ray_plane(o, d, self.tip_box.0, n).map(|p| p[1]).unwrap_or(self.tip_box.0[1]);
                    Drag::Payload { kg0: payload, y0, n }
                }
                2 => Drag::Pose,
                _ => Drag::Orbit,
            };
        }
        self.cam.held = matches!(self.drag, Drag::Orbit | Drag::Pan | Drag::Pinch);
    }

    pub fn pointer_up(&mut self, id: i32) {
        self.ptrs.retain(|p| p.0 != id);
        if self.ptrs.is_empty() {
            self.drag = Drag::None;
            self.cam.held = false;
        } else if self.drag == Drag::Pinch {
            self.drag = Drag::Pan;
        }
    }

    pub fn pointer_move(&mut self, id: i32, x: f32, y: f32) -> Option<Edit> {
        let Some(k) = self.ptrs.iter().position(|p| p.0 == id) else {
            let h = self.pick(x, y);
            if h != self.hover {
                self.hover = h;
                self.dirty = true;
            }
            return None;
        };
        let (_, px, py) = self.ptrs[k];
        let (dx, dy) = (x - px, y - py);
        let old = self.ptrs.clone();
        self.ptrs[k] = (id, x, y);
        let ch = self.canvas.client_height().max(1) as f32;
        let pan_scale = 2.0 * self.cam.cur.dist * (FOV / 2.0).tan() / ch;
        match self.drag {
            Drag::Orbit => self.cam.rotate(-dx * 0.008, dy * 0.008),
            Drag::Pan => self.pan(dx, dy, pan_scale),
            Drag::Pinch if old.len() >= 2 => {
                let (a0, b0) = (old[0], old[1]);
                let (a1, b1) = (self.ptrs[0], self.ptrs[1]);
                let dist = |a: (i32, f32, f32), b: (i32, f32, f32)| ((a.1 - b.1).powi(2) + (a.2 - b.2).powi(2)).sqrt().max(1.0);
                self.cam.zoom(dist(a0, b0) / dist(a1, b1));
                self.pan(((a1.1 + b1.1) - (a0.1 + b0.1)) / 2.0, ((a1.2 + b1.2) - (a0.2 + b0.2)) / 2.0, pan_scale);
            }
            Drag::Payload { kg0, y0, n } => {
                let (o, d) = self.ray(x, y);
                let p = math::ray_plane(o, d, self.tip_box.0, n)?;
                let kg = kg0 + ((y0 - p[1]) * 20.0 / (0.6 * self.ltot)) as f64;
                return Some(Edit::Payload(((kg * 20.0).round() / 20.0).clamp(0.0, 20.0)));
            }
            Drag::Pose => {
                let (o, d) = self.ray(x, y);
                let p = math::ray_plane(o, d, self.root, Z)?;
                let mut a = ((p[1] - self.root[1]).atan2(p[0] - self.root[0]) as f64 - std::f64::consts::FRAC_PI_2).to_degrees();
                if a < -180.0 {
                    a += 360.0;
                }
                return Some(Edit::Elevation(a.round().clamp(ELEV_MIN, 90.0)));
            }
            _ => {}
        }
        None
    }

    fn pan(&mut self, dx: f32, dy: f32, s: f32) {
        let f = math::norm(math::sub(self.cam.cur.target, self.cam.cur.eye()));
        let r = math::norm(math::cross(f, [0.0, 1.0, 0.0]));
        let up = math::cross(r, f);
        self.cam.pan(math::add(math::scale(r, -dx * s), math::scale(up, dy * s)));
    }

    pub fn wheel(&mut self, dy: f64) {
        self.cam.zoom((dy * 0.0012).clamp(-0.5, 0.5).exp() as f32);
    }

    /// Advance and draw. `dt` in seconds.
    pub fn frame(&mut self, dt: f32, sc: &Scene, dark: bool) {
        let moving = self.cam.step(dt);
        if self.ring {
            self.ring_t += dt * 0.1;
            self.dirty = true;
        }
        let dpr = web_sys::window().map(|w| w.device_pixel_ratio()).unwrap_or(1.0);
        let (cw, ch) = (self.canvas.client_width().max(1), self.canvas.client_height().max(1));
        let (pw, ph) = ((cw as f64 * dpr) as u32, (ch as f64 * dpr) as u32);
        let resized = self.canvas.width() != pw || self.canvas.height() != ph;
        if resized {
            self.canvas.set_width(pw);
            self.canvas.set_height(ph);
        }
        let theme = if dark { gl::DARK } else { gl::LIGHT };
        let themed = theme.sky_top != self.r.theme.sky_top;
        if !(moving || self.dirty || resized || themed) {
            return;
        }
        self.r.theme = theme;
        if self.dirty {
            self.build(sc);
        }
        let o = self.cam.cur;
        let eye = o.eye();
        let view = math::look_at(eye, o.target, [0.0, 1.0, 0.0]);
        let proj = math::perspective(FOV, cw as f32 / ch as f32, (o.dist * 0.01).max(1e-3), o.dist * 100.0 + 10.0);
        self.inv_vp = math::invert(&math::mul(&proj, &view)).unwrap_or(math::IDENTITY);
        self.r.fog = [o.dist * 3.0 + self.ltot, o.dist * 12.0 + 4.0 * self.ltot];
        let mag = 10f32.powf((self.ltot / 5.0).log10().round());
        self.r.cell = mag;
        self.r.begin(&view, &proj, eye, [0, 0, pw as i32, ph as i32]);

        let (s, c) = self.theta.sin_cos();
        let (u, n0) = ([c, s, 0.0], [-s, c, 0.0]);
        let mut solid: Vec<(&Mesh, M4, Paint)> = vec![];
        let id = math::IDENTITY;
        for m in &self.links {
            solid.push((m, id, Paint::lit([1.0; 4])));
        }
        let hub = self.hub;
        let rt = self.root;
        let steel = Paint::lit([0.45, 0.48, 0.53, 1.0]);
        solid.push((&self.cube, math::basis([1., 0., 0.], [0., 1., 0.], [0., 0., 1.], [hub * 2.4, rt[1] - hub, hub * 3.0], [0.0, (rt[1] - hub) / 2.0, 0.0]), steel));
        solid.push((&self.cube, math::basis([1., 0., 0.], [0., 1., 0.], [0., 0., 1.], [hub * 5.0, hub * 0.4, hub * 5.0], [0.0, hub * 0.2, 0.0]), steel));
        let axis_z = |r: f32, len: f32, p: V| math::basis([1., 0., 0.], [0., 0., 1.], [0., -1., 0.], [2.0 * r, len, 2.0 * r], p);
        if !self.section {
            solid.push((&self.cyl, axis_z(hub, hub * 2.6, rt), Paint::lit([0.30, 0.33, 0.38, 1.0])));
            let glow = |k: u8| if self.hover == k || (k == 2 && self.drag == Drag::Pose) { 0.35 } else { 0.0 };
            let lever = hub * 3.2;
            solid.push((&self.cube, math::basis(u, n0, [0., 0., 1.], [hub * 0.35, lever, hub * 0.35], math::add(rt, math::scale(n0, lever / 2.0))), steel));
            let (hp, hr) = self.handle;
            solid.push((&self.sph, math::basis([1., 0., 0.], [0., 1., 0.], [0., 0., 1.], [hr; 3], hp), Paint { glow: glow(2), ..Paint::lit([0.95, 0.55, 0.15, 1.0]) }));
            for (i, l) in sc.chain.iter().enumerate() {
                if l.end_mass > 0.0 {
                    let (p, r) = self.joints[i];
                    let r = r * (1.0 + 0.25 * (l.end_mass as f32).cbrt());
                    solid.push((&self.cyl, axis_z(r, r * 1.6, p), Paint::lit([0.25, 0.27, 0.32, 1.0])));
                }
            }
            let (bp, b) = self.tip_box;
            let a = if sc.loads.payload > 0.0 { 1.0 } else { 0.35 };
            solid.push((&self.cube, math::basis([1., 0., 0.], [0., 1., 0.], [0., 0., 1.], [b; 3], bp), Paint { glow: glow(1), ..Paint::lit([0.20, 0.65, 0.55, a]) }));
        }
        for (m, model, p) in &solid {
            if p.color[3] >= 1.0 {
                self.r.shadow(m, model);
            }
        }
        for (m, model, p) in solid.iter().filter(|s| s.2.color[3] >= 1.0) {
            self.r.draw(m, model, *p);
        }
        if self.grav_on && !self.section {
            let len = (0.25 * self.ltot * (sc.loads.g / lf_core::G) as f32).clamp(0.02 * self.ltot, 0.5 * self.ltot);
            let w = 0.012 * self.ltot;
            let top = [0.5 * self.ltot, rt[1] + 0.45 * self.ltot, 0.35 * self.ltot];
            let yel = Paint::lit([1.0, 0.82, 0.2, 1.0]);
            self.r.draw(&self.cube, &math::basis([1., 0., 0.], [0., 1., 0.], [0., 0., 1.], [w, len, w], math::sub(top, [0.0, len / 2.0, 0.0])), yel);
            self.r.draw(&self.head, &math::basis([1., 0., 0.], [0., 1., 0.], [0., 0., 1.], [w * 4.0, w * 6.0, w * 4.0], math::sub(top, [0.0, len + w * 3.0, 0.0])), yel);
        }
        if self.ghost_on && !self.section {
            if let Some(e) = &self.edges {
                self.r.draw(e, &id, Paint::flat([0.55, 0.9, 1.0, 1.0]));
            }
            for m in &self.ghost {
                self.r.draw(m, &id, Paint { color: [0.55, 0.85, 1.0, 0.18], ..Paint::lit([1.0; 4]) });
            }
        }
        for (m, model, p) in solid.iter().filter(|s| s.2.color[3] < 1.0) {
            self.r.draw(m, model, *p);
        }
    }
}
