"""Offline mapping contracts. Run: python3 tools/skin-studio/test_connected_canvas.py"""
import unittest
import re
import json
import base64
import struct
import zlib
from pathlib import Path
from unittest.mock import patch
from copy import deepcopy
from connected_canvas import CanvasPen,surface_map,surface_parts,runtime_rects,_intersection,control_cell,context_rect,study_canvas,presentation_bounds,capture_canvas,_screenshot_size


class ConnectedCanvasTests(unittest.TestCase):
    @staticmethod
    def screenshot_header(w,h):
        chunk=b'IHDR'+struct.pack('>IIBBBBB',w,h,8,6,0,0,0)
        header=b'\x89PNG\r\n\x1a\n'+struct.pack('>I',13)+chunk+struct.pack('>I',zlib.crc32(chunk))
        return {'content':[{'type':'image','mimeType':'image/png','data':base64.b64encode(header).decode()}]}

    def test_presentation_bounds_even_odd_and_zoom_cap(self):
        self.assertEqual(presentation_bounds([1160,850]),[305,48,550,754])
        self.assertEqual(presentation_bounds([1157,871]),[303,58,550,754])
        self.assertEqual(presentation_bounds([1159,873]),[304,59,550,754])
        self.assertEqual(presentation_bounds([550,754]),[0,0,550,754])
        self.assertEqual(presentation_bounds([1160,850],zoom=8),[305,48,550,754])
        self.assertEqual(presentation_bounds([1160,850],zoom=1),[442,236,275,377])
        self.assertEqual(presentation_bounds([1160,850],playlist_height=522),[442,48,275,754])
        for size in ([549,754],[550,753],[True,850],[1160.0,850]):
            with self.assertRaises(ValueError):presentation_bounds(size)

    def test_capture_measures_scene_and_requests_engine_crop_without_state_changes(self):
        status={'content':[{'type':'text','text':json.dumps({'view':{
            'presentation':True,'zoom':2,'preview_playlist_height':145}})}]}
        crop=self.screenshot_header(550,754)
        with patch('pixel_pen.call',side_effect=[status,self.screenshot_header(1157,871),crop]) as call:
            self.assertIs(capture_canvas('target/measured.png'),crop)
            self.assertEqual([c.args for c in call.call_args_list],[('studio_status',),
                ('studio_screenshot',{}),('studio_screenshot',{
                    'path':str(Path('target/measured.png').resolve()),'crop':[303,58,550,754]})])

    def test_capture_rejects_invalid_png_or_nonpresentation_without_mutations(self):
        for data in ('','!!!!',base64.b64encode(bytes(33)).decode()):
            with self.assertRaises(ValueError):
                _screenshot_size({'content':[{'type':'image','mimeType':'image/png','data':data}]})
        with self.assertRaises(ValueError):_screenshot_size(self.screenshot_header(0,871))
        status={'content':[{'type':'text','text':json.dumps({'view':{'presentation':False}})}]}
        with patch('pixel_pen.call',return_value=status) as call,self.assertRaisesRegex(ValueError,'view unchanged'):
            capture_canvas('target/measured.png')
        call.assert_called_once_with('studio_status')

    def test_bands_cover_every_pixel_once_with_exact_palette_and_boundaries(self):
        colors=['#476976','#87aeb8','#d9eeef']
        edges=[0,2,5,6]
        with patch('pixel_pen.call') as call:
            for axis,w,h in [('y',4,6),('x',6,4)]:
                p=CanvasPen()
                self.assertIs(p.bands(-2,3,w,h,colors,edges,axis=axis),p)
                painted={}
                for op in p.ops:
                    self.assertEqual(op['op'],'rect')
                    self.assertTrue(op['fill'])
                    self.assertNotIn('ramp',op)
                    for y in range(op['y'],op['y']+op['height']):
                        for x in range(op['x'],op['x']+op['width']):
                            self.assertNotIn((x,y),painted) # no overlap
                            painted[x,y]=op['color']
                expected={(x,y):colors[0 if offset<2 else 1 if offset<5 else 2]
                          for x in range(-2,-2+w) for y in range(3,3+h)
                          for offset in [y-3 if axis=='y' else x+2]}
                self.assertEqual(painted,expected) # no gaps, extra pixels or shades
            call.assert_not_called()

    def test_bands_group_recolor_and_control_mapping_preserve_palette_geometry(self):
        p=CanvasPen();colors=['#476976','#d9eeef'];edges=[0,8,10]
        with p.group(1,0) as local:
            local.bands(0,0,28,10,colors,edges)
        colors[0]='#000000';edges[1]=1
        copy=CanvasPen().place(p,colors={'#d9eeef':'#b6d8df'})
        original=deepcopy(p.ops)
        parts=copy.control_parts('seek',state='held',exclude=[[1,0,2,2]])
        for part in parts:
            self.assertTrue(part['clip_to_rect'])
            self.assertIsNone(_intersection(part['rect'],[279,0,2,2]))
            a,b=part['operations']
            self.assertEqual([a['x'],a['y'],a['width'],a['height']],[279,0,28,8])
            self.assertEqual([b['x'],b['y'],b['width'],b['height']],[279,8,28,2])
            self.assertEqual([a['color'],b['color']],['#476976','#b6d8df'])
        self.assertEqual(p.ops,original)
        self.assertEqual(local.ops[0]['x'],0)

    def test_bands_reject_invalid_input_atomically(self):
        p=CanvasPen();p.pixel(0,0,'#abcdef');original=deepcopy(p.ops)
        defaults=dict(x=0,y=0,w=4,h=6,colors=['#476976','#d9eeef'],edges=[0,5,6])
        cases=[{'edges':e} for e in ([1,5,6],[0,5,7],[0,0,6],[0,6,5],
                                     [0,5],[0,5.0,6],[0,True,6],None)]
        cases += [{'colors':c} for c in ([],None,'#ffffff',['#fff','#ffffff'],['#ffffff',3])]
        cases += [{'axis':'z'},{'w':0},{'h':-1},{'x':.5},{'y':True},{'h':float('nan')}]
        for case in cases:
            with self.subTest(case=case),self.assertRaises(ValueError):
                p.bands(**dict(defaults,**case))
            self.assertEqual(p.ops,original)

    def test_context_rect_keeps_exterior_halo_across_panel_join(self):
        rect=[5,103,12,10]
        self.assertEqual(context_rect(rect),[1,99,20,18])
        self.assertEqual(context_rect(rect,padding=0),rect)
        self.assertIsNot(context_rect(rect,padding=0),rect)
        self.assertEqual(rect,[5,103,12,10])

    def test_context_rect_clamps_all_canvas_edges_and_tall_playlist(self):
        for rect,expected in [([0,0,2,2],[0,0,6,6]),([273,0,2,2],[269,0,6,6]),
                              ([0,375,2,2],[0,371,6,6]),([273,375,2,2],[269,371,6,6])]:
            self.assertEqual(context_rect(rect),expected)
        self.assertEqual(context_rect([12,110,4,8],1000),[0,0,275,377])
        self.assertEqual(context_rect([270,750,5,4],playlist_height=522),[266,746,9,8])

    def test_study_invalid_arguments_never_call_mcp(self):
        with patch('pixel_pen.call') as call:
            cases=[{'padding':p} for p in (-1,1.5,True,None)]
            cases += [{'zoom':z} for z in (0,9,1.5,True)]
            cases += [{'playlist_height':144},{'path':''},{'path':None},{'path':Path.cwd()}]
            cases += [{'rect':r} for r in ([0,377,1,1],[-1,0,1,1],[274,0,2,1],[0,0,0,1])]
            for case in cases:
                with self.subTest(case=case),self.assertRaises(ValueError):
                    study_canvas(**dict({'rect':[0,0,1,1],'path':'target/study.png'},**case))
            call.assert_not_called()

    def test_study_raw_image_result_and_absolute_path_without_state_mutation(self):
        status={'content':[{'type':'text','text':json.dumps({'view':{'panel':'canvas','preview_playlist_height':145}})}]}
        result={'content':[{'type':'image','data':'native-image'},{'type':'text','text':'Study caption, not JSON'}]}
        with patch('pixel_pen.call',side_effect=[status,result]) as call:
            self.assertIs(study_canvas([199,132,4,20],'target/study.png'),result)
            self.assertEqual(call.call_args_list[0].args,('studio_status',))
            self.assertEqual(call.call_args_list[1].args,('studio_study',{
                'rect':[195,128,12,28],'path':str(Path('target/study.png').resolve()),'zoom':4,
                'selected':False,'values':False,'grid':False,'geometry':False}))
            self.assertEqual(call.call_count,2)

    def test_study_refuses_wrong_panel_or_height_without_changing_view(self):
        for view in ({'panel':'main','preview_playlist_height':145},
                     {'panel':'canvas','preview_playlist_height':261}):
            status={'content':[{'type':'text','text':json.dumps({'view':view})}]}
            with patch('pixel_pen.call',return_value=status) as call,self.assertRaisesRegex(ValueError,'view unchanged'):
                study_canvas([5,103,12,10],'target/study.png')
            call.assert_called_once_with('studio_status')

    def test_round_outline_matches_filled_round_geometry_without_fill(self):
        filled=CanvasPen(); outline=CanvasPen()
        filled.round(.5,110.25,274,266,8.5,'#cdeeff')
        outline.round_outline(.5,110.25,274,266,8.5,'#cdeeff',width=2)
        expected=deepcopy(filled.ops[0]);expected.update(fill=False,brush_size=2)
        self.assertEqual(outline.ops,[expected])
        self.assertTrue(any(isinstance(v,float) for cmd in expected['points'] for v in cmd))

    def test_round_outline_survives_mask_mapping_as_complete_stroke(self):
        p=CanvasPen();p.round_outline(0,110,275,267,8.5,'#cdeeff',width=2)
        original=deepcopy(p.ops)
        options={'exclude':[[4,114,2,5]]}
        parts=p.parts([0,110,12,18],**options)
        self.assertGreater(len(parts),3)
        for part in parts:
            self.assertTrue(part['clip_to_rect'])
            self.assertEqual(len(part['operations']),1)
            op=part['operations'][0]
            self.assertEqual(op['points'],original[0]['points'])
            self.assertFalse(op['fill'])
            self.assertEqual(op['brush_size'],2)
        for target in p.plan([0,110,12,18],**options).targets:
            self.assertIsNone(_intersection([*target['destination'],*target['rect'][2:]],[4,114,2,5]))
        self.assertEqual(p.ops,original)

    def test_round_outline_invalid_width_is_atomic(self):
        p=CanvasPen();p.pixel(0,0,'#abcdef');original=deepcopy(p.ops)
        for width in (0,33,1.5,True,float('nan')):
            with self.assertRaises(ValueError):p.round_outline(0,0,275,377,8,'#cdeeff',width)
            self.assertEqual(p.ops,original)

    def test_named_controls_match_player_sprite_definitions(self):
        source=(Path(__file__).resolve().parents[2]/'src/winamp/sprites.rs').read_text()
        constants={name:[int(float(n)) for n in coords.split(',') if n.strip()]
                   for name,coords in re.findall(r'pub const (\w+): SpriteRect = \(([\d.,\s]+)\);',source)}
        names={**{n:(f'{n.upper()}_BUTTON',f'{n.upper()}_BUTTON_ACTIVE')
                   for n in ('prev','play','pause','stop','next','eject')},
               'seek':('POSBAR_THUMB','POSBAR_THUMB_ACTIVE'),
               'volume':('VOLUME_THUMB','VOLUME_THUMB_ACTIVE'),
               'balance':('BALANCE_THUMB','BALANCE_THUMB_ACTIVE'),
               'eq':('EQ_SLIDER_THUMB','EQ_SLIDER_THUMB_SELECTED'),
               'scroll':('PLAYLIST_SCROLL_HANDLE','PLAYLIST_SCROLL_HANDLE_SELECTED'),
               **{n:tuple(f'{n.upper()}_{s}' for s in ('OFF','OFF_ACTIVE','ON','ON_ACTIVE'))
                  for n in ('shuffle','repeat')}}
        for name,sprites in names.items():
            states=('released','held') if len(sprites)==2 else ('off','off-held','on','on-held')
            for state,sprite in zip(states,sprites):
                with self.subTest(control=name,state=state):
                    self.assertEqual(control_cell(name,state=state)['rect'],constants[sprite])

    def test_track_endpoints_and_eq_row_boundary_are_native_cells(self):
        expected={
            'volume_track':[(0,[0,0,68,13]),(27,[0,405,68,13])],
            'balance_track':[(0,[9,0,38,13]),(27,[9,405,38,13])],
            'eq_track':[(0,[13,164,14,63]),(13,[208,164,14,63]),
                        (14,[13,229,14,63]),(27,[208,229,14,63])]}
        for name,cases in expected.items():
            for frame,rect in cases:
                self.assertEqual(control_cell(name,state='frame',frame=frame)['rect'],rect)
        # Every frame is distinct and EQ rows cannot overlap adjacent controls.
        cells=[control_cell('eq_track',state='frame',frame=i)['rect'] for i in range(28)]
        for i,a in enumerate(cells):
            for b in cells[i+1:]:self.assertIsNone(_intersection(a,b))

    def test_track_frame_rejection_and_explicit_state(self):
        for name in ('volume_track','balance_track','eq_track'):
            for bad in (-1,28,True,False,1.0,'1',None):
                with self.assertRaises(ValueError):control_cell(name,state='frame',frame=bad)
            with self.assertRaises(ValueError):control_cell(name,state='held',frame=4)
        with self.assertRaises(ValueError):control_cell('volume',state='released',frame=0)
        with self.assertRaises(ValueError):control_cell('eq_graph',state='normal',frame=1)

    def test_track_local_masks_preserve_source_geometry(self):
        p=CanvasPen();p.line([[-1,0],[14,62]],'#ffffff');before=deepcopy(p.ops)
        parts=p.control_parts('eq_track',state='frame',frame=27,
                              rect=[1,2,12,59],exclude=[[5,10,3,6]])
        self.assertGreater(len(parts),1)
        for q in parts:
            self.assertEqual(_intersection(q['rect'],[209,231,12,59]),q['rect'])
            self.assertIsNone(_intersection(q['rect'],[213,239,3,6]))
            self.assertEqual(q['operations'][0]['points'],[[-1,0],[14,62]])
            self.assertEqual((q['operations'][0]['x'],q['operations'][0]['y']),(208,229))
        self.assertEqual(p.ops,before)
        with self.assertRaises(ValueError):
            p.control_parts('balance_track',state='frame',frame=0,rect=[0,0,39,13])

    def test_eq_named_switches_graph_and_presets(self):
        states=('off','off-held','on','on-held')
        for name,rects in {
            'eq_on':[[10,119,26,12],[128,119,26,12],[69,119,26,12],[187,119,26,12]],
            'eq_auto':[[36,119,32,12],[154,119,32,12],[95,119,32,12],[213,119,32,12]],
        }.items():
            for state,rect in zip(states,rects):
                self.assertEqual(control_cell(name,state=state)['rect'],rect)
        self.assertEqual(control_cell('eq_presets',state='released')['rect'],[224,164,44,12])
        self.assertEqual(control_cell('eq_presets',state='held')['rect'],[224,176,44,12])
        self.assertEqual(control_cell('eq_graph',state='normal')['rect'],[0,294,113,19])
        self.assertEqual(control_cell('eq_preamp',state='normal')['rect'],[0,314,113,1])
        with self.assertRaises(ValueError):control_cell('eq_on',state='released')
        with self.assertRaises(ValueError):control_cell('eq_graph',state='held')

    def test_track_prepare_passes_frame_and_baseline_without_applying(self):
        p=CanvasPen();p.pixel(0,0,'#ffffff');baseline=[{'expected':'prior'}]
        with patch('regional_patch.prepare',return_value={'prepared':True}) as prepare, \
             patch('regional_patch.validate') as validate,patch('regional_patch.apply') as apply:
            result=p.control_prepare('groove','balance_track',state='frame',frame=27,baseline=baseline)
            self.assertEqual(prepare.call_args.args[1][0]['rect'],[9,405,38,13])
            self.assertIs(prepare.call_args.args[2],baseline)
            validate.assert_called_once_with(result);apply.assert_not_called()

    def test_control_complete_curve_and_ramp_map_once_with_local_masks(self):
        p=CanvasPen()
        p.curve([-2,1],[7.5,-3.25],[30,8],2,'#abcdef',clean_corners=True)
        p.rect(0,0,29,10,'#112233',ramp=['#112233','#ffffff'],axis=[-.5,0,28.5,9])
        original=deepcopy(p.ops)
        parts=p.control_parts('seek',state='held',rect=[3,1,20,8],exclude=[[10,3,2,4]])
        self.assertGreater(len(parts),1)
        for part in parts:
            self.assertTrue(part['clip_to_rect'])
            self.assertIsNone(_intersection(part['rect'],[288,3,2,4]))
            self.assertEqual(_intersection(part['rect'],[281,1,20,8]),part['rect'])
            curve,gradient=part['operations']
            self.assertEqual([curve['x'],curve['y'],curve['x2'],curve['y2']],[276,1,308,8])
            self.assertEqual(curve['control'],[285.5,-3.25])
            self.assertEqual(gradient['ramp_axis'],[277.5,0,306.5,9])
            self.assertEqual(len(part['operations']),2) # no implicit cell clear
        self.assertEqual(p.ops,original)

    def test_control_states_are_explicit_and_cell_ownership_is_bounded(self):
        p=CanvasPen();p.pixel(0,0,'#ffffff')
        for name,state in (('missing','released'),('shuffle','released'),('eq','on'),
                           ('stop','pressed'),([],None)):
            with self.assertRaises(ValueError):p.control_parts(name,state=state)
        with self.assertRaises(TypeError):p.control_parts('stop')
        for rect in ([0,0,23,16],[0,0,22,17],[-1,0,2,2],[0,0,0,1],[0,0,True,1]):
            with self.assertRaises(ValueError):p.control_parts('eject',state='held',rect=rect)
        for mask in ([[21,0,2,1]],[[0,0,22,16]]):
            with self.assertRaises(ValueError):p.control_parts('eject',state='held',exclude=mask)
        self.assertEqual(p.control_parts('eject',state='held')[0]['rect'],[114,16,22,16])

    def test_control_variants_do_not_mutate_source_or_other_variants(self):
        p=CanvasPen();p.stamp(1,2,['xx'],{'x':'#ddeeff'})
        normal=p.control_parts('volume',state='released')
        held=p.control_parts('volume',state='held')
        self.assertEqual((normal[0]['operations'][0]['x'],held[0]['operations'][0]['x']),(16,1))
        normal[0]['operations'][0]['palette']['x']='#000000'
        normal[0]['rect'][0]=999
        self.assertEqual(held[0]['operations'][0]['palette']['x'],'#ddeeff')
        self.assertEqual(p.ops[0]['palette']['x'],'#ddeeff')
        self.assertEqual(control_cell('volume',state='released')['rect'],[15,422,14,11])

    def test_control_prepare_validates_without_applying_and_preserves_baseline(self):
        p=CanvasPen();p.pixel(0,0,'#ffffff')
        baseline=[{'source_token':'existing-token'}]
        with patch('regional_patch.prepare',return_value={'prepared':True}) as prepare, \
             patch('regional_patch.validate') as validate,patch('regional_patch.apply') as apply:
            result=p.control_prepare('paw','stop',state='held',baseline=baseline)
            self.assertIs(prepare.call_args.args[2],baseline)
            self.assertEqual(prepare.call_args.args[1][0]['rect'],[69,18,23,18])
            validate.assert_called_once_with(result)
            apply.assert_not_called()

    def test_group_crosses_join_without_moving_local_controls_or_ramp_twice(self):
        p=CanvasPen()
        with p.group(10,110) as g:
            g.round(0,0,12,18,3,'#abcdee',ramp=['#000000','#ffffff'],axis=[.5,-.5,12.5,18.5])
            g.curve([2,2],[6.25,7.75],[11,16],2,'#ffffff')
        self.assertEqual(p.ops[0]['points'],g.ops[0]['points'])
        self.assertEqual(p.ops[0]['ramp_axis'],[10.5,109.5,22.5,128.5])
        self.assertEqual(p.ops[1]['control'],[16.25,117.75])
        active=next(t for t in p.parts([10,110,12,18]) if t['label']=='equalizer.title.active')
        self.assertEqual(active['operations'][1]['control'],[16.25,135.75])
        self.assertEqual(active['operations'][0]['ramp_axis'],[10.5,127.5,22.5,146.5])
        self.assertEqual(g.ops[1]['control'],[6.25,7.75])

    def test_reusable_motif_recolors_solid_ramp_stamp_and_preserves_source(self):
        motif=CanvasPen()
        motif.rect(0,0,4,3,'#112233',ramp=['#112233','#ddeeff'],axis=[0,0,0,3])
        motif.stamp(1,0,['xx'],{'x':'#ddeeff'})
        original=deepcopy(motif.ops)
        p=CanvasPen().place(motif,20,110).place(motif,30,110,colors={'#112233':'#345566','#ddeeff':'#aabbcc80'})
        self.assertEqual(p.ops[2]['color'],'#345566')
        self.assertEqual(p.ops[2]['ramp'],['#345566','#aabbcc80'])
        self.assertEqual(p.ops[3]['palette'],{'x':'#aabbcc80'})
        self.assertEqual(p.ops[3]['rows'],['xx'])
        self.assertEqual(motif.ops,original)
        p.ops[0]['ramp'][0]='#ffffff'
        p.ops[1]['palette']['x']='#000000'
        self.assertEqual(motif.ops,original)
        self.assertEqual(p.ops[3]['palette'],{'x':'#aabbcc80'})

    def test_group_nesting_order_and_exception_rollback(self):
        p=CanvasPen()
        with p.group(10,20) as outer:
            outer.pixel(0,0,'#112233')
            with outer.group(3,4) as inner:inner.pixel(2,1,'#ddeeff')
            outer.pixel(0,2,'#abcdef')
        self.assertEqual([(v['x'],v['y']) for v in p.ops],[(10,20),(15,25),(10,22)])
        before=deepcopy(p.ops)
        with self.assertRaisesRegex(RuntimeError,'aborted'):
            with p.group(0,0) as g:
                g.pixel(1,1,'#ffffff')
                raise RuntimeError('aborted')
        self.assertEqual(p.ops,before)

    def test_place_refuses_invalid_copy_atomically(self):
        p=CanvasPen();p.pixel(0,0,'#ffffff')
        before=deepcopy(p.ops)
        for motif,x,y,colors in (
            ([{'op':'rect','x':0,'y':0},{'op':'blur'}],0,0,None),
            (before,.5,0,None),(before,True,0,None),
            (before,0,0,{'white':'#ffffff'}),(before,0,0,[]),
        ):
            with self.assertRaises(ValueError):p.place(motif,x,y,colors=colors)
            self.assertEqual(p.ops,before)

    def test_local_group_respects_canvas_exclusion_and_keeps_complete_stroke(self):
        p=CanvasPen()
        with p.group(10,110) as g:g.line([[0,0],[20,20]],'#ffffff')
        parts=p.parts([10,110,21,21],exclude=[[16,114,3,5]])
        for target in p.plan([10,110,21,21],exclude=[[16,114,3,5]]).targets:
            self.assertIsNone(_intersection([*target['destination'],*target['rect'][2:]],[16,114,3,5]))
        self.assertTrue(all(v['operations'][0]['points']==[[0,0],[20,20]] for v in parts))

    def test_cross_join_preserves_fractional_geometry_and_ramp(self):
        p=CanvasPen()
        p.round(0,110,12,18,3,'#abcdee',ramp=['#000000','#ffffff'],axis=[.5,109.5,12.5,128.5])
        p.curve([2,112],[6.25,117.75],[11,126],2,'#ffffff')
        original=deepcopy(p.ops)
        parts=p.parts([0,110,12,18])
        by_label={v['label']:v for v in parts}
        self.assertEqual(by_label['main.background']['rect'],[0,110,12,5])
        self.assertEqual(by_label['equalizer.background']['rect'],[0,0,12,12])
        active=by_label['equalizer.title.active']
        self.assertEqual(active['rect'],[0,134,12,12])
        self.assertEqual(active['operations'][0]['ramp_axis'],[.5,127.5,12.5,146.5])
        self.assertEqual(active['operations'][1]['control'],[6.25,135.75])
        self.assertEqual(active['operations'][0]['points'],p.ops[0]['points'])
        self.assertEqual(p.ops,original)
        self.assertTrue(all(p['clip_to_rect'] for p in parts))

    def test_dock_alias_is_single_authority(self):
        p=surface_map([3,114,4,3])
        self.assertEqual([t['rect'] for t in p.targets if t['sheet']=='main.bmp'],[[3,114,4,1]])
        self.assertEqual(len(p.aliases),1)
        edge=surface_map([3,115,4,1])
        self.assertEqual(edge.targets[0]['rect'],[3,114,4,1])
        self.assertEqual(edge.targets[0]['destination'],[3,115])

    def test_fixed_playlist_title_variants(self):
        p=surface_map([90,234,20,5])
        self.assertEqual([t['rect'] for t in p.targets],[[29,23,20,5],[29,2,20,5]])
        self.assertEqual(p.skipped,[])
        self.assertEqual(p.aliases,[])

    def test_repeat_default_skip_reports(self):
        p=surface_map([26,234,4,3])
        self.assertFalse(p.targets)
        self.assertEqual(len(p.skipped),2)
        with self.assertRaises(ValueError):p.parts([{'op':'pixel','x':26,'y':234,'color':'#ffffff'}])

    def test_repeat_optin_exact_translation(self):
        p=surface_map([26,234,4,3],repeats='shared')
        self.assertEqual([t['rect'] for t in p.targets],[[128,23,4,3],[128,2,4,3]])
        self.assertEqual(len(p.aliases),2)
        self.assertIn('every instance',p.aliases[0]['effect'])

    def test_repeat_multiple_copies_refused(self):
        with self.assertRaisesRegex(ValueError,'multiple canvas copies'):
            surface_map([25,232,50,20],repeats='shared')

    def test_join_patch_cannot_silently_lose_playlist_rail(self):
        p=CanvasPen();p.line([[6,248],[6,256]],'#abcdef')
        # Previously this succeeded with only the title half of the stroke.
        with self.assertRaisesRegex(ValueError,'No patch produced'):
            p.parts([4,248,5,9])
        parts=p.parts([4,248,5,9],repeats='shared')
        self.assertTrue(any(t['label'].startswith('playlist.left.rail') for t in parts))
        self.assertTrue(any(t['label'].startswith('playlist.top.left') for t in parts))
        omitted=p.parts([4,248,5,9],repeats='skip')
        self.assertFalse(any('rail' in t['label'] for t in omitted))

    def test_footer_join_requires_explicit_shared_mapping_at_all_heights(self):
        for height in [145,146,261,384,522]:
            y=232+height-38
            p=CanvasPen();p.rect(4,y-3,3,6,'#abcdef')
            with self.assertRaisesRegex(ValueError,'No patch produced'):
                p.parts([4,y-3,3,6],playlist_height=height)
            parts=p.parts([4,y-3,3,6],playlist_height=height,repeats='shared')
            self.assertTrue(any('rail' in t['label'] for t in parts))
            self.assertTrue(any('bottom' in t['label'] for t in parts))

    def test_rails_repeat_native_period29_and_crop(self):
        first=surface_map([0,252,5,4],repeats='shared')
        second=surface_map([0,281,5,4],repeats='shared')
        self.assertEqual(first.targets[0]['rect'],second.targets[0]['rect'])
        self.assertEqual(first.targets[0]['rect'],[0,42,5,4])
        third=surface_map([260,310,5,4],repeats='shared')
        self.assertEqual(third.targets[0]['rect'],[36,42,5,4])

    def test_runtime_and_custom_masks_are_exact_native_clips(self):
        p=surface_map([45,24,25,20],exclude=[[45,24,2,20]])
        for t in p.targets:
            d=[*t['destination'],*t['rect'][2:]]
            for forbidden in runtime_rects()+[[45,24,2,20]]:
                self.assertIsNone(_intersection(d,forbidden))
        self.assertTrue(p.readonly)
        allowed=surface_map([48,26,9,13],preserve_runtime=False)
        self.assertEqual(allowed.targets[0]['rect'],[48,26,9,13])

    def test_footer_height_and_runtime(self):
        p=surface_map([125,339,10,10],preserve_runtime=False)
        self.assertEqual(p.targets[0]['rect'],[126,72,10,10])
        tall=surface_map([125,455,10,10],playlist_height=261,preserve_runtime=False)
        self.assertEqual(tall.targets[0]['rect'],p.targets[0]['rect'])

    def test_whole_canvas_never_silently_creates_extensions(self):
        p=surface_map([0,0,275,377])
        self.assertLessEqual(len(p.targets),64)
        self.assertEqual(set(t['sheet'] for t in p.targets),{'main.bmp','eqmain.bmp','titlebar.bmp','pledit.bmp'})
        self.assertTrue(any(s['label']=='playlist.list' for s in p.skipped))

    def test_unsupported_and_invalid_refused(self):
        for kwargs in ({'titles':'maybe'},{'repeats':True},{'playlist_height':144},{'preserve_runtime':'yes'}):
            with self.assertRaises(ValueError):surface_map([0,0,1,1],**kwargs)
        for rect in ([0,0,276,377],[0,377,1,1],[-1,0,1,1],[0,0,0,1]):
            with self.assertRaises(ValueError):surface_map(rect)
        with self.assertRaises(ValueError):surface_parts([{'op':'blur','x':0,'y':0}],[0,0,1,1])


if __name__=='__main__':unittest.main()
