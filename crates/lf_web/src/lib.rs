//! link-flex web UI: inputs, readouts, 2D plots and the 3D view. Plain web-sys, no frameworks.
pub mod gl;
pub mod math;
pub mod plot;
pub mod view3d;

use lf_core::{analyze, equivalents, kind, make_section, preset, sweep, Link, Loads, Results, Section};
use plot::{fmt, Plot, Series};
use std::cell::RefCell;
use view3d::{Edit, Scene, View};
use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, HtmlCanvasElement, HtmlInputElement, PointerEvent, WheelEvent};

const T: &[(&str, &str, &str)] = &[
    ("sub", "로봇 링크 강성·처짐 계산기", "Robot link stiffness & deflection lab"),
    ("lang", "English", "한국어"),
    ("links", "링크 수", "Links"),
    ("link", "링크", "Link"),
    ("len", "길이 L (mm)", "Length L (mm)"),
    ("sect", "단면", "Section"),
    ("k0", "원형 튜브", "Round tube"),
    ("k1", "사각 튜브", "Rect tube"),
    ("k2", "사각 단면 (속찬)", "Solid rectangle"),
    ("k3", "I형 빔", "I-beam"),
    ("od", "외경 OD (mm)", "OD (mm)"),
    ("wall", "두께 t (mm)", "Wall t (mm)"),
    ("w", "폭 W (mm)", "Width W (mm)"),
    ("h", "높이 H (mm)", "Height H (mm)"),
    ("tf", "플랜지 두께 (mm)", "Flange t (mm)"),
    ("tw", "웹 두께 (mm)", "Web t (mm)"),
    ("rot", "90° 회전 (약축 굽힘)", "Turn 90° (weak axis)"),
    ("mat", "재료", "Material"),
    ("m0", "알루미늄 6061 (대표값)", "Aluminium 6061 (typical)"),
    ("m1", "강철 (대표값)", "Steel (typical)"),
    ("m2", "CFRP 튜브 (대표값, 준등방)", "CFRP tube (typical, quasi-isotropic)"),
    ("m3", "PLA 출력 (대표값, 100 % 채움)", "PLA printed (typical, 100 % infill)"),
    ("m4", "PETG 출력 (대표값)", "PETG printed (typical)"),
    ("m5", "나일론-CF 출력 (대표값)", "Nylon-CF printed (typical)"),
    ("e", "E (GPa)", "E (GPa)"),
    ("rho", "밀도 (kg/m³)", "Density (kg/m³)"),
    ("sy", "항복/강도 (MPa)", "Yield / strength (MPa)"),
    ("kds", "출력 감소계수 (강도)", "Print knock-down (strength)"),
    ("kde", "출력 감소계수 (E)", "Print knock-down (E)"),
    ("m", "끝단 관절·액추에이터 질량 (kg)", "End joint / actuator mass (kg)"),
    ("loads", "하중 · 자세", "Loads & pose"),
    ("pay", "끝단 페이로드 (kg)", "Tip payload (kg)"),
    ("g", "중력 (m/s²)", "Gravity (m/s²)"),
    ("elev", "자세 각도 (°, 0 = 수평, 90 = 수직)", "Elevation (°, 0 = horizontal, 90 = vertical)"),
    ("dyn", "동적 계수 (응력용)", "Dynamic factor (stress)"),
    ("tip", "끝단 처짐", "Tip deflection"),
    ("k", "강성", "Stiffness"),
    ("smax", "최대 응력", "Max stress"),
    ("sf", "최소 SF", "Min SF"),
    ("mass", "총 질량", "Total mass"),
    ("res", "결과", "Results"),
    ("c_mass", "질량 (kg)", "Mass (kg)"),
    ("c_mom", "루트 모멘트 (N·m)", "Root moment (N·m)"),
    ("c_s", "최대 응력 (MPa)", "Max stress (MPa)"),
    ("c_st", "판정", "Status"),
    ("ok", "OK", "OK"),
    ("fail", "파손 위험", "fails"),
    ("f1c", "단일 캔틸레버 닫힌해 f1", "Closed-form single-cantilever f1"),
    ("f1r", "Rayleigh f1 (정적 처짐 형상)", "Rayleigh f1 (static deflection shape)"),
    ("tmass", "링크 + 관절 질량, 페이로드 제외", "links + joint masses, payload excluded"),
    ("p_defl", "체인을 따른 처짐", "Deflection along the chain"),
    ("p_mom", "굽힘 모멘트", "Bending moment"),
    ("p_sweep", "링크 1 스윕: 끝단 처짐 · 질량", "Link 1 sweep: tip deflection & mass"),
    ("pos", "위치 (mm)", "position (mm)"),
    ("dy", "처짐 (mm, 아래 −)", "deflection (mm, down −)"),
    ("my", "M (N·m)", "M (N·m)"),
    ("sx_wall", "링크 1 두께 (mm)", "link 1 wall (mm)"),
    ("sx_h", "링크 1 높이 H (mm)", "link 1 height H (mm)"),
    ("sdefl", "끝단 처짐 (mm)", "tip deflection (mm)"),
    ("smass", "총 질량 (kg)", "total mass (kg)"),
    ("eq", "같은 강성, 더 가볍게? (링크 1의 E·I 일치, 외형 유지 · 두께 조정)", "Same stiffness, lighter? (match link 1 E·I, keep envelope, scale wall)"),
    ("dims", "치수 (mm)", "Dimensions (mm)"),
    ("cur", "현재", "current"),
    ("ghost", "고스트", "Ghost"),
    ("stress", "응력색", "Stress"),
    ("grav", "중력", "Gravity"),
    ("exag", "과장", "Exaggerate"),
    ("ring", "울림 0.1×", "Ring 0.1×"),
    ("auto", "자동", "auto"),
    ("v0", "원근", "Persp"),
    ("v1", "위", "Top"),
    ("v2", "옆", "Side"),
    ("v3", "단면", "Section"),
    ("help", "드래그: 회전 · 우클릭/두 손가락: 이동 · 휠/핀치: 줌 · 더블클릭: 리셋 · 상자를 위아래로: 페이로드 · 주황 핸들: 자세", "Drag: orbit · right-drag / two fingers: pan · wheel / pinch: zoom · double-click: reset · drag the box up/down: payload · orange handle: pose"),
    ("nogl", "WebGL2를 사용할 수 없어 3D 뷰를 표시할 수 없습니다. 수치와 그래프는 정상 동작합니다.", "WebGL2 is not available, so the 3D view is off. Numbers and plots still work."),
    ("notes", "가정: Euler–Bernoulli 보, 미소 변형, 선형 탄성, 강체 관절, 단면은 높이 H가 굽힘 방향. 전단 변형·전단 응력·비틀림 무시 → von Mises = |M|·c/I. 축방향 하중 무시. 동적 계수는 응력·SF에만 적용하고 처짐·강성은 정적 값. 재료값은 일반적인 대표값이며 출력 재료는 감소계수 적용. 울림 애니메이션 감쇠비는 0.03 가정.", "Assumptions: Euler–Bernoulli beams, small deflection, linear elastic, rigid joints, height H lies in the bending direction. Shear deflection, shear stress and torsion ignored → von Mises = |M|·c/I. Axial load ignored. The dynamic factor scales stress / SF only; deflection and stiffness are static. Material values are generic typical values; printed materials use the knock-down factors. Ring animation assumes 3 % damping."),
    ("disc", "교육용 추정치입니다 — 로봇에 맡기기 전에 실제 시험이나 FEM으로 검증하세요.", "Educational estimates — validate with real tests / FEM before trusting a robot with it."),
];

struct App {
    links: [Link; 3],
    n: usize,
    loads: Loads,
    ko: bool,
    res: Results,
    view: Option<View>,
    last_t: f64,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

fn with<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|a| a.try_borrow_mut().ok()?.as_mut().map(f))
}

fn doc() -> Document {
    web_sys::window().expect("window").document().expect("document")
}
fn el(id: &str) -> Option<Element> {
    doc().get_element_by_id(id)
}
fn dark() -> bool {
    web_sys::window().and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok().flatten()).is_some_and(|m| m.matches())
}
fn mm(v: f64) -> String {
    let s = format!("{:.2}", v * 1e3);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

impl App {
    fn t(&self, key: &str) -> &'static str {
        T.iter().find(|e| e.0 == key).map(|e| if self.ko { e.1 } else { e.2 }).unwrap_or("?")
    }
    fn chain(&self) -> &[Link] {
        &self.links[..self.n]
    }

    fn num(&self, key: &str, label: &str, v: f64, step: &str) -> String {
        format!("<label>{}<input type=\"number\" data-k=\"{key}\" value=\"{}\" step=\"{step}\" min=\"0\"></label>", self.t(label), fmt_in(v))
    }

    fn render_form(&self) {
        let mut h = format!("<fieldset><legend>{}</legend><label>{}<select data-k=\"n\">", self.t("loads"), self.t("links"));
        for k in 1..=3 {
            h += &format!("<option value=\"{k}\"{}>{k}</option>", sel(k == self.n));
        }
        h += "</select></label>";
        let l = &self.loads;
        h += &self.num("pay", "pay", l.payload, "0.1");
        h += &self.num("g", "g", l.g, "0.01");
        h += &format!("<label>{}<input type=\"number\" data-k=\"elev\" value=\"{}\" step=\"1\" min=\"-30\" max=\"90\"></label>", self.t("elev"), fmt_in(l.elevation.to_degrees()));
        h += &self.num("dyn", "dyn", l.dyn_factor, "0.1");
        h += "</fieldset>";
        for (i, lk) in self.chain().iter().enumerate() {
            h += &format!("<fieldset><legend>{} {}</legend>", self.t("link"), i + 1);
            h += &self.num(&format!("{i}:len"), "len", lk.length * 1e3, "1");
            h += &format!("<label>{}<select data-k=\"{i}:kind\">", self.t("sect"));
            for k in 0..4 {
                h += &format!("<option value=\"{k}\"{}>{}</option>", sel(k == kind(&lk.section)), self.t(&format!("k{k}")));
            }
            h += "</select></label>";
            let d = |key: &str, v: f64| self.num(&format!("{i}:{key}"), key, v * 1e3, "0.1");
            h += &match lk.section {
                Section::RoundTube { od, wall } => d("od", od) + &d("wall", wall),
                Section::RectTube { w, h: hh, wall } => d("w", w) + &d("h", hh) + &d("wall", wall),
                Section::SolidRect { w, h: hh } => d("w", w) + &d("h", hh),
                Section::IBeam { w, h: hh, tf, tw } => d("w", w) + &d("h", hh) + &d("tf", tf) + &d("tw", tw),
            };
            if !matches!(lk.section, Section::RoundTube { .. }) {
                h += &format!("<label class=\"chk\"><input type=\"checkbox\" data-k=\"{i}:rot\"{}>{}</label>", if lk.rotated { " checked" } else { "" }, self.t("rot"));
            }
            h += &format!("<label>{}<select data-k=\"{i}:mat\">", self.t("mat"));
            for k in 0..6 {
                h += &format!("<option value=\"{k}\"{}>{}</option>", sel(k == lk.material.preset), self.t(&format!("m{k}")));
            }
            h += "</select></label>";
            let m = &lk.material;
            h += &self.num(&format!("{i}:e"), "e", m.e / 1e9, "0.1");
            h += &self.num(&format!("{i}:rho"), "rho", m.density, "10");
            h += &self.num(&format!("{i}:sy"), "sy", m.strength / 1e6, "1");
            if m.printed {
                h += &self.num(&format!("{i}:kds"), "kds", m.kd_strength, "0.05");
                h += &self.num(&format!("{i}:kde"), "kde", m.kd_e, "0.05");
            }
            h += &self.num(&format!("{i}:m"), "m", lk.end_mass, "0.05");
            h += "</fieldset>";
        }
        if let Some(f) = el("form") {
            f.set_inner_html(&h);
        }
    }

    fn texts(&self) {
        let d = doc();
        d.document_element().map(|e| e.set_attribute("lang", if self.ko { "ko" } else { "en" }));
        if let Ok(list) = d.query_selector_all("[data-i18n]") {
            for i in 0..list.length() {
                if let Some(e) = list.item(i).and_then(|n| n.dyn_into::<Element>().ok()) {
                    let k = e.get_attribute("data-i18n").unwrap_or_default();
                    e.set_text_content(Some(self.t(&k)));
                }
            }
        }
    }

    /// Apply one form edit. Returns true if the form must be re-rendered.
    fn apply(&mut self, key: &str, val: &str, checked: bool) -> bool {
        let v: Option<f64> = val.trim().parse().ok().filter(|v: &f64| v.is_finite());
        let (idx, field) = match key.split_once(':') {
            Some((i, f)) => (i.parse::<usize>().ok(), f),
            None => (None, key),
        };
        let Some(i) = idx else {
            let Some(v) = v else { return false };
            match field {
                "n" => {
                    self.n = (v as usize).clamp(1, 3);
                    return true;
                }
                "pay" => self.loads.payload = v.max(0.0),
                "g" => self.loads.g = v.max(0.0),
                "elev" => self.loads.elevation = v.clamp(-30.0, 90.0).to_radians(),
                "dyn" => self.loads.dyn_factor = v.max(0.0),
                _ => {}
            }
            return false;
        };
        let Some(lk) = self.links.get_mut(i) else { return false };
        if field == "rot" {
            lk.rotated = checked;
            return false;
        }
        let Some(v) = v else { return false };
        let si = v.max(0.0) * 1e-3;
        let s = &mut lk.section;
        match (field, s) {
            ("len", _) => lk.length = si.max(1e-3),
            ("kind", s) => {
                let (w, h) = s.envelope();
                let t = lf_core::sweep_param(s).min(w.min(h) * 0.15).max(5e-4);
                *s = make_section(v as usize, w, h, t);
                return true;
            }
            ("mat", _) => {
                lk.material = preset(v as usize);
                return true;
            }
            ("od", Section::RoundTube { od, .. }) => *od = si,
            ("wall", Section::RoundTube { wall, .. } | Section::RectTube { wall, .. }) => *wall = si,
            ("w", Section::RectTube { w, .. } | Section::SolidRect { w, .. } | Section::IBeam { w, .. }) => *w = si,
            ("h", Section::RectTube { h, .. } | Section::SolidRect { h, .. } | Section::IBeam { h, .. }) => *h = si,
            ("tf", Section::IBeam { tf, .. }) => *tf = si,
            ("tw", Section::IBeam { tw, .. }) => *tw = si,
            ("e", _) => lk.material.e = (v * 1e9).max(1e6),
            ("rho", _) => lk.material.density = v.max(0.0),
            ("sy", _) => lk.material.strength = (v * 1e6).max(1.0),
            ("kds", _) => lk.material.kd_strength = v.clamp(0.01, 1.5),
            ("kde", _) => lk.material.kd_e = v.clamp(0.01, 1.5),
            ("m", _) => lk.end_mass = v.max(0.0),
            _ => {}
        }
        false
    }

    fn recompute(&mut self) {
        self.res = analyze(self.chain(), &self.loads);
        if let Some(v) = &mut self.view {
            v.dirty = true;
        }
        self.outputs();
    }

    fn hud(&self) {
        let r = &self.res;
        let ex = self.view.as_ref().map(|v| format!("<span class=\"ex\">×{:.0}</span>", v.ex_used)).unwrap_or_default();
        let smax = r.links.iter().map(|l| l.max_stress).fold(0.0, f64::max);
        let sfc = if r.fails { " class=\"bad\"" } else { "" };
        let h = format!(
            "<div>{} <b>{} mm</b></div><div>{} <b>{} N/mm</b></div><div>{} <b>{} MPa</b></div><div{sfc}>{} <b>{}</b></div><div>f1 <b>{} Hz</b></div><div>{} <b>{} kg</b></div>{ex}",
            self.t("tip"), fmt(r.tip_defl * 1e3), self.t("k"), fmt(r.stiffness / 1e3), self.t("smax"), fmt(smax / 1e6),
            self.t("sf"), sf_txt(r.min_sf), fmt(r.f1), self.t("mass"), fmt(r.total_mass)
        );
        if let Some(e) = el("hud") {
            e.set_inner_html(&h);
        }
    }

    fn outputs(&self) {
        self.hud();
        let r = &self.res;
        let mut h = format!(
            "<table><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>SF</th><th>{}</th></tr>",
            self.t("link"), self.t("c_mass"), self.t("c_mom"), self.t("c_s"), self.t("c_st")
        );
        for (i, l) in r.links.iter().enumerate() {
            let bad = l.sf < 1.0;
            h += &format!(
                "<tr{}><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                if bad { " class=\"bad\"" } else { "" }, i + 1, fmt(l.mass), fmt(l.root_moment), fmt(l.max_stress / 1e6), sf_txt(l.sf),
                self.t(if bad { "fail" } else { "ok" })
            );
        }
        h += "</table><ul>";
        h += &format!("<li>{}: <b>{} Hz</b></li>", self.t("f1r"), fmt(r.f1));
        if let Some(f) = r.f1_closed {
            h += &format!("<li>{}: <b>{} Hz</b></li>", self.t("f1c"), fmt(f));
        }
        h += &format!("<li>{}: <b>{} N/mm</b> · {}: <b>{} kg</b> ({})</li></ul>", self.t("k"), fmt(r.stiffness / 1e3), self.t("mass"), fmt(r.total_mass), self.t("tmass"));
        if let Some(e) = el("summary") {
            e.set_inner_html(&h);
        }

        let l0 = self.links[0];
        let mats: Vec<_> = (0..6).map(|k| if k == l0.material.preset { l0.material } else { preset(k) }).collect();
        let mut h = format!("<table><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr>", self.t("mat"), self.t("sect"), self.t("dims"), self.t("c_mass"));
        for e in equivalents(&l0, &mats).iter().take(10) {
            let cur = e.material.preset == l0.material.preset && kind(&e.section) == kind(&l0.section);
            let dims = match e.section {
                Section::RoundTube { od, wall } => format!("Ø{} × {}", mm(od), mm(wall)),
                Section::RectTube { w, h, wall } => format!("{} × {} × {}", mm(w), mm(h), mm(wall)),
                Section::SolidRect { w, h } => format!("{} × {}", mm(w), mm(h)),
                Section::IBeam { w, h, tf, .. } => format!("{} × {}, t {}", mm(w), mm(h), mm(tf)),
            };
            h += &format!(
                "<tr{}><td>{}</td><td>{}</td><td>{dims}</td><td>{}{}</td></tr>",
                if cur { " class=\"cur\"" } else { "" }, self.t(&format!("m{}", e.material.preset)), self.t(&format!("k{}", kind(&e.section))),
                fmt(e.mass), if cur { format!(" ({})", self.t("cur")) } else { String::new() }
            );
        }
        h += "</table>";
        if let Some(e) = el("equiv") {
            e.set_inner_html(&format!("<h3>{}</h3>{h}", self.t("eq")));
        }
        self.plots();
    }

    fn plots(&self) {
        let r = &self.res;
        let dk = dark();
        let mut bounds = vec![];
        let mut acc = 0.0;
        for l in &self.chain()[..self.n - 1] {
            acc += l.length * 1e3;
            bounds.push(acc);
        }
        let defl: Vec<(f64, f64)> = r.samples.iter().map(|s| (s.s * 1e3, -s.defl * 1e3)).collect();
        let mom: Vec<(f64, f64)> = r.samples.iter().map(|s| (s.s * 1e3, s.moment)).collect();
        let canvas = |id: &str| el(id).and_then(|e| e.dyn_into::<HtmlCanvasElement>().ok());
        let one = |pts, color, label| Plot { xlabel: self.t("pos"), series: vec![Series { pts, color, label }], vlines: &bounds, mark: None, dark: dk };
        if let Some(c) = canvas("p_defl") {
            plot::draw(&c, &one(&defl, "#3b82f6", self.t("dy")));
        }
        if let Some(c) = canvas("p_mom") {
            plot::draw(&c, &one(&mom, "#ef6c3a", self.t("my")));
        }
        let (pts, cur) = sweep(self.chain(), &self.loads, 40);
        let a: Vec<(f64, f64)> = pts.iter().map(|p| (p.x * 1e3, p.tip_defl * 1e3)).collect();
        let b: Vec<(f64, f64)> = pts.iter().map(|p| (p.x * 1e3, p.mass)).collect();
        let xl = if matches!(self.links[0].section, Section::SolidRect { .. }) { "sx_h" } else { "sx_wall" };
        if let Some(c) = canvas("p_sweep") {
            let p = Plot {
                xlabel: self.t(xl),
                series: vec![Series { pts: &a, color: "#3b82f6", label: self.t("sdefl") }, Series { pts: &b, color: "#16a34a", label: self.t("smass") }],
                vlines: &[],
                mark: Some(cur * 1e3),
                dark: dk,
            };
            plot::draw(&c, &p);
        }
    }

    fn scene(&self) -> Scene<'_> {
        Scene { chain: &self.links[..self.n], res: &self.res, loads: &self.loads }
    }
}

fn fmt_in(v: f64) -> String {
    let s = format!("{v:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}
fn sel(b: bool) -> &'static str {
    if b { " selected" } else { "" }
}
fn sf_txt(sf: f64) -> String {
    if sf.is_finite() && sf < 1e4 { format!("{sf:.2}") } else { "∞".into() }
}

fn listen<E: JsCast + 'static>(target: &web_sys::EventTarget, ev: &str, mut f: impl FnMut(E) + 'static) {
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| f(e.unchecked_into::<E>()));
    let opts = web_sys::AddEventListenerOptions::new();
    opts.set_passive(false);
    let _ = target.add_event_listener_with_callback_and_add_event_listener_options(ev, cb.as_ref().unchecked_ref(), &opts);
    cb.forget();
}

fn set_input(key: &str, v: f64) {
    if let Some(i) = doc().query_selector(&format!("[data-k=\"{key}\"]")).ok().flatten().and_then(|e| e.dyn_into::<HtmlInputElement>().ok()) {
        i.set_value(&fmt_in(v));
    }
}

fn sync_tools(a: &App) {
    let Some(v) = &a.view else { return };
    if let Ok(list) = doc().query_selector_all("#tools [data-t]") {
        for i in 0..list.length() {
            if let Some(e) = list.item(i).and_then(|n| n.dyn_into::<Element>().ok()) {
                let on = match e.get_attribute("data-t").as_deref() {
                    Some("ghost") => v.ghost_on,
                    Some("stress") => v.stress_on,
                    Some("grav") => v.grav_on,
                    Some("exag") => v.exag_on,
                    Some("ring") => v.ring,
                    _ => false,
                };
                let _ = e.class_list().toggle_with_force("on", on);
            }
        }
    }
}

fn raf() {
    let cb = Closure::once_into_js(|t: f64| {
        with(|a| {
            let dt = if a.last_t > 0.0 { ((t - a.last_t) / 1000.0).clamp(0.0, 0.1) } else { 1.0 / 60.0 };
            a.last_t = t;
            let dk = dark();
            let shown = a.view.as_ref().map(|v| v.ex_used);
            if let Some(mut v) = a.view.take() {
                v.frame(dt as f32, &a.scene(), dk);
                let cur = v.cursor();
                a.view = Some(v);
                if let Some(c) = el("view").and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
                    let _ = c.style().set_property("cursor", cur);
                }
            }
            if a.view.as_ref().map(|v| v.ex_used) != shown {
                a.hud();
                if let (Some(v), Some(s)) = (&a.view, el("exs").and_then(|e| e.dyn_into::<HtmlInputElement>().ok())) {
                    if v.ex_auto {
                        s.set_value(&format!("{:.0}", v.ex_used));
                    }
                }
            }
        });
        raf();
    });
    if let Some(w) = web_sys::window() {
        let _ = w.request_animation_frame(cb.unchecked_ref());
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    let ko = web_sys::window().and_then(|w| w.navigator().language()).is_some_and(|l| l.to_lowercase().starts_with("ko"));
    let mut links = [Link {
        length: 0.3,
        section: Section::RoundTube { od: 0.025, wall: 0.002 },
        material: preset(0),
        rotated: false,
        end_mass: 0.6,
    }; 3];
    links[1].section = Section::RoundTube { od: 0.02, wall: 0.0015 };
    links[1].length = 0.25;
    links[1].end_mass = 0.3;
    links[2].section = Section::RectTube { w: 0.02, h: 0.02, wall: 0.002 };
    links[2].material = preset(3);
    links[2].length = 0.12;
    links[2].end_mass = 0.0;
    let loads = Loads { payload: 1.0, ..Loads::default() };
    let res = analyze(&links[..2], &loads);
    let app = App { links, n: 2, loads, ko, res, view: None, last_t: 0.0 };
    app.texts();
    app.render_form();
    app.outputs();
    APP.with(|a| *a.borrow_mut() = Some(app));


    if let Some(f) = el("form") {
        listen(&f, "input", |e: web_sys::Event| {
            let Some(t) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) else { return };
            let Some(key) = t.get_attribute("data-k") else { return };
            let (val, checked) = match t.dyn_ref::<HtmlInputElement>() {
                Some(i) => (i.value(), i.checked()),
                None => (t.dyn_ref::<web_sys::HtmlSelectElement>().map(|s| s.value()).unwrap_or_default(), false),
            };
            with(|a| {
                if a.apply(&key, &val, checked) {
                    a.render_form();
                }
                a.recompute();
            });
        });
    }
    if let Some(b) = el("lang") {
        listen(&b, "click", |_: web_sys::Event| {
            with(|a| {
                a.ko = !a.ko;
                a.texts();
                a.render_form();
                a.outputs();
            });
        });
    }
    if let Some(w) = web_sys::window() {
        listen(&w, "resize", |_: web_sys::Event| {
            with(|a| a.plots());
        });
    }

    // 3D view last: if anything here fails, the readouts above keep working.
    let canvas = el("view").and_then(|e| e.dyn_into::<HtmlCanvasElement>().ok());
    let view = canvas.clone().and_then(View::new);
    if view.is_none() {
        if let Some(m) = el("nogl") {
            let _ = m.remove_attribute("hidden");
        }
        if let Some(t) = el("tools") {
            let _ = t.set_attribute("hidden", "");
        }
        return;
    }
    with(|a| {
        a.view = view;
        let mut v = a.view.take();
        if let Some(v) = &mut v {
            v.preset(0, &a.scene());
            v.cam.cur = v.cam.goal;
        }
        a.view = v;
        sync_tools(a);
    });
    let Some(c) = canvas else { return };
    let xy = |e: &web_sys::MouseEvent| (e.offset_x() as f32, e.offset_y() as f32);
    listen(&c, "pointerdown", move |e: PointerEvent| {
        e.prevent_default();
        if let Some(t) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
            let _ = t.set_pointer_capture(e.pointer_id());
        }
        let (x, y) = xy(&e);
        with(|a| {
            let p = a.loads.payload;
            if let Some(v) = &mut a.view {
                v.pointer_down(e.pointer_id(), x, y, e.button(), p);
            }
        });
    });
    listen(&c, "pointermove", move |e: PointerEvent| {
        let (x, y) = xy(&e);
        with(|a| {
            let edit = a.view.as_mut().and_then(|v| v.pointer_move(e.pointer_id(), x, y));
            match edit {
                Some(Edit::Payload(kg)) if kg != a.loads.payload => {
                    a.loads.payload = kg;
                    set_input("pay", kg);
                    a.recompute();
                }
                Some(Edit::Elevation(deg)) if (deg - a.loads.elevation.to_degrees()).abs() > 1e-9 => {
                    a.loads.elevation = deg.to_radians();
                    set_input("elev", deg);
                    a.recompute();
                }
                _ => {}
            }
        });
    });
    for ev in ["pointerup", "pointercancel"] {
        listen(&c, ev, |e: PointerEvent| {
            with(|a| a.view.as_mut().map(|v| v.pointer_up(e.pointer_id())));
        });
    }
    listen(&c, "wheel", |e: WheelEvent| {
        e.prevent_default();
        with(|a| a.view.as_mut().map(|v| v.wheel(e.delta_y())));
    });
    listen(&c, "contextmenu", |e: web_sys::Event| e.prevent_default());
    listen(&c, "dblclick", |_: web_sys::Event| {
        with(|a| {
            if let Some(mut v) = a.view.take() {
                v.preset(0, &a.scene());
                a.view = Some(v);
            }
        });
    });
    if let Some(t) = el("tools") {
        listen(&t, "click", |e: web_sys::Event| {
            let Some(t) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) else { return };
            with(|a| {
                let Some(mut v) = a.view.take() else { return };
                if let Some(k) = t.get_attribute("data-t") {
                    match k.as_str() {
                        "ghost" => v.ghost_on ^= true,
                        "stress" => v.stress_on ^= true,
                        "grav" => v.grav_on ^= true,
                        "exag" => v.exag_on ^= true,
                        "ring" => v.ring ^= true,
                        _ => {}
                    }
                    v.dirty = true;
                }
                if let Some(p) = t.get_attribute("data-p").and_then(|p| p.parse().ok()) {
                    v.preset(p, &a.scene());
                }
                a.view = Some(v);
                sync_tools(a);
                a.hud();
            });
        });
        listen(&t, "input", |e: web_sys::Event| {
            let Some(i) = e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) else { return };
            with(|a| {
                if let Some(v) = &mut a.view {
                    if i.id() == "exauto" {
                        v.ex_auto = i.checked();
                    } else if let Ok(x) = i.value().parse::<f32>() {
                        v.ex_val = x;
                        v.ex_auto = false;
                        set_checked("exauto", false);
                    }
                    v.dirty = true;
                }
            });
        });
    }

    raf();
}

fn set_checked(id: &str, on: bool) {
    if let Some(i) = el(id).and_then(|e| e.dyn_into::<HtmlInputElement>().ok()) {
        i.set_checked(on);
    }
}
