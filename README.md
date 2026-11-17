# link-flex

**한국어** · [English](#english)

로봇 팔·다리 링크의 강성과 처짐을 브라우저에서 바로 계산하는 정적 웹 앱입니다.
"이 튜브(또는 3D 프린팅 빔)에 이 페이로드를 달면 끝단이 얼마나 처지고, 응력은 안전하며, 얼마나 단단하고 몇 Hz로 울리는가?"에 답합니다.
Rust → WebAssembly, 100 % 클라이언트 측에서 동작하며 네트워크 요청·분석·추적이 전혀 없습니다.

> **교육용 추정치입니다 — 로봇에 맡기기 전에 실제 시험이나 FEM으로 검증하세요.**

## 기능

- 최대 3개의 직렬 링크(각각 길이·단면·재료·끝단 관절 질량 개별 설정)
- 단면: 원형 튜브, 사각 튜브, 속찬 사각형, I형 빔 (90° 회전 시 약축 굽힘)
- 재료 프리셋(모두 "대표값"): 알루미늄 6061, 강철, CFRP 튜브(준등방), PLA·PETG·나일론-CF 출력물. E·밀도·강도 편집 가능, 출력물은 감소계수(기본 강도 0.6, E 0.8)
- 끝단 페이로드, 중력, 자세 각도(0° = 수평, 90° = 수직), 동적 계수(기본 1.5)
- 결과: 끝단 처짐·기울기, 링크별 루트 모멘트·최대 응력·안전율(SF < 1이면 "파손 위험"), 끝단 강성 k = F/δ (N/mm), 1차 고유진동수, 총 질량
- 스윕: 링크 1의 두께(속찬 사각형은 높이)에 따른 끝단 처짐·질량 곡선
- "같은 강성, 더 가볍게?": 링크 1의 E·I를 맞추는 다른 재료·단면의 질량 목록(외형 유지, 두께 조정)
- WebGL2 3D 뷰: 실제 단면 압출, 과장 배율(1×–200×, 기본 자동 ≈ 길이의 10 %), 응력 색상(파랑→노랑→빨강, 빨강 = 항복), 변형 전 고스트, 중력 화살표, 그림자, 울림 애니메이션(0.1× 슬로모션)
- 2D 그래프: 체인을 따른 처짐, 굽힘 모멘트, 스윕 곡선
- 한국어/영어 전환(브라우저 언어 기본), 모바일 대응, 라이트/다크(시스템 설정)

## 모델과 가정

- Euler–Bernoulli 보, 미소 변형, 선형 탄성, 관절은 강체(별도 표기 없는 한)
- 각 링크는 이전 링크 끝에 고정된 캔틸레버. 점하중(페이로드·관절 질량)과 등분포하중(자중)을 중첩하고, 링크 루트의 변위·회전이 이후 링크에 강체 운동으로 더해집니다
- 중력은 자세 각도로 굽힘 방향에 투영(g·cos θ). 축방향 하중은 무시
- 굽힘 응력 σ = M·c/I. 전단 변형·전단 응력·비틀림은 무시하므로 von Mises = |σ|
- 동적 계수는 응력과 SF에만 곱하고, 처짐·강성은 정적 값
- SF = (강도 × 출력 감소계수) / (σ × 동적 계수)
- 1차 고유진동수: 정적 처짐 형상을 이용한 Rayleigh 법 f = (1/2π)·√(g·Σmδ / Σmδ²). 링크가 하나일 때는 닫힌해(1.875², 끝단 질량이 있으면 0.2357·m_beam 보정)도 함께 표시
- 단면의 높이 H가 굽힘(중력) 방향. 비틀림·좌굴·피로·FEM·복합재 적층 해석은 범위 밖

## 조작

- 왼쪽 드래그: 회전 · 오른쪽 드래그/두 손가락: 이동 · 휠/핀치: 줌 · 더블클릭: 리셋 (모두 관성 포함)
- 끝단 상자를 위아래로 드래그: 페이로드(0–20 kg) 변경, 주황색 핸들 드래그: 자세 각도 변경
- 뷰 버튼: 고스트 · 응력색 · 중력 · 과장 · 울림 · 원근/위/옆/단면 프리셋

## 로컬 빌드

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.22.1 --locked   # 또는 릴리스 바이너리
cargo test
trunk build --release   # 결과물: dist/
sh tools/privacy_check.sh
```

`trunk serve`로 로컬에서 확인할 수 있습니다(공개 경로 `/link-flex/`).

## 라이선스

MIT — `LICENSE` 참고 (Copyright link-flex contributors).

---

<a id="english"></a>

## English

A static web app for robot arm / leg links: "My link is this tube or printed beam with this payload —
how much does the tip sag, is the stress safe, and how stiff / how fast does it ring?"
Rust → WebAssembly, runs 100 % client-side: no network requests, analytics or trackers.

> **Educational estimates — validate with real tests / FEM before trusting a robot with it.**

### Features

- Up to 3 serial links, each with its own length, section, material and end joint / actuator mass
- Sections: round tube, rectangular tube, solid rectangle, I-beam (turn 90° to bend about the weak axis)
- Material presets, all labelled "typical": aluminium 6061, steel, CFRP tube (quasi-isotropic), printed PLA / PETG / nylon-CF.
  E, density and strength are editable; printed materials get knock-down factors (default 0.6 on strength, 0.8 on E)
- Tip payload, gravity, elevation (0° = horizontal, 90° = vertical), dynamic factor (default 1.5)
- Results: tip deflection and slope, root moment / max stress / safety factor per link ("fails" when SF < 1),
  end-effector stiffness k = F/δ (N/mm), first natural frequency, total mass
- Sweep: tip deflection and mass vs link 1 wall thickness (height for a solid rectangle)
- "Same stiffness, lighter?": mass of every material / section matching link 1's E·I (keep envelope, scale wall), sorted by mass
- WebGL2 3D view: real section extrusions, exaggeration (1×–200×, default auto ≈ 10 % of length), stress colour
  (blue → yellow → red, red = at yield), undeformed ghost, gravity arrow, shadows, ring animation (0.1× slow motion)
- 2D plots: deflection along the chain, bending moment, sweep curve
- Korean / English toggle (browser default), mobile friendly, light / dark via the system setting

### Model and assumptions

- Euler–Bernoulli beams, small deflection, linear elastic, joints rigid unless noted
- Each link is a cantilever clamped to the previous link's tip. Point loads (payload, joint masses) and the uniform
  self-weight are superposed; each link's root displacement and rotation add rigid-body motion to later links
- Gravity is projected onto the bending direction by the elevation (g·cos θ); axial load is ignored
- Bending stress σ = M·c/I. Shear deflection, shear stress and torsion are ignored, so von Mises = |σ|
- The dynamic factor scales stress and SF only; deflection and stiffness are static
- SF = (strength × print knock-down) / (σ × dynamic factor)
- First natural frequency by Rayleigh's method with the static deflection shape, f = (1/2π)·√(g·Σmδ / Σmδ²).
  For a single link the closed-form check is shown too (1.875², or tip mass + 0.2357·m_beam)
- Height H of each section lies in the bending (gravity) direction. Torsion, buckling, fatigue, FEM and composite
  layup analysis are out of scope

### Controls

- Left drag: orbit · right drag / two fingers: pan · wheel / pinch: zoom · double-click: reset (all with inertia)
- Drag the tip box up / down to set the payload (0–20 kg); drag the orange handle at the base to change the elevation
- View buttons: ghost · stress colour · gravity · exaggeration · ring · perspective / top / side / section presets

### Local build

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.22.1 --locked   # or a release binary
cargo test
trunk build --release   # output in dist/
sh tools/privacy_check.sh
```

`trunk serve` runs it locally (public path `/link-flex/`).

### Layout

- `crates/lf_core` — pure Rust model (sections, materials, chain solver, metrics, sweep, equivalent stiffness) with unit tests
- `crates/lf_web` — UI (`lib.rs`), 3D scene (`view3d.rs`), 2D plots (`plot.rs`), reusable renderer (`gl.rs`) and math (`math.rs`)

### License

MIT — see `LICENSE` (Copyright link-flex contributors).
