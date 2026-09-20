#!/usr/bin/env python3
"""Evidence gate negatives: a zero-test pass and a compiler error are not proofs."""
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('verus_pilot', Path(__file__).with_name('verus-pilot.py'))
pilot = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pilot)


def result(success=True):
    return {
        'verus': {'version':'pinned','commit':'exact'},
        'verification-results': {'is-verifying-entire-crate':True,'encountered-vir-error':False,'encountered-error':not success,'success':success,'verified':1 if success else 0,'errors':0 if success else 1},
        'times-ms': {'smt': {'smt-run-module-times':[{'function-breakdown':[{'function':'rullst_age_policy_pilot::AgePolicy::permits','success':success}]}]}},
    }


class EvidenceTests(unittest.TestCase):
    config = {'release':'pinned','commit':'exact'}

    def test_requires_exact_production_and_failing_control(self):
        pilot.check_result(result(),True,self.config)
        pilot.check_result(result(False),False,self.config)
        for expected in (True,False):
            with self.assertRaises(ValueError):
                pilot.check_result(result(not expected),expected,self.config)

    def test_zero_or_partial_verification_cannot_pass(self):
        for field, value in [('verified',0),('errors',1),('is-verifying-entire-crate',False),('encountered-vir-error',True),('encountered-error',True)]:
            candidate=result()
            candidate['verification-results'][field]=value
            with self.assertRaises(ValueError): pilot.check_result(candidate,True,self.config)

    def test_controls_must_fail_the_named_function_not_compilation(self):
        candidate=result(False)
        candidate['verification-results']['encountered-vir-error']=True
        with self.assertRaises(ValueError): pilot.check_result(candidate,False,self.config)
        for name in ('','other::permits'):
            candidate=result(False)
            candidate['times-ms']['smt']['smt-run-module-times'][0]['function-breakdown'][0]['function']=name
            with self.assertRaises(ValueError): pilot.check_result(candidate,False,self.config)

    def test_tool_changes_and_missing_results_fail(self):
        for field in ('version','commit'):
            candidate=result()
            candidate['verus'][field]='different'
            with self.assertRaises(ValueError): pilot.check_result(candidate,True,self.config)
        candidate=copy.deepcopy(result())
        candidate['times-ms']['smt']['smt-run-module-times']=[]
        with self.assertRaises(ValueError): pilot.check_result(candidate,True,self.config)


if __name__ == '__main__':
    unittest.main()
