// Optional live-GPU integration check; start Skin Studio before running this file.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import {call} from '../../tools/skin-studio/midnight_snack.mjs';
const root=path.resolve(import.meta.dirname,'../..');
const output=path.join(root,'target/midnight-snack-gpu');
await fs.mkdir(output,{recursive:true});
const status=await call('studio_status');
assert.equal(status.dirty,false,'Save Studio work before the GPU check.');
const project=path.join(root,'assets/skins/Catamp Midnight Snack.cstudio');
const archive=path.join(root,'assets/skins/Catamp Midnight Snack.wsz');
const standard={active:true,pressed:false,playback:0,position:10,volume:20,balance:14,scroll:5,
 eq:[14,23,18,13,9,6,9,13,17,21,24],zoom:2,presentation:true,preview_playlist_height:145};
async function shot(file){return call('studio_screenshot',{presentation:true,panel:'all',path:path.join(output,file)});}
try{
 await call('studio_project',{action:'open',path:project});
 await call('studio_canvas',standard);
 await shot('project.png');
 await call('studio_open',{path:archive});
 await call('studio_canvas',standard);
 await shot('export.png');
  assert.deepEqual(await fs.readFile(path.join(output,'export.png')),await fs.readFile(path.join(output,'project.png')),
   'The exported classic skin must have exactly the same GPU pixels as its layered project.');
   const palette=await call('studio_options');
   assert.equal(palette.visualizer_background.transparent,true);
   const spectrum=[24,43,76,16];
   await call('studio_canvas',{...standard,zoom:1,presentation:false,crop:spectrum,magnify:1,path:path.join(output,'spectrum-editor.png')});
   await call('studio_screenshot',{presentation:true,panel:'all',crop:spectrum,magnify:1,path:path.join(output,'spectrum-gpu.png')});
   assert.deepEqual(await fs.readFile(path.join(output,'spectrum-gpu.png')),await fs.readFile(path.join(output,'spectrum-editor.png')),
    'Stopped spectrum must reveal the same artwork as the editor, without a runtime rectangle.');
   await call('studio_canvas',standard);
  const validation=await call('studio_validate');
  assert.deepEqual(validation.divergences,[]);
  assert.equal(validation.plays_the_same_elsewhere,false,'Color-key support in other players is unverified.');
  assert.deepEqual(validation.transparency.opaque_moving_sprites,[]);
  assert.equal(validation.transparency.moving_cells.length,10);
  assert.ok(validation.transparency.moving_cells.every(c=>c.key_pixels>0&&c.opaque_pixels>0&&c.alpha_pixels===0&&c.missing_pixels===0));
  await fs.writeFile(path.join(output,'transparency.json'),JSON.stringify(validation.transparency,null,2));
  const preamp=await call('studio_pixel',{x:100,y:142});
  assert.equal(preamp.hits.find(h=>h.layer==='equalizer.preamp.line').sprite_key,true,'The old preamp stripe must not cover the cat.');
  for(let frame=0;frame<28;frame++){
   await call('studio_canvas',{eq:Array(11).fill(frame)});
   const pixel=await call('studio_pixel',{x:80,y:200});
   assert.equal(pixel.hits.find(h=>h.layer==='equalizer.band1.track').sprite_key,true,'Track backdrop must reveal the unique cloth in every frame.');
  }
  for(let height=145;height<174;height++){
   await call('studio_canvas',{preview_playlist_height:height});
    const seam=await call('studio_pixel',{x:12,y:194+height,width:243,height:1,palette:true});
    assert.equal(seam.palette.unique_colors,1,'Footer shadow must meet the playlist fill across its entire top edge.');
    assert.equal(seam.palette.colors[0].hex,palette.playlist_colors.NormalBG+'ff');
   for(const x of [3,271]){
    const line=await call('studio_pixel',{x,y:223,width:1,height:height+9,palette:true});
    assert.equal(line.palette.unique_colors,1,'The outer stroke must cross header, every repeated tile and footer without a gap.');
   }
  }
  await call('studio_canvas',standard);
 const options=await call('studio_options');
 await fs.writeFile(path.join(output,'readability.json'),JSON.stringify(options,null,2));
 for(const frame of [0,13,27])for(const active of [true,false])for(const pressed of [false,true]){
  await call('studio_canvas',{...standard,position:frame,volume:frame,balance:frame,scroll:frame,eq:Array(11).fill(frame),active,pressed});
  await shot('frame-'+frame+'-'+Number(active)+Number(pressed)+'.png');
 }
 for(const height of [146,158,174,261,522]){
  await call('studio_canvas',{...standard,preview_playlist_height:height,zoom:1});
  await shot('height-'+height+'.png');
 }
   console.log('PASS: project/WSZ GPU pixels match; spectrum reveals editor artwork; ten keyed silhouettes; all 28 EQ track frames; continuous borders and footer shadow at all 29 tile phases; 12 state/slider captures and five playlist heights.');
}finally{
 await call('studio_project',{action:'open',path:project});
 await call('studio_canvas',standard);
}
