// Tests for the PR label mapping inlined in .github/workflows/pr-labels.yml.
//
// The mapping is inlined (not required from a file) because the workflow runs
// on pull_request_target and must never check out the repository. This test
// extracts the code between the BEGIN/END markers and runs it, so the tested
// code is exactly the code the workflow runs.
//
// Run with: node --test .github/scripts/

const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const GITHUB_DIR = path.join(__dirname, '..');
const workflow = fs.readFileSync(path.join(GITHUB_DIR, 'workflows', 'pr-labels.yml'), 'utf8');

function extractMapping(source) {
  const lines = source.split('\n');
  const begin = lines.findIndex((l) => l.trim() === '// BEGIN label-mapping');
  const end = lines.findIndex((l) => l.trim() === '// END label-mapping');
  assert.ok(begin >= 0 && end > begin, 'label-mapping markers not found in pr-labels.yml');
  const code = lines.slice(begin + 1, end).join('\n');
  return new Function(`${code}\nreturn { CHANGELOG_LABELS, parseTitle, planLabels };`)();
}

const { CHANGELOG_LABELS, parseTitle, planLabels } = extractMapping(workflow);

// Labels the workflow would add to a freshly opened PR.
function labelsFor(title, body = '') {
  return planLabels({ action: 'opened', title, body, current: [] }).add;
}

test('each type maps to its label', () => {
  const cases = {
    feat: 'enhancement',
    perf: 'enhancement',
    fix: 'bug',
    docs: 'documentation',
    deps: 'dependencies',
    ci: 'internal',
    chore: 'internal',
    refactor: 'internal',
    test: 'internal',
    style: 'internal',
    build: 'internal',
    revert: 'internal',
  };
  for (const [type, label] of Object.entries(cases)) {
    assert.deepEqual(labelsFor(`${type}: do something`), [label], type);
  }
});

test('scoped titles', () => {
  assert.deepEqual(labelsFor('feat(session): add builder'), ['enhancement']);
  assert.deepEqual(labelsFor('fix(transport/tcp): handle EOF'), ['bug']);
  assert.deepEqual(labelsFor('docs(): empty scope'), ['documentation']);
});

test('deps scope maps to dependencies for any type', () => {
  assert.deepEqual(labelsFor('chore(deps): bump actions/checkout from 4 to 5'), ['dependencies']);
  assert.deepEqual(labelsFor('build(deps): bump serde to 1.0.200'), ['dependencies']);
  assert.deepEqual(labelsFor('chore(deps-dev): bump foo'), ['dependencies']);
  assert.deepEqual(labelsFor('chore(DEPS): bump foo'), ['dependencies']);
});

test('! before the colon adds breaking-change', () => {
  assert.deepEqual(labelsFor('feat!: drop std support'), ['enhancement', 'breaking-change']);
  assert.deepEqual(labelsFor('refactor(api)!: rename Session'), ['internal', 'breaking-change']);
});

test('type is matched case-insensitively', () => {
  assert.deepEqual(labelsFor('FEAT: shout'), ['enhancement']);
  assert.deepEqual(labelsFor('Fix(Core): typo'), ['bug']);
});

test('non-matching titles get no label', () => {
  for (const title of [
    'Add a feature',
    'feat:no space after colon',
    'feat add colon',
    'feat (scope): space before scope',
    'Revert "feat: x"',
    'wip: unknown type',
    'constructor: not an own property',
    '',
  ]) {
    assert.deepEqual(labelsFor(title), [], JSON.stringify(title));
  }
  assert.deepEqual(labelsFor(undefined), []);
});

test('BREAKING CHANGE in the body adds breaking-change', () => {
  assert.deepEqual(
    labelsFor('feat: new api', 'Some text.\r\n\r\nBREAKING CHANGE: Session::open is gone'),
    ['enhancement', 'breaking-change'],
  );
  assert.deepEqual(labelsFor('fix: x', 'BREAKING-CHANGE: also accepted'), ['bug', 'breaking-change']);
  // Only as a footer at the start of a line, and only uppercase.
  assert.deepEqual(labelsFor('fix: x', 'This is not a BREAKING CHANGE: really'), ['bug']);
  assert.deepEqual(labelsFor('fix: x', 'breaking change: lowercase'), ['bug']);
  assert.deepEqual(labelsFor('fix: x', null), ['bug']);
  // A breaking body on a non-conventional title still flags it.
  assert.deepEqual(labelsFor('Rework everything', 'BREAKING CHANGE: yes'), ['breaking-change']);
});

test('title edit swaps the label added for the old title', () => {
  assert.deepEqual(
    planLabels({ action: 'edited', title: 'fix: x', oldTitle: 'feat: x', current: ['enhancement'] }),
    { add: ['bug'], remove: ['enhancement'] },
  );
  // Dropping the `!` removes breaking-change.
  assert.deepEqual(
    planLabels({ action: 'edited', title: 'feat: x', oldTitle: 'feat!: x', current: ['enhancement', 'breaking-change'] }),
    { add: [], remove: ['breaking-change'] },
  );
  // ...unless the body still declares it.
  assert.deepEqual(
    planLabels({
      action: 'edited', title: 'feat: x', oldTitle: 'feat!: x', body: 'BREAKING CHANGE: y',
      current: ['enhancement', 'breaking-change'],
    }),
    { add: [], remove: [] },
  );
  // Same label before and after: nothing to do.
  assert.deepEqual(
    planLabels({ action: 'edited', title: 'feat: y', oldTitle: 'feat: x', current: ['enhancement'] }),
    { add: [], remove: [] },
  );
  // New title doesn't match: the old label is removed and the check fails.
  assert.deepEqual(
    planLabels({ action: 'edited', title: 'whatever', oldTitle: 'fix: x', current: ['bug'] }),
    { add: [], remove: ['bug'] },
  );
  // Body-only edit (no changes.title): nothing is removed.
  assert.deepEqual(
    planLabels({ action: 'edited', title: 'fix: x', oldTitle: undefined, current: ['enhancement'] }),
    { add: [], remove: [] },
  );
  // oldTitle is ignored on other events.
  assert.deepEqual(
    planLabels({ action: 'synchronize', title: 'fix: x', oldTitle: 'feat: x', current: ['enhancement'] }),
    { add: [], remove: [] },
  );
});

test('a label set by a maintainer is kept', () => {
  // Maintainer chose a different changelog label: leave it.
  assert.deepEqual(
    planLabels({ action: 'synchronize', title: 'feat: x', current: ['internal'] }),
    { add: [], remove: [] },
  );
  assert.deepEqual(
    planLabels({ action: 'reopened', title: 'feat: x', current: ['new feature', 'needs-review'] }),
    { add: [], remove: [] },
  );
  // ...but breaking-change is still added when it applies.
  assert.deepEqual(
    planLabels({ action: 'opened', title: 'feat!: x', current: ['new feature'] }),
    { add: ['breaking-change'], remove: [] },
  );
  // A title edit that doesn't touch the maintainer's label.
  assert.deepEqual(
    planLabels({ action: 'edited', title: 'fix: stuff', oldTitle: 'stuff', current: ['internal'] }),
    { add: [], remove: [] },
  );
  // breaking-change alone is not a type label: the type label is still added.
  assert.deepEqual(
    planLabels({ action: 'opened', title: 'fix: x', current: ['breaking-change'] }),
    { add: ['bug'], remove: [] },
  );
  // Non-changelog labels (e.g. Dependabot's `github_actions`) are ignored.
  assert.deepEqual(
    planLabels({ action: 'opened', title: 'chore(deps): bump x', current: ['github_actions'] }),
    { add: ['dependencies'], remove: [] },
  );
  // Dependabot already labeled it.
  assert.deepEqual(
    planLabels({ action: 'opened', title: 'chore(deps): bump x', current: ['dependencies', 'github_actions'] }),
    { add: [], remove: [] },
  );
});

test('changelog labels agree with the check and with release.yml', () => {
  // Labels required by the check step in the same workflow.
  const input = workflow.match(/^ +labels: >-\n((?: {12}.*\n)+)/m);
  assert.ok(input, 'labels input of the check step not found');
  const checked = input[1].split(',').map((l) => l.trim()).filter(Boolean);
  assert.deepEqual([...checked].sort(), [...CHANGELOG_LABELS].sort());

  // Labels used to build the release notes (ignoring the "*" catch-all).
  const release = fs.readFileSync(path.join(GITHUB_DIR, 'release.yml'), 'utf8');
  const released = new Set([...release.matchAll(/^ +- ([a-z][a-z -]*)$/gm)].map((m) => m[1].trim()));
  assert.deepEqual([...released].sort(), [...CHANGELOG_LABELS].sort());

  // Every label the mapping can produce is a changelog label.
  for (const title of ['feat: x', 'fix: x', 'docs: x', 'deps: x', 'ci: x', 'feat!: x']) {
    for (const label of labelsFor(title)) assert.ok(CHANGELOG_LABELS.includes(label), label);
  }
});
