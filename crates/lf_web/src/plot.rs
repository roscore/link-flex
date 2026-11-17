//! Tiny canvas-2D line plots: up to two series with independent y axes (left / right),
//! optional vertical guide lines and a highlighted marker.
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d as Ctx, HtmlCanvasElement};

pub struct Series<'a> {
    pub pts: &'a [(f64, f64)],
    pub color: &'a str,
    pub label: &'a str,
}

pub struct Plot<'a> {
    pub xlabel: &'a str,
    pub series: Vec<Series<'a>>,
    pub vlines: &'a [f64],
    /// Marker at this x (drawn on every series).
    pub mark: Option<f64>,
    pub dark: bool,
}

fn nice_range(lo: f64, hi: f64) -> (f64, f64) {
    let (lo, hi) = if (hi - lo).abs() < 1e-12 { (lo - 1.0, hi + 1.0) } else { (lo, hi) };
    let pad = (hi - lo) * 0.06;
    (lo - pad, hi + pad)
}

pub fn fmt(v: f64) -> String {
    let a = v.abs();
    if !v.is_finite() {
        "—".into()
    } else if a >= 1000.0 || a == 0.0 {
        format!("{v:.0}")
    } else if a >= 10.0 {
        format!("{v:.1}")
    } else if a >= 0.1 {
        format!("{v:.2}")
    } else {
        format!("{v:.3}")
    }
}

fn interp(pts: &[(f64, f64)], x: f64) -> Option<f64> {
    pts.windows(2).find(|w| w[0].0 <= x && x <= w[1].0).map(|w| {
        let t = if w[1].0 > w[0].0 { (x - w[0].0) / (w[1].0 - w[0].0) } else { 0.0 };
        w[0].1 + (w[1].1 - w[0].1) * t
    })
}

pub fn draw(canvas: &HtmlCanvasElement, p: &Plot) {
    let dpr = web_sys::window().map(|w| w.device_pixel_ratio()).unwrap_or(1.0);
    let (cw, ch) = (canvas.client_width().max(50) as f64, canvas.client_height().max(50) as f64);
    let (pw, ph) = ((cw * dpr) as u32, (ch * dpr) as u32);
    if canvas.width() != pw || canvas.height() != ph {
        canvas.set_width(pw);
        canvas.set_height(ph);
    }
    let Some(c) = canvas.get_context("2d").ok().flatten().and_then(|c| c.dyn_into::<Ctx>().ok()) else { return };
    let _ = c.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
    c.clear_rect(0.0, 0.0, cw, ch);
    let fg = if p.dark { "#c8ccd4" } else { "#3a3f48" };
    let grid = if p.dark { "#343a44" } else { "#dde0e5" };
    c.set_font("11px system-ui, sans-serif");
    let (l, r, t, b) = (48.0, if p.series.len() > 1 { cw - 48.0 } else { cw - 22.0 }, 18.0, ch - 30.0);
    let all = p.series.iter().flat_map(|s| s.pts.iter());
    let (x0, x1) = all.fold((f64::INFINITY, f64::NEG_INFINITY), |a, q| (a.0.min(q.0), a.1.max(q.0)));
    if !(x0.is_finite() && x1 > x0) {
        return;
    }
    let sx = |x: f64| l + (x - x0) / (x1 - x0) * (r - l);
    c.set_stroke_style_str(grid);
    c.set_line_width(1.0);
    c.stroke_rect(l, t, r - l, b - t);
    c.set_fill_style_str(fg);
    c.set_text_align("center");
    for k in 0..=4 {
        let x = x0 + (x1 - x0) * k as f64 / 4.0;
        let _ = c.fill_text(&fmt(x), sx(x), b + 13.0);
    }
    let _ = c.fill_text(p.xlabel, (l + r) / 2.0, ch - 3.0);
    for &v in p.vlines {
        c.set_stroke_style_str(grid);
        c.begin_path();
        c.move_to(sx(v), t);
        c.line_to(sx(v), b);
        c.stroke();
    }
    for (k, s) in p.series.iter().enumerate() {
        let (y0, y1) = s.pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, q| (a.0.min(q.1), a.1.max(q.1)));
        if !(y0.is_finite() && y1.is_finite()) {
            continue;
        }
        let (y0, y1) = nice_range(y0, y1);
        let sy = |y: f64| b - (y - y0) / (y1 - y0) * (b - t);
        c.set_fill_style_str(s.color);
        let (ax, align) = if k == 0 { (l - 4.0, "right") } else { (r + 4.0, "left") };
        c.set_text_align(align);
        for j in 0..=4 {
            let y = y0 + (y1 - y0) * j as f64 / 4.0;
            let _ = c.fill_text(&fmt(y), ax, sy(y) + 4.0);
        }
        c.set_text_align(if k == 0 { "left" } else { "right" });
        let _ = c.fill_text(s.label, if k == 0 { 4.0 } else { cw - 4.0 }, t - 5.0);
        if y0 < 0.0 && y1 > 0.0 {
            c.set_stroke_style_str(grid);
            c.begin_path();
            c.move_to(l, sy(0.0));
            c.line_to(r, sy(0.0));
            c.stroke();
        }
        c.set_stroke_style_str(s.color);
        c.set_line_width(2.0);
        c.begin_path();
        for (i, &(x, y)) in s.pts.iter().enumerate() {
            if i == 0 { c.move_to(sx(x), sy(y)) } else { c.line_to(sx(x), sy(y)) }
        }
        c.stroke();
        if let Some(y) = p.mark.and_then(|m| interp(s.pts, m)) {
            c.begin_path();
            let _ = c.arc(sx(p.mark.unwrap_or(0.0)), sy(y), 4.5, 0.0, std::f64::consts::TAU);
            c.fill();
        }
    }
    c.set_line_width(1.0);
}
