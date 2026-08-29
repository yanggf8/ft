#!/usr/bin/env node
// gen-stars.js — throwaway generator for the cosmic-silver star field
// (spec §3). Run once from crates/web: node scripts/gen-stars.js
// Output is pasted into style.css under a "generated — do not edit" banner.
'use strict';

function mulberry32(a) {
  return function () {
    a |= 0; a = (a + 0x6D2B79F5) | 0;
    var t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function layer(count, dim) {
  var rnd = mulberry32(dim ? 0x5EEDB : 0x5EEDA);
  var parts = [];
  for (var i = 0; i < count; i++) {
    var x = (rnd() * 100).toFixed(2);
    var y = (rnd() * 100).toFixed(2);
    var a = (dim ? 0.35 + rnd() * 0.4 : 0.5 + rnd() * 0.5).toFixed(2);
    parts.push(x + 'vw ' + y + 'vh 0 0 rgba(229,228,226,' + a + ')');
  }
  if (parts.length !== count) throw new Error('bad count'); // sanity
  return parts.join(',\n  ');
}

var a = layer(30, false); // .sky-stars-a — 1px, brighter
var b = layer(30, true);  // .sky-stars-b — 2px, dimmer
if (!/^[\d.]+vw/.test(a) || !/^[\d.]+vw/.test(b)) throw new Error('bad format');
console.log('-- STARS-A --\n' + a + '\n-- STARS-B --\n' + b);
