"""Apply the authored continuity repair through Studio's native patch engine."""
import json
from pathlib import Path
from pixel_pen import call
from regional_patch import value,prepare,validate,apply
from connected_canvas import capture_canvas
import atelier22_main,atelier22_structure,atelier22_eqcats,atelier22_controls
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'target/catamp22';OUT.mkdir(exist_ok=True)
VIEW={'panel':'canvas','layers':[],'paint_layer':None,'presentation':True,'guides':False,'zoom':2,'preview_playlist_height':145,'active':True,'pressed':False,'volume':20,'balance':14,'position':12,'scroll':8,'eq':[14,27,22,17,11,6,0,5,12,20,27]}

def run():
    s=value(call('studio_status'))
    if s['dirty']:raise RuntimeError('Save current edits before applying refinement')
    expected=ROOT/'assets/skins/checkpoints/Silverplay 21/Catamp Silverplay.wsz'
    call('studio_open',{'path':str(expected)})
    s=value(call('studio_status'))
    (OUT/'before-status.json').write_text(json.dumps(s,indent=2))
    call('studio_project',{'action':'save','path':str(OUT/'before.cstudio')})
    groups=[('22 · continuous crystal case',atelier22_structure.build()),
            ('22 · sculpted feline navigator',atelier22_main.build()),
            ('22 · readable silver tabbies',atelier22_eqcats.build())]
    groups += [('22 · crystal transport and switches',atelier22_controls.main_controls()),
               ('22 · fish sliders and tracks',atelier22_controls.rails()),
               ('22 · EQ paws and grooves',atelier22_controls.eq_controls()),
               ('22 · scroll jewel',atelier22_controls.scrollbar())]
    for name,parts in groups:
        patch=prepare(name,parts);r=validate(patch)
        (OUT/(name.replace(' · ','-')+'.json')).write_text(json.dumps(patch))
        print(name,r['pixels_written'],flush=True);apply(patch)
    call('studio_set_palette',{'Normal':'#b9d8dc','Current':'#f1f5e6','NormalBG':'#21465c','SelectedBG':'#32627b','MbFG':'#102b40','MbBG':'#afced2'})
    call('studio_state',VIEW)
    call('studio_render',{'zoom':2,'path':str(OUT/'assembled-editor.png')})
    capture_canvas(OUT/'assembled-gpu.png')
    print('Applied and captured. Final export awaits visual inspection.',flush=True)

if __name__=='__main__':run()
