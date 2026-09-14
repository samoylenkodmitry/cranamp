#!/usr/bin/env python3
"""Read-only registration tests; synthetic fixtures contain no exported artwork."""
import random
import unittest
from PIL import Image
from locate_crop import locate,block_grid


def scene(w=28,h=24):
    rng=random.Random(42)
    im=Image.new('RGB',(w,h))
    im.putdata([tuple(rng.randrange(30,210) for _ in range(3)) for _ in range(w*h)])
    return im


def integer_blocks(im,scale):
    out=Image.new('RGB',(im.width*scale,im.height*scale))
    src=im.load();out.putdata([src[x//scale,y//scale] for y in range(out.height) for x in range(out.width)])
    return out


class CropTests(unittest.TestCase):
    def test_exact_with_profile_delta_and_partial_edge(self):
        base=scene(); ref=integer_blocks(base,2)
        crop=integer_blocks(base,4).crop((21,29,56,64))
        crop.putdata([tuple(v+4 for v in c) for c in [crop.getpixel((x,y)) for y in range(crop.height) for x in range(crop.width)]])
        r=locate(ref,crop,padding=4)
        self.assertEqual(r['grid']['scale'],4)
        self.assertEqual(r['candidates'][0]['rect'],[5,7,9,9])
        self.assertEqual(r['candidates'][0]['fractional_origin'],[5.25,7.25])
        self.assertEqual(r['confidence'],'high')
    def test_outside_canvas(self):
        base=scene(); extended=Image.new('RGB',(32,30));extended.paste(base,(0,4))
        crop=integer_blocks(extended,4).crop((100,0,124,40))
        r=locate(integer_blocks(base,2),crop,padding=8)
        self.assertEqual(r['candidates'][0]['rect'],[25,-4,6,10])
        self.assertEqual(r['candidates'][0]['intersection'],[25,0,3,6])
    def test_uniform_is_ambiguous(self):
        r=locate(Image.new('RGB',(40,40),'#abcdef'),Image.new('RGB',(8,8),'#abcdef'),padding=0)
        self.assertEqual(r['confidence'],'ambiguous')
    def test_repeated_motif_is_ambiguous(self):
        base=scene(); base.paste(base.crop((2,2,7,7)),(15,12))
        r=locate(integer_blocks(base,2),integer_blocks(base.crop((2,2,7,7)),4),padding=0)
        self.assertEqual(r['confidence'],'ambiguous')
        self.assertEqual(r['candidate_gap'],0)
    def test_unrelated_reports_poor(self):
        r=locate(integer_blocks(scene(),2),Image.new('RGB',(16,16),'white'),padding=0)
        self.assertEqual(r['confidence'],'poor')
    def test_reject_noninteger_reference(self):
        with self.assertRaisesRegex(ValueError,'not exact'):
            locate(scene(),Image.new('RGB',(8,8)),padding=0)
    def test_explicit_scale_phase(self):
        im=integer_blocks(scene(),4).crop((1,2,31,32))
        g=block_grid(im,exact_scale=4)
        self.assertEqual(g['phase'],[3,2])
        self.assertEqual(g['agreement'],1)

if __name__=='__main__':unittest.main()
