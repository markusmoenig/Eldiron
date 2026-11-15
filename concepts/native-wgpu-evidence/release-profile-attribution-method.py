import collections, datetime, gzip, json, pathlib, statistics, subprocess
import xml.etree.ElementTree as ET

BASE = pathlib.Path('/var/folders/h0/565dc64n6d39c2lrzq1lf6zm0000gn/T')
METAL = BASE / 'eldiron-release-metal-full-1bo8v6ih'
CPU = BASE / 'eldiron-release-cpu-full-kpbugc2m'

class Table:
    def __init__(self, path):
        self.root = ET.parse(path).getroot()
        self.ids = {x.attrib['id']: x for x in self.root.iter() if 'id' in x.attrib}
        self.columns = [x.findtext('mnemonic') for x in self.root.findall('.//schema/col')]
    def resolve(self, element):
        while 'ref' in element.attrib:
            element = self.ids[element.attrib['ref']]
        return element
    def number(self, element):
        return int(self.resolve(element).text or '0')
    def owner(self, element):
        element = self.resolve(element)
        pid = element.find('pid')
        return self.number(pid) if pid is not None else None
    def rows(self):
        for row in self.root.iter('row'):
            yield {k: self.resolve(v) for k, v in zip(self.columns, row)}

def info(root):
    toc = ET.parse(root / 'toc.xml').getroot()
    start = datetime.datetime.fromisoformat(toc.findtext('.//start-date'))
    epoch = int(start.timestamp() * 1e9)
    pid = int(toc.find('.//target/process').attrib['pid'])
    benchmark = json.loads((root / 'benchmark.json').read_text())
    phases = {k: [x - epoch for x in v['trace_wallclock_ns']] for k, v in benchmark['modes'].items()}
    return pid, phases

def phase_at(time, phases, guard=2000000):
    return next((k for k, (a, b) in phases.items() if a + guard <= time <= b - guard), None)

def union(intervals):
    end = -1
    total = 0
    for a, b in sorted(intervals):
        if b > end:
            total += b - max(a, end)
            end = b
    return total

def distribution(values):
    values = sorted(values)
    def percentile(q):
        p = (len(values) - 1) * q
        i = int(p)
        return values[i] + (values[min(i + 1, len(values) - 1)] - values[i]) * (p - i)
    return {'count': len(values), 'mean_ms': statistics.mean(values) / 1e6,
            'p50_ms': percentile(.5) / 1e6, 'p95_ms': percentile(.95) / 1e6,
            'p99_ms': percentile(.99) / 1e6}

def cpu_report(root):
    pid, phases = info(root)
    table = Table(root / 'time-profile.xml')
    raw_names = sorted({x.attrib['name'] for x in table.root.iter('frame') if 'name' in x.attrib})
    result = subprocess.run(['xcrun', 'llvm-cxxfilt'], input='\n'.join(raw_names)+'\n',
                            text=True, capture_output=True, check=True)
    names = dict(zip(raw_names, result.stdout.splitlines()))
    assert len(names) == len(raw_names)
    reports = {k: {'samples': 0, 'sample_weight_ns': 0, 'frame_samples': 0,
                    'frame_weight_ns': 0, 'leaf': collections.Counter(),
                    'inclusive': collections.Counter()} for k in phases}
    times = []
    stacks, stack_ids, normalized = [], {}, []
    for row in table.rows():
        if table.owner(row['process']) != pid:
            continue
        time = table.number(row['time'])
        times.append(time)
        mode = phase_at(time, phases)
        if mode is None:
            continue
        frames = [names.get(table.resolve(x).attrib.get('name', ''), '') for x in row['stack']]
        weight = table.number(row['weight'])
        key = tuple(frames)
        if key not in stack_ids:
            stack_ids[key] = len(stacks)
            stacks.append(frames)
        normalized.append([mode, time, weight, stack_ids[key]])
        report = reports[mode]
        report['samples'] += 1
        report['sample_weight_ns'] += weight
        if not any('release_benchmark' in x and 'Application' in x and '::frame' in x for x in frames):
            continue
        report['frame_samples'] += 1
        report['frame_weight_ns'] += weight
        if frames:
            report['leaf'][frames[0]] += weight
        for name in set(frames):
            report['inclusive'][name] += weight
    for report in reports.values():
        for key in ('leaf', 'inclusive'):
            report[key] = [{'function': k, 'sample_weight_ns': v,
                             'frame_sample_share': v / report['frame_weight_ns']}
                            for k, v in report[key].most_common()
                            if key == 'leaf' or any(c in k for c in
                                ('rusterix', 'eldiron_presentation', 'renderkit', 'wgpu_', 'theframework'))][:35]
    (root / 'cpu-phase-samples.json.gz').write_bytes(gzip.compress(json.dumps({'pid':pid, 'phase_ranges_ns':phases, 'columns':['mode','trace_time_ns','statistical_sample_weight_ns','stack_id'], 'stacks':stacks, 'samples':normalized},separators=(',',':')).encode(),mtime=0))
    return {'pid': pid, 'sample_range_ns': [min(times), max(times)],
            'phase_ranges_ns': phases, 'modes': reports}

def gpu_report(root):
    pid, phases = info(root)
    encoding = Table(root / 'metal-application-encoders-list.xml')
    encoders = {}
    for row in encoding.rows():
        if encoding.owner(row['process']) != pid:
            continue
        buffer_label = row['cmdbuffer-label'].text or 'unlabelled buffer'
        encoder_label = row['encoder-label'].text or 'unlabelled encoder'
        label = buffer_label + ' / ' + encoder_label
        encoders[encoding.number(row['encoder-id'])] = label
    table = Table(root / 'metal-gpu-intervals.xml')
    intervals = {k: [] for k in phases}
    grouped = {k: collections.defaultdict(list) for k in phases}
    events = collections.Counter()
    unmatched = collections.Counter()
    normalized = []
    for row in table.rows():
        if table.owner(row['process']) != pid or table.number(row['event-depth']) != 0:
            continue
        if row['state'].attrib.get('fmt') != 'Active':
            continue
        start = table.number(row['start'])
        end = start + table.number(row['duration'])
        mode = phase_at((start + end) // 2, phases)
        if mode is None:
            continue
        if start < phases[mode][0] + 2000000 or end > phases[mode][1] - 2000000:
            continue
        encoder = table.number(row['encoder-id'])
        label = encoders.get(encoder, 'unmatched encoder')
        if encoder not in encoders:
            unmatched[mode] += 1
        normalized.append([mode, start, end-start, encoder, label, row['channel-name'].attrib.get('fmt')])
        intervals[mode].append((start, end))
        grouped[mode][(label, encoder)].append((start, end))
        events[mode] += 1
    reports = {}
    for mode, groups in grouped.items():
        stages = collections.defaultdict(list)
        for (label, encoder), spans in groups.items():
            stages[label].append(union(spans))
        busy = union(intervals[mode])
        reports[mode] = {'depth_zero_active_events': events[mode], 'unmatched_events': unmatched[mode],
                         'owned_gpu_busy_union_ms': busy / 1e6,
                         'phase_wall_ms': (phases[mode][1] - phases[mode][0]) / 1e6,
                         'stages': {k: distribution(v) for k, v in stages.items()}}
    (root / 'gpu-phase-events.json.gz').write_bytes(gzip.compress(json.dumps({'pid':pid, 'phase_ranges_ns':phases, 'columns':['mode','trace_start_ns','duration_ns','encoder_id','label','channel'], 'events':normalized},separators=(',',':')).encode(),mtime=0))
    return {'pid': pid, 'phase_ranges_ns': phases, 'modes': reports}

result = {'scope': 'Instrumented release attribution, not normal frame-time or physical input/present latency',
          'source_commit': '53af6e17', 'test_harness_delta': 'optional per-mode wall-clock markers',
          'clock_alignment_guard_ns': 2000000,
          'cpu_weight_note': 'Statistical 1ms sample weights, not measured function durations; inclusive shares overlap',
          'gpu_label_note': 'wgpu-hal29 Metal relabels the final command buffer to (wgpu internal) Signal when submitting. Generic compute labels do not distinguish original kernels and are not solely fence overhead.',
          'gpu_overlap_note': 'Only target-PID depth-zero Active intervals; union Vertex/Fragment/Compute per encoder and globally; stage distributions are encoder durations, not frame times',
          'cpu_only': cpu_report(CPU), 'metal_cpu': cpu_report(METAL), 'gpu': gpu_report(METAL)}
(METAL / 'attribution.json').write_text(json.dumps(result, indent=2)+'\n')
print('evidence', METAL / 'attribution.json')
for mode in ('idle', 'hover', 'pan', 'zoom'):
    cpu = result['cpu_only']['modes'][mode]
    gpu = result['gpu']['modes'][mode]
    print(mode, 'CPU frame samples', cpu['frame_samples'], 'GPU busy ms', round(gpu['owned_gpu_busy_union_ms'], 2),
          'phase ms', round(gpu['phase_wall_ms'], 2), 'unmatched', gpu['unmatched_events'])
    print('CPU leaf', [(x['function'][:180], round(x['frame_sample_share']*100, 2)) for x in cpu['leaf'][:8]])
    print('GPU stages', [(k, v['count'], round(v['p50_ms'], 4)) for k, v in sorted(gpu['stages'].items(), key=lambda x:x[1]['mean_ms']*x[1]['count'], reverse=True)[:8]])
