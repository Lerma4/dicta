import { readFileSync, writeFileSync } from 'node:fs'
import assert from 'node:assert/strict'
import process from 'node:process'

export function csv(result) {
  assert.equal(result.inference_ms.length, 5, 'Five trials are required')
  assert(result.inference_ms.every(x => Number.isFinite(x) && x >= 0))
  assert(Number.isFinite(result.audio_ms) && result.audio_ms > 0)
  return 'trial,inference_ms,audio_ms,real_time_factor\n' + result.inference_ms.map((ms, i) => `${i + 1},${ms},${result.audio_ms},${ms / result.audio_ms}`).join('\n') + '\n'
}

if (process.argv[2] === '--self-test') {
  assert(csv({ inference_ms: [1, 2, 3, 4, 5], audio_ms: 10 }).includes('1,1,10,0.1'))
  assert.throws(() => csv({ inference_ms: [1], audio_ms: 10 }))
} else if (process.argv[2]) {
  writeFileSync(process.argv[2].replace(/\.json$/, '') + '.csv', csv(JSON.parse(readFileSync(process.argv[2], 'utf8'))))
}
