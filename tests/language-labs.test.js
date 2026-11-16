import { test, expect } from 'bun:test';
import { presets, builders, anagramLesson, spoonLesson, ladderLesson, bridgeLesson, validatePath, adjacent } from '../frontend/lessons/lab-core.js';

test('every preset produces a complete, finite walkthrough', () => {
  for (const [kind, examples] of Object.entries(presets)) for (const preset of examples) {
    const steps = builders[kind](preset.values, preset);
    expect(steps.length).toBeGreaterThanOrEqual(4);
    expect(steps.length).toBeLessThan(45);
    expect(steps.every(step => step.title && step.text)).toBe(true);
  }
});
test('sound swaps keep each remainder in place and move whole clusters', () => {
  const preset = presets.spoonerism[2];
  expect(spoonLesson(preset.values, preset)[3].tokens).toEqual(['/krʌʃɪŋ/', '/bloʊ/']);
  expect(spoonLesson(['z', 'aɪt', 'r', 'eɪn'], presets.spoonerism[0])[3].text).toContain('not a dictionary-validated');
  expect(spoonLesson(['l', '', 'r', 'eɪn'], presets.spoonerism[0])).toHaveLength(1);
});
test('anagram counts preserve duplicate letters and distinguish identity', () => {
  expect(anagramLesson(['LISTEN', ' silent '])[3].text).toContain('All letter counts match');
  const mismatch = anagramLesson(['letter', 'teller']);
  expect(mismatch[1].rows.find(row => row[0] === 'l')).toEqual(['l', '1', '2', 'Different']);
  expect(mismatch[3].text).toContain('counts differ');
  expect(anagramLesson(['letter', 'letter'])[3].text).toContain('same word');
  expect(anagramLesson(['a'.repeat(25), 'a'])).toHaveLength(1);
  expect(anagramLesson(['two words', 'words two'])).toHaveLength(1);
});
test('BFS finds shortest routes and reports disconnected components', () => {
  const steps = ladderLesson(presets['word-ladder'][0].values);
  const result = steps.find(step => step.title === 'A shortest route');
  expect(result.tokens).toHaveLength(4);
  expect(result.tokens.slice(1).every((word, i) => adjacent(result.tokens[i], word))).toBe(true);
  expect(steps.at(-1).text).toContain('Valid path');
  expect(ladderLesson(presets['word-ladder'][2].values).some(step => step.title === 'No connecting route')).toBe(true);
});
test('path checks accept alternatives and reject jumps, repeats and absent words', () => {
  const words = ['cat', 'cot', 'cog', 'dog', 'dot'];
  for (const path of [['cat', 'cot', 'cog', 'dog'], ['cat', 'cot', 'dot', 'dog']]) expect(validatePath(path, 'cat', 'dog', words)).toContain('Valid path');
  expect(validatePath(['cat', 'dog'], 'cat', 'dog', words)).toContain('exactly one');
  expect(validatePath(['cat', 'cot', 'cat', 'dog'], 'cat', 'dog', words)).toContain('repeat');
  expect(validatePath(['cat', 'bat', 'dog'], 'cat', 'dog', words)).toContain('missing');
  expect(validatePath(['cot', 'dog'], 'cat', 'dog', words)).toContain('start and end');
});
test('graph editing rejects ambiguous lengths and bounds workload', () => {
  for (const values of [
    ['cat', 'cat', 'cat cot', 'cat'],
    ['cat', 'dog', 'cat cot', 'cat dog'],
    ['cat', 'dog', 'cat dog pond', 'cat dog'],
    ['cat', 'dog', Array(41).fill('cat').join(' '), 'cat dog'],
  ]) expect(ladderLesson(values)).toHaveLength(1);
});
test('compound checks examine both sides without claiming a complete dictionary', () => {
  expect(bridgeLesson(['sun', 'flower', 'pot'])[3].text).toContain('Both combinations');
  const incomplete = bridgeLesson(['sun', 'flower', 'tie']);
  expect(incomplete[1].text).toContain('In this lab’s reviewed collection');
  expect(incomplete[2].text).toContain('does not prove it is invalid');
  expect(incomplete[3].text).toContain('outside this lab');
  expect(bridgeLesson(['sun', '', 'pot'])).toHaveLength(1);
});
