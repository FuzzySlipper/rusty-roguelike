import nx from '@nx/eslint-plugin';

const depConstraints = [
  {
    sourceTag: 'type:app',
    onlyDependOnLibsWithTags: ['type:feature', 'type:lib'],
  },
  { sourceTag: 'type:feature', onlyDependOnLibsWithTags: ['type:lib'] },
  { sourceTag: 'type:lib', onlyDependOnLibsWithTags: ['type:lib'] },
  {
    sourceTag: 'type:testing',
    onlyDependOnLibsWithTags: [
      'type:app',
      'type:feature',
      'type:lib',
      'type:testing',
    ],
  },
  { sourceTag: 'scope:protocol', onlyDependOnLibsWithTags: [] },
  { sourceTag: 'scope:platform', onlyDependOnLibsWithTags: [] },
  {
    sourceTag: 'scope:transport',
    onlyDependOnLibsWithTags: ['scope:protocol', 'scope:platform'],
  },
  {
    sourceTag: 'scope:store',
    onlyDependOnLibsWithTags: ['scope:protocol', 'scope:transport'],
  },
  {
    sourceTag: 'scope:renderer',
    onlyDependOnLibsWithTags: ['scope:platform', 'scope:protocol'],
  },
  {
    sourceTag: 'scope:feature',
    onlyDependOnLibsWithTags: [
      'scope:protocol',
      'scope:platform',
      'scope:store',
      'scope:renderer',
      'scope:theme',
    ],
  },
  { sourceTag: 'scope:theme', onlyDependOnLibsWithTags: [] },
];

export default [
  ...nx.configs['flat/base'],
  ...nx.configs['flat/typescript'],
  ...nx.configs['flat/javascript'],
  { ignores: ['dist/**', 'coverage/**', 'node_modules/**', 'tmp/**'] },
  {
    files: ['**/*.ts', '**/*.js', '**/*.mts', '**/*.mjs'],
    rules: {
      '@nx/enforce-module-boundaries': [
        'error',
        {
          allow: ['^.*/eslint(\\.base)?\\.config\\.[cm]?[jt]s$'],
          depConstraints,
          enforceBuildableLibDependency: false,
        },
      ],
      '@typescript-eslint/consistent-type-imports': 'error',
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-non-null-assertion': 'error',
    },
  },
];
