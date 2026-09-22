import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

test('desktop release preserves Debug formatting used to generate GPU shader source', () => {
  const workflow = readFileSync(new URL('../.github/workflows/release.yml', import.meta.url), 'utf8');
  assert.equal(/-Z\s*fmt-debug(?:=|\s+)(?:none|shallow)/.test(workflow), false,
    'Naga uses Debug formatting for HLSL floating-point literals; stripping it corrupts Windows shaders');
});
