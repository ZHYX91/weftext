"""Finite algorithm oracle; not Weftext, filesystem or authorization acceptance."""
from dataclasses import dataclass
from threading import RLock
import json

@dataclass(frozen=True)
class Cut:
    generation: int
    revisions: tuple
    auth_generation: int

class JournalModel:
    def __init__(self):
        self.lock = RLock()
        self.generation = 0
        self.auth_generation = 1
        self.current = {'A':'A0', 'B':'B0'}
        self.blobs = {'A0':10, 'B0':0}
        self.roots = {}

    def commit_both(self):
        with self.lock:
            self.blobs.update({'A1':0, 'B1':10})
            self.current = {'A':'A1','B':'B1'}
            self.generation += 1

    def open_cut(self):
        with self.lock:
            cut = Cut(self.generation, tuple(sorted(self.current.items())), self.auth_generation)
            self.roots[id(cut)] = frozenset(ref for _, ref in cut.revisions)
            return cut

    def read_at(self, cut, entity):
        return self.blobs[dict(cut.revisions)[entity]]

    def gc(self):
        with self.lock:
            retained = set(self.current.values()).union(*self.roots.values())
            self.blobs = {r:v for r,v in self.blobs.items() if r in retained}

    def publish_run(self, cut):
        # Full evaluation, then freeze order and payload together. Ref tie-breaker.
        data = tuple(sorted((name, self.read_at(cut,name)) for name,_ in cut.revisions))
        return {'cut':cut, 'data':data, 'auth_generation':cut.auth_generation, 'coverage':'complete'}

    def page(self, run, ordinal, size):
        if run['auth_generation'] != self.auth_generation:
            return {'state':'invalidated', 'rows':None}
        data = run['data']
        return {'state':'ok', 'rows':data[ordinal:ordinal+size],
                'next':min(ordinal+size,len(data)), 'terminal':ordinal+size>=len(data)}

def main():
    naive = JournalModel()
    a = naive.blobs[naive.current['A']]
    naive.commit_both()
    b = naive.blobs[naive.current['B']]
    report = {'naive_individual_reads':{'A':a,'B':b,'sum':a+b}}
    assert a+b == 20

    j = JournalModel()
    c = j.open_cut()
    a = j.read_at(c,'A')
    j.commit_both()
    j.gc()
    b = j.read_at(c,'B')
    report['one_cut_reads_across_commit'] = {'generation':c.generation,'A':a,'B':b,'sum':a+b,
                                           'old_blobs_retained':{'A0','B0'}.issubset(j.blobs)}
    assert a+b == 10
    assert report['one_cut_reads_across_commit']['old_blobs_retained']
    newer = j.open_cut()
    report['new_cut_sum'] = j.read_at(newer,'A')+j.read_at(newer,'B')
    assert report['new_cut_sum'] == 10
    run = j.publish_run(c)
    p0 = j.page(run,0,1)
    j.current = {'Z':'B1','A':'A1'}  # model later data/order change
    p1 = j.page(run,1,1)
    report['frozen_pages'] = {'first':p0,'second_after_live_reorder':p1}
    assert p0['rows'] == (('A',10),) and p1['rows'] == (('B',0),)
    j.auth_generation += 1
    revoked = j.page(run,0,1)
    report['revoked_page'] = revoked
    assert revoked['rows'] is None and revoked['state'] == 'invalidated'
    report['assertions_passed'] = True
    report['evidence_limit'] = 'Small in-memory model only; no physical installation, external cut, security, persistence or crash proof.'
    print(json.dumps(report,indent=2))

if __name__ == '__main__': main()
