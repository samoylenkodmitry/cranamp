"""Build local, linked cheatsheets from native CUA review captures."""
import argparse
import html
import json
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('root', type=Path)
args = parser.parse_args()
root = args.root.resolve()
cases = json.loads((root / 'cases.json').read_text())
style = '''body{margin:0;background:#101218;color:#edf0f6;font:14px system-ui}header{padding:20px;position:sticky;top:0;background:#181c26;z-index:2}a{color:#80d5ee}h1{font-size:22px;margin:0 0 8px}p{margin:6px 0}nav{display:flex;gap:12px;flex-wrap:wrap}main{display:grid;grid-template-columns:repeat(10,minmax(0,1fr));gap:10px;padding:12px}figure{margin:0;background:#1c202a;border:1px solid #343a49;border-radius:6px;overflow:hidden}figure img{display:block;width:100%;height:auto;max-height:250px;object-fit:contain;background:#11141c}figcaption{padding:8px;font-size:11px;min-height:52px;overflow-wrap:anywhere} .detail{height:120px;overflow:hidden;border-top:1px solid #455062}.detail svg{width:100%;height:100%}input,select{font:inherit;background:#242b3a;color:inherit;border:1px solid #455062;padding:6px} .missing{min-height:200px;padding:10px;color:#ffc28a}@media print{header{position:static}figure{break-inside:avoid}}'''
dirs = sorted((d for d in root.iterdir() if d.is_dir() and 'x' in d.name and d.name.split('x')[0].isdigit()), key=lambda d: tuple(map(int,d.name.split('x'))))
links=[]
manifest=[]
for directory in dirs:
    width,height=map(int,directory.name.split('x'))
    captures=[]
    for i,name in enumerate(cases,1):
        filename=f'{i:03}-{name}.jpg'
        if (directory/filename).exists(): captures.append((i,name,filename))
    links.append(f'<a href="{directory.name}/index.html">{directory.name}: {len(captures)}/{len(cases)}</a>')
    manifest.append({'width':width,'height':height,'captured':len(captures),'expected':len(cases)})
    for page in range((len(cases)+19)//20):
        subset=[c for c in captures if page*20<c[0]<=(page+1)*20]
        cards=[]
        for i,name,file in subset:
            detail=''
            if width>=900:
                x=width-(304 if width>=1280 else 272)
                # Native title is 32 logical pixels. The crop keeps actual pixels.
                detail=f'<div class="detail"><svg viewBox="{x*2} 176 {(width-x)*2} {min(height-56,720)*2}"><defs><clipPath id="crop-{i}"><rect x="{x*2}" y="176" width="{(width-x)*2}" height="{min(height-56,720)*2}"/></clipPath></defs><image clip-path="url(#crop-{i})" href="{file}" width="{width*2}" height="{(height+72)*2}"/></svg></div>'
            cards.append(f'<figure><figcaption>{i}. {html.escape(name).replace("-", " ")}</figcaption><a href="{file}"><img src="{file}"></a>{detail}</figure>')
        nav=' '.join(f'<a href="page-{p+1}.html">{p*20+1}-{min((p+1)*20,len(cases))}</a>' for p in range((len(cases)+19)//20))
        text=f'<!doctype html><meta charset="utf-8"><title>Studio {directory.name} review {page+1}</title><style>{style}</style><header><h1>Studio {directory.name} — cases {page*20+1}–{min((page+1)*20,len(cases))}</h1><nav><a href="../index.html">All widths</a>{nav}</nav></header><main>{"".join(cards)}</main>'
        (directory/f'page-{page+1}.html').write_text(text)
        if page==0: (directory/'index.html').write_text(text)
(root/'index.html').write_text(f'<!doctype html><meta charset="utf-8"><title>Skin Studio visual review</title><style>{style}main{{display:block;padding:24px}}nav a{{padding:10px;background:#242b3a;border-radius:5px}}</style><header><h1>Skin Studio visual review</h1><p>Native GPU captures. {len(cases)} distinct control states per size. Select a width, then a range. Click any screen to inspect its original pixels.</p><p>Each card includes the real window and review footer. Wide layouts include an enlarged inspector. Screens cover finite control branches and boundary values; arbitrary numeric inputs are not enumerated.</p></header><main><nav>{"".join(links)}</nav></main>')
(root/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(f'{sum(m["captured"] for m in manifest)} captures across {len(dirs)} sizes')
