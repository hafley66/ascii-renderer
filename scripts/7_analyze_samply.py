"""Summarize every thread in a locally symbolicated samply recording.
Usage: python3 scripts/7_analyze_samply.py DIRECTORY
Expects profile.json.gz, symbol-pairs.json and symbol-response.json.
"""
import collections
import gzip
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
profile = json.load(gzip.open(root / 'profile.json.gz'))
pairs = json.loads((root / 'symbol-pairs.json').read_text())
response = json.loads((root / 'symbol-response.json').read_text())
symbols = {tuple(pair): result[0].get('function', str(pair))
           for pair, result in zip(pairs, response['results'][0]['stacks'])}
summaries = []
for thread in profile['threads']:
    functions = []
    for index in range(thread['funcTable']['length']):
        name = thread['stringArray'][thread['funcTable']['name'][index]]
        resource = thread['funcTable']['resource'][index]
        if resource >= 0 and name.startswith('0x'):
            name = symbols.get((thread['resourceTable']['lib'][resource], int(name, 16)), name)
        functions.append(name)
    stacks = []
    for index, frame in enumerate(thread['stackTable']['frame']):
        prefix = thread['stackTable']['prefix'][index]
        names = () if prefix is None else stacks[prefix]
        stacks.append(names + (functions[thread['frameTable']['func'][frame]],))
    inclusive = collections.Counter()
    leaf = collections.Counter()
    cpu_leaf = collections.Counter()
    paths = collections.Counter()
    samples = thread['samples']
    for index, stack in enumerate(samples['stack']):
        if stack is None:
            continue
        names = stacks[stack]
        weight = samples['weight'][index] if samples.get('weight') else 1
        cpu = (samples.get('threadCPUDelta') or [0] * samples['length'])[index] or 0
        for name in set(names):
            inclusive[name] += weight
        leaf[names[-1]] += weight
        cpu_leaf[names[-1]] += cpu
        paths[' -> '.join(names)] += weight
    summaries.append(dict(pid=thread['pid'], tid=thread['tid'], name=thread['name'],
                          main=thread['isMainThread'], sample_rows=samples['length'],
                          sample_weight=sum(leaf.values()),
                          inclusive=inclusive.most_common(), leaf=leaf.most_common(),
                          cpu_delta_at_sample_leaf_us=cpu_leaf.most_common(), paths=paths.most_common()))
(root / 'stacks-summary.json').write_text(json.dumps(summaries, indent=2) + '\n')
for row in summaries:
    if row['main']:
        print(row['pid'], row['name'], 'rows', row['sample_rows'], 'weight', row['sample_weight'])
        for name, count in row['leaf'][:8]:
            print(count, name)
