"""Command-line test selection for the hosted HTTP suites."""

import argparse
from pathlib import Path
import unittest


def run_http_suite(case):
    parser = argparse.ArgumentParser(description=f'Run {case.__name__} HTTP tests')
    parser.add_argument('executable', type=Path, help='Weftext CLI executable')
    parser.add_argument('tests', nargs='*', metavar='TEST', help='Exact test method defined in this suite')
    args = parser.parse_args()
    if not args.executable.is_file():
        parser.error(f'Executable does not exist: {args.executable}')

    # Subclasses share product fixtures. Each check belongs to its declaring suite.
    owned = {name for name, method in vars(case).items() if name.startswith('test_') and callable(method)}
    if not owned:
        parser.error(f'{case.__name__} defines no tests')
    selected = set(args.tests) if args.tests else owned
    unknown = selected - owned
    if unknown:
        parser.error('Unknown test selection: ' + ', '.join(sorted(unknown)))

    suite = unittest.TestSuite(case(name) for name in sorted(selected))
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    return 0 if result.wasSuccessful() else 1
