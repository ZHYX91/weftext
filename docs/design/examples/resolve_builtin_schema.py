"""Resolve compact author configuration using one trusted design registry.

This is a design fixture generator, not a runtime plugin loader or authorization.
The production equivalent obtains signatures from compiled Rust registrations.
"""
import argparse
import copy
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def exact(value, keys):
    if not isinstance(value, dict) or set(value) != set(keys):
        raise ValueError('Unknown or missing configuration members')


def load(path):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError('Duplicate JSON member: ' + key)
            result[key] = value
        return result
    return json.loads(Path(path).read_text(encoding='utf-8'), object_pairs_hook=unique)


def resolve(author, registry):
    exact(author, ['version', 'definitions', 'fields', 'types', 'aliases',
                   'relations', 'validators', 'providers', 'series_policies'])
    if author['version'] != 1:
        raise ValueError('Unsupported schema version')
    result = copy.deepcopy(author)
    providers = {}
    for provider in result['providers']:
        exact(provider, ['id', 'implementation', 'data', 'limits'])
        impl = provider['implementation']
        exact(impl, ['kind', 'id', 'abi'])
        if impl['kind'] != 'builtin' or impl['id'] not in registry['implementations']:
            raise ValueError('Unknown builtin implementation')
        trusted = registry['implementations'][impl['id']]
        if impl['abi'] != trusted['abi']:
            raise ValueError('Wrong builtin ABI')
        if provider['id'] in providers:
            raise ValueError('Duplicate provider')
        provider['family'] = trusted['family']
        provider['exports'] = copy.deepcopy(trusted['exports'])
        providers[provider['id']] = provider
    seen = set()
    for validator in result['validators']:
        exact(validator, ['id', 'provider', 'operation'])
        key = validator['id']
        if key in seen or key not in registry['validators']:
            raise ValueError('Unknown or duplicate validator')
        seen.add(key)
        trusted = registry['validators'][key]
        if any(validator[k] != trusted[k] for k in ['provider', 'operation']):
            raise ValueError('Validator binding differs from registry')
        provider = providers.get(validator['provider'])
        if not provider or validator['operation'] not in [v['name'] for v in provider['exports']]:
            raise ValueError('Missing provider operation')
        validator.update(copy.deepcopy(trusted))
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    expanded = resolve(load(ROOT/'schema.authoring.json'), load(ROOT/'builtin-abi.design.json'))
    if args.check:
        if expanded != load(ROOT/'schema.json'):
            raise SystemExit('Generated schema differs; run the resolver and review the delta')
        print('PASS: compact author schema resolves to the complete schema fixture')
    else:
        (ROOT/'schema.json').write_text(json.dumps(expanded, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
