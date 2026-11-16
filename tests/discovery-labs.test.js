import { test, expect } from 'bun:test';
import { builders, presets, rotate } from '../frontend/lessons/discovery-labs.js';
test('every discovery preset produces four usable steps',()=>{
  for(const [kind,examples] of Object.entries(presets)) for(const example of examples){const steps=builders[kind](example.values);expect(steps).toHaveLength(4);expect(steps.every(s=>s.title&&s.text)).toBe(true);}
});
test('classification distinguishes unknown from incorrect',()=>{
  expect(builders['word-sorting'](['tulip','tulip','Flowers']).at(-1).text).toContain('matches');
  expect(builders['word-sorting'](['cloud','cloud','Flowers']).at(-1).text).toContain('cannot verify');
});
test('sequence reasoning checks every visible term and constrains families',()=>{
  expect(builders['sequence-detective'](['2 5 ? 11 14','addition','8']).at(-1).text).toContain('fits');
  expect(builders['sequence-detective'](['2 5 ? 12 14','addition','8'])).toHaveLength(1);
  expect(builders['sequence-detective'](['2 6 ? 54 162','multiplication','18']).at(-1).text).toContain('fits');
  expect(builders['sequence-detective'](['4 9 ? 25 36','squares','16']).at(-1).text).toContain('fits');
  expect(builders['sequence-detective'](['4 9 ? 24 36','squares','16'])).toHaveLength(1);
});
test('card matching needs distinct positions and an actual pair deck',()=>{
  expect(builders['memory-pairs'](['acorn star acorn star','0','2'])[2].text).toContain('faces match');
  for(const input of [['acorn star','0','1'],['acorn acorn','0','0'],['acorn acorn','','1']])expect(builders['memory-pairs'](input)).toHaveLength(1);
});
test('pipe rotation preserves ports and distinguishes reciprocal edges',()=>{
  for(let mask=1;mask<=15;mask++){let result=mask;for(let i=0;i<4;i++)result=rotate(result);expect(result).toBe(mask);}
  expect(builders['path-connections'](['3','1','12'])[2].text).toContain('meets');
  expect(builders['path-connections'](['2','0','1'])[2].text).toContain('leaks');
});
test('Latin rows, columns and inequalities supply separate evidence',()=>{
  const good=presets.futoshiki[0].values;
  expect(builders.futoshiki(good).at(-1).text).toContain('satisfies');
  expect(builders.futoshiki(['1 1 3 4;2 3 4 1;3 4 1 2;4 1 2 3',good[1]])[0].rows[1][2]).toBe('Repeated number');
  expect(builders.futoshiki([good[0],'r1c1>r1c2'])[2].rows[1][1]).toBe('Violated');
  expect(builders.futoshiki(['1 2','r1c1<r1c2'])).toHaveLength(1);
});
test('rhyming and grammar respect the reviewed model rather than guessed meaning',()=>{
  expect(builders['sound-families'](['light','night kite white rain','night kite white']).at(-1).text).toContain('every reviewed match');
  expect(builders['sound-families'](['light','night unknown','night']).at(-1).text).toContain('cannot certify');
  expect(builders['sentence-assembly'](['quietly the curious rabbit rests']).at(-1).text).toContain('fits');
  expect(builders['sentence-assembly'](['rabbit the curious rests quietly']).at(-1).text).toContain('outside');
});
