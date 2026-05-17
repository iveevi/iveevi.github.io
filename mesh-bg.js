// Flowing Delaunay-triangulated 2D mesh background — composited from
// multiple layers at different densities, ink weights, and flow directions.
//
// Each layer has its own Poisson-disk point set, its own Delaunay
// triangulation, and an independently-rotating global flow direction.
// All layers share the same vertex shader and are drawn in back-to-front
// order with alpha blending.

(function () {
  const canvas = document.getElementById('mesh-bg');
  if (!canvas) return;

  const gl = canvas.getContext('webgl', { antialias: true, alpha: true })
          || canvas.getContext('experimental-webgl');
  if (!gl) {
    console.warn('[mesh-bg] WebGL unavailable');
    return;
  }

  // ------------------------------------------------------------------------
  // Bowyer-Watson Delaunay
  // ------------------------------------------------------------------------

  function delaunayTriangulate(points) {
    const n = points.length;
    if (n < 3) return new Uint32Array();

    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const [x, y] of points) {
      if (x < minX) minX = x; if (y < minY) minY = y;
      if (x > maxX) maxX = x; if (y > maxY) maxY = y;
    }
    const cx = (minX + maxX) * 0.5, cy = (minY + maxY) * 0.5;
    const dmax = Math.max(maxX - minX, maxY - minY) * 50;

    const pts = points.slice();
    const stA = n, stB = n + 1, stC = n + 2;
    pts.push([cx - dmax, cy - dmax]);
    pts.push([cx + dmax, cy - dmax]);
    pts.push([cx,        cy + dmax]);

    function circumcircle(a, b, c) {
      const ax = pts[a][0], ay = pts[a][1];
      const bx = pts[b][0], by = pts[b][1];
      const ccx = pts[c][0], ccy = pts[c][1];
      const d = 2 * (ax * (by - ccy) + bx * (ccy - ay) + ccx * (ay - by));
      if (Math.abs(d) < 1e-12) return null;
      const a2 = ax * ax + ay * ay;
      const b2 = bx * bx + by * by;
      const c2 = ccx * ccx + ccy * ccy;
      const ux = (a2 * (by - ccy) + b2 * (ccy - ay) + c2 * (ay - by)) / d;
      const uy = (a2 * (ccx - bx) + b2 * (ax - ccx) + c2 * (bx - ax)) / d;
      const r2 = (ux - ax) * (ux - ax) + (uy - ay) * (uy - ay);
      return [ux, uy, r2];
    }

    let tris = [{ a: stA, b: stB, c: stC, cc: circumcircle(stA, stB, stC) }];

    for (let i = 0; i < n; i++) {
      const px = pts[i][0], py = pts[i][1];
      const bad = [];
      const remaining = [];
      for (const t of tris) {
        const cc = t.cc;
        const dx = px - cc[0], dy = py - cc[1];
        if (dx * dx + dy * dy < cc[2]) bad.push(t);
        else remaining.push(t);
      }
      tris = remaining;

      const counts = new Map();
      for (const t of bad) {
        for (const [u, v] of [[t.a, t.b], [t.b, t.c], [t.c, t.a]]) {
          const key = u < v ? u * 1000003 + v : v * 1000003 + u;
          counts.set(key, (counts.get(key) || 0) + 1);
        }
      }
      for (const t of bad) {
        for (const [u, v] of [[t.a, t.b], [t.b, t.c], [t.c, t.a]]) {
          const key = u < v ? u * 1000003 + v : v * 1000003 + u;
          if (counts.get(key) === 1) {
            const cc = circumcircle(u, v, i);
            if (cc) tris.push({ a: u, b: v, c: i, cc });
          }
        }
      }
    }

    const keep = tris.filter((t) => t.a < n && t.b < n && t.c < n);
    const out = new Uint32Array(keep.length * 3);
    for (let k = 0; k < keep.length; k++) {
      out[k * 3 + 0] = keep[k].a;
      out[k * 3 + 1] = keep[k].b;
      out[k * 3 + 2] = keep[k].c;
    }
    return out;
  }

  // ------------------------------------------------------------------------
  // Bridson Poisson-disk
  // ------------------------------------------------------------------------

  function poissonDisk(width, height, radius, k = 25) {
    const cell = radius / Math.SQRT2;
    const gw = Math.ceil(width / cell);
    const gh = Math.ceil(height / cell);
    const grid = new Array(gw * gh).fill(-1);
    const pts = [];

    function gridAt(p) {
      return Math.floor(p[1] / cell) * gw + Math.floor(p[0] / cell);
    }
    function farEnough(p) {
      const cx = Math.floor(p[0] / cell);
      const cy = Math.floor(p[1] / cell);
      const r2 = radius * radius;
      for (let dy = -2; dy <= 2; dy++) {
        const ny = cy + dy;
        if (ny < 0 || ny >= gh) continue;
        for (let dx = -2; dx <= 2; dx++) {
          const nx = cx + dx;
          if (nx < 0 || nx >= gw) continue;
          const idx = grid[ny * gw + nx];
          if (idx === -1) continue;
          const q = pts[idx];
          const dxq = q[0] - p[0], dyq = q[1] - p[1];
          if (dxq * dxq + dyq * dyq < r2) return false;
        }
      }
      return true;
    }

    const seed = [Math.random() * width, Math.random() * height];
    pts.push(seed);
    grid[gridAt(seed)] = 0;
    const active = [0];

    while (active.length > 0) {
      const ai = (Math.random() * active.length) | 0;
      const p = pts[active[ai]];
      let placed = false;
      for (let i = 0; i < k; i++) {
        const a = Math.random() * Math.PI * 2;
        const r = radius * (1 + Math.random());
        const np = [p[0] + Math.cos(a) * r, p[1] + Math.sin(a) * r];
        if (np[0] < 0 || np[0] >= width || np[1] < 0 || np[1] >= height) continue;
        if (!farEnough(np)) continue;
        pts.push(np);
        grid[gridAt(np)] = pts.length - 1;
        active.push(pts.length - 1);
        placed = true;
        break;
      }
      if (!placed) active.splice(ai, 1);
    }
    return pts;
  }

  // ------------------------------------------------------------------------
  // Layer construction
  // ------------------------------------------------------------------------

  const PAD = 0.12;
  const W = 1 + 2 * PAD;

  function buildLayer(spec) {
    const interior = poissonDisk(W, W, spec.radius, 25);
    const points = [];
    const phases = [];
    function add(u, v) {
      points.push([u, v]);
      phases.push(Math.random() * 6.28318);
    }

    // Border points keep the boundary clean.
    const NB = Math.max(2, Math.round(W / spec.radius));
    add(-PAD, -PAD);
    add( 1 + PAD, -PAD);
    add(-PAD,  1 + PAD);
    add( 1 + PAD,  1 + PAD);
    for (let i = 1; i < NB; i++) {
      const u = (i / NB) * W - PAD;
      add(u, -PAD);
      add(u,  1 + PAD);
    }
    for (let j = 1; j < NB; j++) {
      const v = (j / NB) * W - PAD;
      add(-PAD,     v);
      add( 1 + PAD,  v);
    }
    for (const [x, y] of interior) add(x - PAD, y - PAD);

    const N = points.length;
    const triIdx = delaunayTriangulate(points);

    // Filter edges by static length: any edge longer than `maxLen` is most
    // likely a Delaunay anomaly along the boundary or a slim sliver across
    // an empty region. Dropping those keeps the mesh visually local — no
    // stray lines spanning the screen.
    const maxLen2 = (spec.radius * 2.6) * (spec.radius * 2.6);

    const edgeSet = new Set();
    for (let i = 0; i < triIdx.length; i += 3) {
      const a = triIdx[i], b = triIdx[i + 1], c = triIdx[i + 2];
      function maybe(x, y) {
        const dx = points[x][0] - points[y][0];
        const dy = points[x][1] - points[y][1];
        if (dx * dx + dy * dy > maxLen2) return;
        const key = x < y ? x * 1000003 + y : y * 1000003 + x;
        edgeSet.add(key);
      }
      maybe(a, b); maybe(b, c); maybe(c, a);
    }
    const lineIdx = new Array(edgeSet.size * 2);
    let li = 0;
    for (const k of edgeSet) {
      const a = Math.floor(k / 1000003);
      const b = k - a * 1000003;
      lineIdx[li++] = a;
      lineIdx[li++] = b;
    }

    const useU32 = N >= 65536 && !!gl.getExtension('OES_element_index_uint');
    const indices = useU32 ? new Uint32Array(lineIdx) : new Uint16Array(lineIdx);
    const indexType = useU32 ? gl.UNSIGNED_INT : gl.UNSIGNED_SHORT;

    const attribs = new Float32Array(N * 3);
    for (let i = 0; i < N; i++) {
      attribs[i * 3 + 0] = points[i][0];
      attribs[i * 3 + 1] = points[i][1];
      attribs[i * 3 + 2] = phases[i];
    }

    const attrBuf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, attrBuf);
    gl.bufferData(gl.ARRAY_BUFFER, attribs, gl.STATIC_DRAW);

    const idxBuf = gl.createBuffer();
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, idxBuf);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, indices, gl.STATIC_DRAW);

    return {
      attrBuf, idxBuf, indices, indexType,
      spec,
      flowAngle:  Math.random() * Math.PI * 2,
      targetAng:  0,
      nextChange: 0,
      vertCount:  N,
      triCount:   triIdx.length / 3,
      edgeCount:  indices.length / 2,
    };
  }

  // ------------------------------------------------------------------------
  // Shader
  // ------------------------------------------------------------------------

  const VS = `
    precision highp float;
    attribute vec3 a_attr;
    uniform vec2 u_resolution;
    uniform float u_time;
    uniform vec2 u_flow;
    uniform float u_flowAmp;
    uniform float u_localAmp;
    uniform float u_localTime;
    varying float v_mag;
    varying float v_edge;

    float hash(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
    float noise(vec2 p) {
      vec2 i = floor(p), f = fract(p);
      vec2 u = f * f * (3.0 - 2.0 * f);
      float a = hash(i);
      float b = hash(i + vec2(1.0, 0.0));
      float c = hash(i + vec2(0.0, 1.0));
      float d = hash(i + vec2(1.0, 1.0));
      return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
    }
    float fbm(vec2 p) {
      float v = 0.0;
      float amp = 0.5;
      for (int i = 0; i < 3; i++) {
        v += amp * noise(p);
        p *= 2.07;
        amp *= 0.55;
      }
      return v;
    }

    void main() {
      vec2 uv = a_attr.xy;
      float ph = a_attr.z;

      float strength = 0.55 + 0.45 * fbm(uv * 1.7 + u_time * 0.06 + ph);
      vec2 globalOff = u_flow * strength * u_flowAmp;

      vec2 g = uv * 3.6;
      float t = u_localTime;
      float dx = fbm(g + vec2(t * 1.1, -t * 0.7) + ph) - 0.5;
      float dy = fbm(g + vec2(80.0 - t * 0.9, 50.0 + t * 1.0) + ph * 0.7) - 0.5;
      vec2 localOff = vec2(dx, dy) * u_localAmp;

      vec2 p = uv * u_resolution + globalOff + localOff;

      v_mag = clamp(strength * 0.5 + length(localOff) * 0.005, 0.0, 1.0);

      // Horizontal vignette: 0 in the centre column (where the page text
      // lives) and 1 toward the left/right edges. Top and bottom are not
      // affected.
      vec2 uv01 = p / u_resolution;
      float r = abs(uv01.x - 0.5);
      v_edge = smoothstep(0.10, 0.30, r);

      vec2 clip = uv01 * 2.0 - 1.0;
      clip.y = -clip.y;
      gl_Position = vec4(clip, 0.0, 1.0);
    }
  `;

  const FS = `
    precision highp float;
    varying float v_mag;
    varying float v_edge;
    uniform vec3 u_ink;
    uniform float u_alpha;
    void main() {
      float a = (0.08 + 0.92 * v_edge) * (0.75 + 0.25 * v_mag) * u_alpha;
      gl_FragColor = vec4(u_ink, a);
    }
  `;

  function compile(t, s) {
    const sh = gl.createShader(t);
    gl.shaderSource(sh, s);
    gl.compileShader(sh);
    if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
      console.error('[mesh-bg]', gl.getShaderInfoLog(sh));
      return null;
    }
    return sh;
  }

  const prog = gl.createProgram();
  gl.attachShader(prog, compile(gl.VERTEX_SHADER, VS));
  gl.attachShader(prog, compile(gl.FRAGMENT_SHADER, FS));
  gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
    console.error('[mesh-bg] link', gl.getProgramInfoLog(prog));
    return;
  }
  gl.useProgram(prog);

  const aAttr     = gl.getAttribLocation(prog, 'a_attr');
  const uRes      = gl.getUniformLocation(prog, 'u_resolution');
  const uTime     = gl.getUniformLocation(prog, 'u_time');
  const uLocalT   = gl.getUniformLocation(prog, 'u_localTime');
  const uFlow     = gl.getUniformLocation(prog, 'u_flow');
  const uFlowAmp  = gl.getUniformLocation(prog, 'u_flowAmp');
  const uLocalAmp = gl.getUniformLocation(prog, 'u_localAmp');
  const uInk      = gl.getUniformLocation(prog, 'u_ink');
  const uAlpha    = gl.getUniformLocation(prog, 'u_alpha');

  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  gl.disable(gl.DEPTH_TEST);

  // ------------------------------------------------------------------------
  // Layers — back to front
  // ------------------------------------------------------------------------

  // Per-layer parameters. Displacement amplitudes are scaled to the layer's
  // Poisson radius so vertices never wander further than about half a cell
  // — keeps neighbours from crossing and avoids long stretched edges.
  const layers = [
    // Background: sparse, slow drift.
    {
      radius:    0.105 * W,
      ink:       [0.078, 0.078, 0.078],
      alpha:     0.95,
      flowAmp:   105,
      localAmp:  130,
      flowSpeed: 0.30,
      localRate: 0.28,
    },
    // Middle: medium density.
    {
      radius:    0.068 * W,
      ink:       [0.078, 0.078, 0.078],
      alpha:     0.85,
      flowAmp:   70,
      localAmp:  85,
      flowSpeed: 0.50,
      localRate: 0.40,
    },
    // Foreground: dense, lively, but smaller absolute displacement.
    {
      radius:    0.040 * W,
      ink:       [0.078, 0.078, 0.078],
      alpha:     0.70,
      flowAmp:   45,
      localAmp:  55,
      flowSpeed: 0.80,
      localRate: 0.55,
    },
  ].map(buildLayer);

  // ------------------------------------------------------------------------
  // Per-layer flow update
  // ------------------------------------------------------------------------

  function updateLayerFlow(layer, now, dt) {
    if (now >= layer.nextChange) {
      const turn = (Math.random() - 0.5) * (Math.PI * 1.1);
      layer.targetAng = layer.flowAngle + turn;
      layer.nextChange = now + 4000 + Math.random() * 5000;
    }
    let diff = layer.targetAng - layer.flowAngle;
    diff = Math.atan2(Math.sin(diff), Math.cos(diff));
    layer.flowAngle += diff * (1 - Math.exp(-layer.spec.flowSpeed * dt));
  }

  // ------------------------------------------------------------------------
  // Resize
  // ------------------------------------------------------------------------

  function resize() {
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    canvas.width  = window.innerWidth  * dpr;
    canvas.height = window.innerHeight * dpr;
    canvas.style.width  = window.innerWidth  + 'px';
    canvas.style.height = window.innerHeight + 'px';
    gl.viewport(0, 0, canvas.width, canvas.height);
    gl.uniform2f(uRes, canvas.width, canvas.height);
  }
  window.addEventListener('resize', resize);
  resize();

  // ------------------------------------------------------------------------
  // Render loop
  // ------------------------------------------------------------------------

  const t0 = performance.now();
  let lastNow = t0;
  function frame(now) {
    const t  = (now - t0) / 1000;
    const dt = Math.max(0.001, (now - lastNow) / 1000);
    lastNow = now;

    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.uniform1f(uTime, t);

    for (const layer of layers) {
      updateLayerFlow(layer, now, dt);

      gl.bindBuffer(gl.ARRAY_BUFFER, layer.attrBuf);
      gl.enableVertexAttribArray(aAttr);
      gl.vertexAttribPointer(aAttr, 3, gl.FLOAT, false, 0, 0);
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, layer.idxBuf);

      gl.uniform2f(uFlow, Math.cos(layer.flowAngle), Math.sin(layer.flowAngle));
      gl.uniform1f(uFlowAmp,  layer.spec.flowAmp);
      gl.uniform1f(uLocalAmp, layer.spec.localAmp);
      gl.uniform1f(uLocalT,   t * layer.spec.localRate);
      gl.uniform3f(uInk,      layer.spec.ink[0], layer.spec.ink[1], layer.spec.ink[2]);
      gl.uniform1f(uAlpha,    layer.spec.alpha);

      gl.drawElements(gl.LINES, layer.indices.length, layer.indexType, 0);
    }

    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);

  console.log('[mesh-bg]', layers.length, 'layers:',
    layers.map((l) => `${l.vertCount}v/${l.edgeCount}e`).join(', '));
})();
