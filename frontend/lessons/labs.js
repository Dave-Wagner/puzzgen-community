import { presets as languagePresets, builders as languageBuilders } from './lab-core.js';
import { presets as discoveryPresets, builders as discoveryBuilders, labels as discoveryLabels, multiline as discoveryMultiline } from './discovery-labs.js';
const presets={...languagePresets,...discoveryPresets};
const builders={...languageBuilders,...discoveryBuilders};

const root = document.querySelector('[data-language-lab]');
if (root) {
  const kind = root.dataset.languageLab;
  const examples = presets[kind];
  const labels = discoveryLabels[kind] ?? {
    spoonerism: ['First onset (sound)', 'First remainder (sounds)', 'Second onset (sound)', 'Second remainder (sounds)'],
    anagram: ['Source word', 'Candidate rearrangement'],
    'word-ladder': ['Start word', 'End word', 'Allowed vocabulary (space-separated)', 'Your path (space-separated, include endpoints)'],
    'compound-bridge': ['Left word', 'Middle word', 'Right word'],
  }[kind];
  const element = (tag, text, parent) => {
    const node = document.createElement(tag);
    if (text !== undefined) node.textContent = text;
    parent?.append(node);
    return node;
  };
  element('h2', 'Experiment, one step at a time', root);
  element('p', 'Change an example, then use Next step to inspect the rule. This lab runs entirely in your browser and makes no API requests.', root);
  const exampleLabel = element('label', 'Choose an example', root);
  exampleLabel.htmlFor = 'lab-example';
  const select = element('select', undefined, root);
  select.id = 'lab-example';
  examples.forEach((example, index) => { const option = element('option', example.name, select); option.value = String(index); });
  const fields = element('div', undefined, root);
  fields.className = 'lab-fields';
  const inputs = labels.map((label, index) => {
    const wrapper = element('div', undefined, fields);
    const caption = element('label', label, wrapper);
    caption.htmlFor = `lab-field-${index}`;
    const multiline = (kind === 'word-ladder' && index >= 2) || discoveryMultiline(kind,index);
    const input = element(multiline ? 'textarea' : 'input', undefined, wrapper);
    input.id = caption.htmlFor;
    input.maxLength = multiline ? 600 : 24;
    input.spellcheck = false;
    input.autocomplete = 'off';
    if (multiline) input.rows = 3;
    input.addEventListener('input', () => { step = 0; render(); });
    return input;
  });
  if (kind === 'spoonerism') element('p', 'Sound blocks are explicit: this lab does not guess phonemes from typed spelling. Changing a block creates a sound-only experiment, not a dictionary-checked phrase.', root);
  if (kind === 'word-ladder') element('p', 'Use at most 40 equal-length words. You define this lab’s vocabulary; unfamiliar or invented entries are not dictionary-checked.', root);
  const controls = element('div', undefined, root);
  controls.className = 'lab-controls';
  const previous = element('button', 'Previous step', controls);
  const next = element('button', 'Next step', controls);
  const restart = element('button', 'Restart steps', controls);
  const reset = element('button', 'Reset example', controls);
  for (const button of [previous, next, restart, reset]) button.type = 'button';
  const progress = element('p', undefined, root);
  progress.className = 'lab-progress';
  progress.setAttribute('role', 'status');
  const output = element('div', undefined, root);
  output.className = 'lab-output';
  let step = 0;
  let steps = [];
  function render() {
    steps = builders[kind](inputs.map(input => input.value), examples[Number(select.value)]);
    step = Math.min(step, steps.length - 1);
    previous.disabled = step === 0;
    next.disabled = step === steps.length - 1;
    progress.textContent = `Step ${step + 1} of ${steps.length}: ${steps[step].title}`;
    output.replaceChildren();
    const current = steps[step];
    const heading = element('h3', current.title, output);
    heading.id = 'lab-step-title';
    output.setAttribute('aria-labelledby', heading.id);
    element('p', current.text, output);
    if (current.tokens?.length) {
      const tokens = element('div', undefined, output);
      tokens.className = 'lab-tokens';
      current.tokens.forEach(text => element('span', text, tokens));
    }
    if (current.rows) {
      const scroller = element('div', undefined, output);
      scroller.className = 'lab-table-scroll';
      const table = element('table', undefined, scroller);
      element('caption', current.title, table);
      const head = element('thead', undefined, table);
      const header = element('tr', undefined, head);
      current.rows[0].forEach(text => { const th = element('th', text, header); th.scope = 'col'; });
      const body = element('tbody', undefined, table);
      current.rows.slice(1).forEach(row => { const tr = element('tr', undefined, body); row.forEach(text => element('td', text, tr)); });
    }
  }
  function loadExample() {
    examples[Number(select.value)].values.forEach((value, index) => { inputs[index].value = value; });
    step = 0;
    render();
  }
  select.addEventListener('change', loadExample);
  previous.addEventListener('click', () => { step = Math.max(0, step - 1); render(); });
  next.addEventListener('click', () => { step = Math.min(steps.length - 1, step + 1); render(); });
  restart.addEventListener('click', () => { step = 0; render(); });
  reset.addEventListener('click', loadExample);
  loadExample();
}
