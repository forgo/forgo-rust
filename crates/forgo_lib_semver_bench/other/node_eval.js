// Evaluate npm ranges with node-semver itself, for the report's
// comparison.
//
//   node other/node_eval.js <node-semver checkout> <cases.json> > results.json
//
// cases.json: { "ranges": [...], "versions": [...] }. For each range, the
// output has `validRange(range)` (null if invalid) and a string of 0s and
// 1s: `satisfies(version, range)` for each version.
'use strict'
const fs = require('fs')
const path = require('path')
const semver = require(path.resolve(process.argv[2]))
const { ranges, versions } = JSON.parse(fs.readFileSync(process.argv[3], 'utf8'))
const out = ranges.map(r => {
  const valid = semver.validRange(r)
  const bits = versions.map(v => (semver.satisfies(v, r) ? '1' : '0')).join('')
  return { valid, bits }
})
process.stdout.write(JSON.stringify(out))
