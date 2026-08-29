// galaxy.js — cosmic-silver Milky-Way particle layer (spec §4).
// Standalone, dependency-free, defer-loaded. Every failure path bails
// silently: the CSS star field must stand alone (spec §3).
(function () {
  'use strict';

  var mql = window.matchMedia('(prefers-reduced-motion: reduce)');
  if (mql.matches) return;

  var canvas = document.getElementById('galaxy-canvas');
  if (!canvas) return;
  var ctx;
  try { ctx = canvas.getContext('2d'); } catch (err) { return; }
  if (!ctx) return;

  var BAND_RAD = (-18 * Math.PI) / 180;
  var COS = Math.cos(BAND_RAD), SIN = Math.sin(BAND_RAD);
  var ABS_COS = Math.abs(COS), ABS_SIN = Math.abs(SIN);
  var SIGMA_FRAC = 0.12;      // band width, fraction of viewport diagonal
  var AREA_PER_STAR = 14400;  // px^2 per band particle
  var COUNT_MIN = 24, COUNT_MAX = 150;
  var SCATTER = 30;           // uniform stars outside the band
  var DPR_CAP = 2;

  var W = 0, H = 0, sigma = 0, axMax = 0, ayMax = 0;
  var parts = [];
  var raf = 0, last = 0, running = false;
  var resizeTimer = 0;
  var meteor = null, nextMeteorAt = 0;

  function rand(a, b) { return a + Math.random() * (b - a); }

  // Box-Muller with 3-sigma rejection (spec §4).
  function gauss() {
    var u, v, s, g;
    do {
      u = Math.random() * 2 - 1;
      v = Math.random() * 2 - 1;
      s = u * u + v * v;
      if (s === 0 || s >= 1) continue;
      g = u * Math.sqrt((-2 * Math.log(s)) / s);
    } while (g > 3 || g < -3);
    return g;
  }

  function resize() {
    W = window.innerWidth;
    H = window.innerHeight;
    var dpr = Math.min(window.devicePixelRatio || 1, DPR_CAP);
    canvas.width = Math.round(W * dpr);
    canvas.height = Math.round(H * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    sigma = Math.sqrt(W * W + H * H) * SIGMA_FRAC;
    var pad = 3 * sigma;
    // viewport extents along/across the band axes, inflated by 3 sigma:
    axMax = (W * ABS_COS + H * ABS_SIN) / 2 + pad;
    ayMax = (W * ABS_SIN + H * ABS_COS) / 2 + pad;
    seed();
    nextMeteorAt = performance.now() + rand(8000, 20000);
    draw(performance.now());
  }

  // Full resample on resize — one visible reshuffle frame, accepted (spec §4).
  function seed() {
    parts.length = 0;
    var n = Math.round(Math.min(COUNT_MAX, Math.max(COUNT_MIN, (W * H) / AREA_PER_STAR)));
    var i;
    for (i = 0; i < n; i++) parts.push(bandParticle());
    for (i = 0; i < SCATTER; i++) parts.push(scatterParticle());
  }

  function bandParticle() {
    return {
      ax: rand(-axMax, axMax),   // along-band coordinate (float, always)
      ay: gauss() * sigma,       // perpendicular Gaussian offset
      vx: (Math.random() < 0.5 ? -1 : 1) * rand(2, 6), // px/s along band
      r: rand(0.4, 1.6),
      a0: rand(0.3, 0.9),
      phase: rand(0, Math.PI * 2),
      tw: rand(2, 6)             // twinkle period, seconds
    };
  }

  function scatterParticle() { // uniform inside the viewport rect
    var ax, ay, x, y;
    do {
      ax = rand(-axMax, axMax);
      ay = rand(-ayMax, ayMax);
      x = ax * COS - ay * SIN;
      y = ax * SIN + ay * COS;
    } while (x < -W / 2 || x > W / 2 || y < -H / 2 || y > H / 2);
    var p = bandParticle();
    p.ax = ax;
    p.ay = ay;
    return p;
  }

  // Toroidal wrap in the inflated band domain — never the viewport rect
  // (spec §4: viewport wrap would pop stars out mid-band).
  function wrap(p) {
    if (p.ax > axMax) p.ax -= axMax * 2;
    else if (p.ax < -axMax) p.ax += axMax * 2;
    if (p.ay > ayMax) p.ay -= ayMax * 2;
    else if (p.ay < -ayMax) p.ay += ayMax * 2;
  }

  // Shooting stars (spec §4): one at a time, random 8-20 s gap, along the
  // band within +/-25deg, ~900 px/s, ~0.9 s life, ~120 px gradient tail.
  function spawnMeteor(now) {
    var ang = (rand(-25, 25) * Math.PI) / 180;
    var dir = Math.random() < 0.5 ? -1 : 1;
    meteor = {
      ax: rand(-axMax * 0.9, axMax * 0.9),
      ay: gauss() * sigma * 0.5,
      vx: dir * Math.cos(ang) * 900,
      vy: Math.sin(ang) * 900,
      born: now,
      ttl: 0.9
    };
  }

  function updateMeteor(now, dt) {
    if (!meteor) {
      if (now >= nextMeteorAt) {
        spawnMeteor(now);
        nextMeteorAt = now + rand(8000, 20000);
      }
      return;
    }
    meteor.ax += meteor.vx * dt;
    meteor.ay += meteor.vy * dt;
    if ((now - meteor.born) / 1000 > meteor.ttl) meteor = null;
  }

  function drawMeteor(now) {
    if (!meteor) return;
    var age = (now - meteor.born) / (meteor.ttl * 1000);
    if (age < 0 || age > 1) { meteor = null; return; }
    var ease = age < 0.15 ? age / 0.15 : (1 - age) / 0.85;
    var x = W / 2 + meteor.ax * COS - meteor.ay * SIN;
    var y = H / 2 + meteor.ax * SIN + meteor.ay * COS;
    var sp = Math.sqrt(meteor.vx * meteor.vx + meteor.vy * meteor.vy);
    var tx = x - (meteor.vx / sp) * 120;
    var ty = y - (meteor.vy / sp) * 120;
    var grad = ctx.createLinearGradient(x, y, tx, ty);
    grad.addColorStop(0, 'rgba(245,247,250,' + (0.9 * Math.max(ease, 0)).toFixed(3) + ')');
    grad.addColorStop(1, 'rgba(245,247,250,0)');
    ctx.strokeStyle = grad;
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(x, y);
    ctx.lineTo(tx, ty);
    ctx.stroke();
  }

  function step(dt) {
    var i, p;
    for (i = 0; i < parts.length; i++) {
      p = parts[i];
      p.ax += p.vx * dt; // float accumulation — sub-pixel speeds
      wrap(p);
    }
  }

  function draw(now) {
    ctx.clearRect(0, 0, W, H);
    ctx.fillStyle = '#f5f7fa';
    var i, p, t = now / 1000, x, y;
    for (i = 0; i < parts.length; i++) {
      p = parts[i];
      x = W / 2 + p.ax * COS - p.ay * SIN;
      y = H / 2 + p.ax * SIN + p.ay * COS;
      ctx.globalAlpha = p.a0 * (0.55 + 0.45 * Math.sin(p.phase + (t * Math.PI * 2) / p.tw));
      ctx.fillRect(x - p.r / 2, y - p.r / 2, p.r, p.r);
    }
    drawMeteor(now);
    ctx.globalAlpha = 1;
  }

  function frame(now) {
    if (!running) return;
    var dt = Math.min((now - last) / 1000, 0.05);
    last = now;
    step(dt);
    updateMeteor(now, dt);
    draw(now);
    raf = requestAnimationFrame(frame);
  }

  function start() {
    if (running) return;
    running = true;
    last = performance.now();
    raf = requestAnimationFrame(frame);
  }

  function stop() {
    running = false;
    cancelAnimationFrame(raf);
  }

  window.addEventListener('resize', function () {
    clearTimeout(resizeTimer);
    resizeTimer = setTimeout(resize, 200);
  });

  document.addEventListener('visibilitychange', function () {
    if (document.hidden) stop();
    else start();
  });

  function onReducedChange(ev) {
    if (ev.matches) {
      stop();
      ctx.clearRect(0, 0, W, H);
    } else {
      start();
    }
  }
  if (mql.addEventListener) mql.addEventListener('change', onReducedChange);

  try {
    resize();
    start();
  } catch (err) {
    stop();
  }
})();
