#!/usr/bin/env node
// Record what node-semver does with the inputs of its own test suite.
//
//   node scripts/capture-node-semver.js <node-semver checkout> > calls.json
//
// node-semver's fixtures and test tables are JavaScript, not data. Rather
// than transcribe them, this runs node-semver's tests (test/functions and
// test/ranges) with a stand-in for `tap` whose assertions do nothing, and
// with node-semver's exported functions wrapped: every call a test makes
// (not node-semver's internal calls) is recorded with its arguments and
// its result or exception. It then adds a cross product of every range
// and version those calls used, under all four option combinations
// (validRange, minVersion, and satisfies for each version).
//
// tests/npm.rs replays the calls against this crate's `npm` module.
'use strict'

const Module = require('module')
const path = require('path')
const fs = require('fs')

// Real path: module filenames are resolved through symlinks.
const dir = fs.realpathSync(path.resolve(process.argv[2]))

// ---------------------------------------------------------------------------
// Recording

const calls = []
const seen = new Set()
let depth = 0

const SemVerFile = path.join(dir, 'classes/semver.js')
const RangeFile = path.join(dir, 'classes/range.js')
const ComparatorFile = path.join(dir, 'classes/comparator.js')

const ser = (x) => {
  if (x === undefined) return { $: 'undefined' }
  if (x === null || typeof x === 'boolean' || typeof x === 'string') return x
  if (typeof x === 'number') return Number.isFinite(x) ? x : { $: String(x) }
  if (Array.isArray(x)) return x.map(ser)
  if (x instanceof RegExp) return { $: 'regexp' }
  if (x instanceof Error) return { $error: x.message }
  const name = x.constructor && x.constructor.name
  if (name === 'SemVer') return { $semver: x.version, build: x.build.join('.') }
  if (name === 'Range') return { $range: x.range, raw: x.raw }
  if (name === 'Comparator') return { $comparator: x.value }
  if (typeof x === 'object' && Object.getPrototypeOf(x) === Object.prototype) {
    const o = {}
    for (const [k, v] of Object.entries(x)) o[k] = ser(v)
    return o
  }
  return { $: 'object' }
}

const record = (f, args, threw, result) => {
  const entry = { f, a: args.map(ser), r: threw ? { $throws: String(result && result.message) } : ser(result) }
  const key = JSON.stringify(entry)
  if (!seen.has(key)) {
    seen.add(key)
    calls.push(entry)
  }
}

const wrap = (name, fn) => function (...args) {
  const top = depth === 0
  depth++
  let result
  let threw = false
  try {
    result = fn.apply(this, args)
  } catch (e) {
    threw = true
    result = e
  } finally {
    depth--
  }
  if (top) record(name, args, threw, result)
  if (threw) throw result
  return result
}

// ---------------------------------------------------------------------------
// A stand-in for `tap`

const noop = () => true
const runTest = (name, opts, fn) => {
  fn = [name, opts, fn].find(x => typeof x === 'function')
  if (fn) {
    try {
      fn(makeT())
    } catch (e) {
      process.stderr.write(`test threw: ${e && e.message}\n`)
    }
  }
}
const makeT = () => new Proxy({}, {
  get (_, prop) {
    if (prop === 'test') return runTest
    if (prop === 'throws' || prop === 'doesNotThrow') {
      return (fn) => {
        if (typeof fn === 'function') {
          try { fn() } catch { /* expected */ }
        }
      }
    }
    if (prop === 'then') return undefined
    return noop
  },
})
const tap = makeT()

// ---------------------------------------------------------------------------
// Load node-semver's modules wrapped, and run its tests

const wrapped = new Map()
const load = Module._load
Module._load = function (request, parent, isMain) {
  if (request === 'tap') return tap
  const exports = load.apply(this, arguments)
  let file
  try {
    file = Module._resolveFilename(request, parent, isMain)
  } catch {
    return exports
  }
  const rel = path.relative(dir, file)
  if (typeof exports === 'function' && /^(functions|ranges)[/\\][^/\\]+\.js$/.test(rel)) {
    const name = rel.replace(/\\/g, '/').replace(/\.js$/, '')
    if (!wrapped.has(file)) wrapped.set(file, wrap(name, exports))
    return wrapped.get(file)
  }
  return exports
}

for (const sub of ['functions', 'ranges']) {
  const testDir = path.join(dir, 'test', sub)
  for (const f of fs.readdirSync(testDir).sort()) {
    if (f.endsWith('.js')) require(path.join(testDir, f))
  }
}

// ---------------------------------------------------------------------------
// Cross product: every range and version the tests used, all options.
// Stored as one row of results per range and options, not as calls.

const fn = (name) => require(path.join(dir, name))
const satisfies = fn('functions/satisfies.js')
const validRange = fn('ranges/valid.js')
const minVersion = fn('ranges/min-version.js')
const valid = fn('functions/valid.js')

const ranges = new Set()
const versions = new Set()
const rangeArgs = {
  'functions/satisfies': [1],
  'ranges/valid': [0],
  'ranges/min-version': [0],
  'ranges/max-satisfying': [1],
  'ranges/min-satisfying': [1],
  'ranges/gtr': [1],
  'ranges/ltr': [1],
  'ranges/outside': [1],
  'ranges/intersects': [0, 1],
  'ranges/subset': [0, 1],
  'ranges/to-comparators': [0],
}
const versionArgs = {
  'functions/satisfies': [0],
  'functions/valid': [0],
  'functions/parse': [0],
  'functions/inc': [0],
  'ranges/gtr': [0],
  'ranges/ltr': [0],
  'ranges/outside': [0],
}
for (const c of calls) {
  for (const i of rangeArgs[c.f] || []) {
    if (typeof c.a[i] === 'string') ranges.add(c.a[i])
  }
  for (const i of versionArgs[c.f] || []) {
    if (typeof c.a[i] === 'string' && c.a[i].length <= 256) versions.add(c.a[i])
  }
}
// The probe versions, as strict versions (their canonical text).
const probe = [...new Set([...versions].map(v => valid(v)).filter(v => v))].sort()
const allOptions = [{}, { loose: true }, { includePrerelease: true }, { loose: true, includePrerelease: true }]
const rows = []
depth++ // don't record the calls below
for (const r of [...ranges].sort()) {
  allOptions.forEach((o, k) => {
    let parsed = null
    try {
      parsed = validRange(r, o)
    } catch { /* not a string */ }
    let min = null
    try {
      const m = minVersion(r, o)
      min = m && m.version
    } catch { /* invalid range */ }
    const bits = probe.map(v => satisfies(v, r, o) ? '1' : '0').join('')
    rows.push([r, k, parsed, min, bits])
  })
}
depth--

process.stdout.write(JSON.stringify({ calls, matrix: { versions: probe, rows } }) + '\n')
process.stderr.write(`recorded ${calls.length} calls and ${rows.length} rows x ${probe.length} versions\n`)
