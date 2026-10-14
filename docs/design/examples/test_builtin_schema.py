"""Behavioral checks for configuration authority and preserved business values."""
import copy
import unittest
from resolve_builtin_schema import ROOT, load, resolve


class BuiltinResolution(unittest.TestCase):
    def setUp(self):
        self.author = load(ROOT/'schema.authoring.json')
        self.registry = load(ROOT/'builtin-abi.design.json')

    def test_complete_business_model_preserved(self):
        original = copy.deepcopy(self.author)
        expanded = resolve(self.author, self.registry)
        self.assertEqual(expanded, load(ROOT/'schema.json'))
        self.assertEqual(self.author, original)
        expanded['providers'][0]['exports'][0]['result'] = 'forged'
        self.assertNotEqual(expanded, resolve(self.author, self.registry))

    def test_author_cannot_override_signatures(self):
        self.author['providers'][0]['exports'] = []
        with self.assertRaises(ValueError): resolve(self.author, self.registry)

    def test_author_cannot_override_validator_type(self):
        self.author['validators'][0]['inputs'] = {'value': 'text'}
        with self.assertRaises(ValueError): resolve(self.author, self.registry)

    def test_unknown_operation_and_wrong_abi_reject(self):
        for mutate in [lambda a: a['validators'][0].update(operation='run-script'),
                       lambda a: a['providers'][0]['implementation'].update(abi=99),
                       lambda a: a['providers'][0]['implementation'].update(id='unknown'),
                       lambda a: a['providers'].append(copy.deepcopy(a['providers'][0]))]:
            with self.subTest(mutate=mutate):
                a = copy.deepcopy(self.author)
                mutate(a)
                with self.assertRaises(ValueError): resolve(a, self.registry)


if __name__ == '__main__': unittest.main()
