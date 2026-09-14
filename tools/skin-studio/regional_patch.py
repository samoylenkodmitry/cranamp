"""Prepare/validate/apply isolated native atlas drawings through Studio MCP.
Artists can build geometry independently; only the coordinator submits patches.
"""
import argparse
import importlib.util
import json
from pathlib import Path
from pixel_pen import call


def value(result):
    return json.loads(next(c['text'] for c in result['content'] if c['type']=='text'))


def prepare(name, parts, baseline=None):
    """Bind geometry to current native source pixels without changing GUI state."""
    requests=[{'sheet':p['sheet'],'rect':p['rect']} for p in parts]
    if baseline is None:
        baseline=value(call('studio_patch',{'action':'inspect','parts':requests}))['parts']
    if requests != [{'sheet':p['sheet'],'rect':p['rect']} for p in baseline]:
        raise ValueError('Baseline regions do not match the authored patch')
    return {'name':name,'parts':[dict(p,expected=b['expected']) for p,b in zip(parts,baseline)]}


def prepare_revision(name, parts, baseline):
    """Replace the complete owned plane using a previously captured review baseline."""
    if not baseline.get('replace_layer') or not baseline.get('layer_expected'):
        raise ValueError('Capture a replacement baseline with the live painting layer ID first')
    patch=prepare(name,parts,baseline['parts'])
    patch.update(replace_layer=baseline['replace_layer'],layer_expected=baseline['layer_expected'])
    return patch


def validate(patch):
    return value(call('studio_patch',dict(patch,action='validate')))


def apply(patch):
    return value(call('studio_patch',dict(patch,action='apply')))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    sub=parser.add_subparsers(dest='action',required=True)
    p=sub.add_parser('prepare');p.add_argument('module',type=Path);p.add_argument('output',type=Path);p.add_argument('--name',required=True);p.add_argument('--baseline',type=Path)
    p=sub.add_parser('inspect');p.add_argument('regions',type=Path);p.add_argument('output',type=Path);p.add_argument('--replace-layer')
    for action in ['validate','apply']:
        p=sub.add_parser(action);p.add_argument('patch',type=Path)
    args=parser.parse_args()
    if args.action=='prepare':
        spec=importlib.util.spec_from_file_location('artist_patch',args.module)
        module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        baseline=json.loads(args.baseline.read_text()) if args.baseline else None
        patch=(prepare_revision(args.name,module.build(),baseline) if baseline and baseline.get('replace_layer')
               else prepare(args.name,module.build(),baseline['parts'] if baseline else None))
        validate(patch)
        args.output.write_text(json.dumps(patch,indent=2)+'\n')
        print(f'Validated patch saved to {args.output}; artwork unchanged')
    elif args.action=='inspect':
        regions=json.loads(args.regions.read_text())
        request={'action':'inspect','parts':regions}
        if args.replace_layer:request['replace_layer']=args.replace_layer
        baseline=value(call('studio_patch',request))
        args.output.write_text(json.dumps(baseline,indent=2)+'\n')
        print(f'Saved source baseline to {args.output}; artwork unchanged')
    else:
        patch=json.loads(args.patch.read_text())
        print(json.dumps(validate(patch) if args.action=='validate' else apply(patch)))


if __name__=='__main__':main()
