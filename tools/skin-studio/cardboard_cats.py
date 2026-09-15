"""The cats of Catamp Cardboard, as hand-authored native pixel grids.

A cat in a box is mostly not visible, which is the whole joke and also what
makes it drawable at this size: a head over a slot, a paw on a slider, a tail
down an edge, two eyes in the dark. Nothing here is a portrait, so nothing here
needs more pixels than an 11x11 equalizer head has.

These were first generated from a toe count and a width, which kept every toe
aligned and made every paw read as a row of bricks. Character at eleven pixels
is asymmetry, and asymmetry has to be typed.

    o  outline, deepest fur      p  paw pad            e  eye
    d  fur in shadow             P  pad in shadow      k  ink: nose, mouth
    f  fur                       -  shadow cast into an opening
    l  fur catching light        =  a shape seen in the gloom inside the box
    w  the pale fur of chin, chest and toes
"""
from cardboard_material import (EYE, EYE_DIM, FUR, FUR_DARK, FUR_LIT, FUR_PALE,
                                FUR_SHADE, GLOW, GLOW_DIM, HOLE_DEEP,
                                HOLE_SOFT, INK, PAD, PAD_DARK)

PALETTE = {'o': FUR_SHADE, 'd': FUR_DARK, 'f': FUR, 'l': FUR_LIT, 'w': FUR_PALE,
           'p': PAD, 'P': PAD_DARK, 'e': EYE, 'E': EYE_DIM, 'k': INK,
           '-': HOLE_DEEP, '=': HOLE_SOFT, '+': GLOW_DIM, '*': GLOW}

# Eleven of these come up through eleven slots cut in the box lid: the
# equalizer. 11x11 is the classic handle cell, and a head is the one cat shape
# that survives it -- two ears and two eyes are read before anything else.
#
# Every one of these was first drawn with `w` for the whole body, which made
# each of them a bright rounded mass inside a dark outline: a loaf of bread, a
# drumstick, a blob. Ginger fur is `f` and `l`; `w` is only the muzzle, chest,
# chin and toes, and `d` does the tabby markings.
EQ_HEAD = [
    ' o       o',
    'ooo     ooo',
    'odfo   ofdo',
    'odffdddffdo',
    'offfdfdfffo',
    'ofeefdfeefo',
    'ofeefffeefo',
    'offwwkwwffo',
    ' offwkwwffo',
    '  oddddddo',
    '    oooo',
]

# Grabbed: the ears go back and the eyes shut, so the cat flinches rather than
# merely changing colour.
EQ_HEAD_HELD = [
    '',
    'oo       oo',
    'odo     odo',
    'odffdddffdo',
    'offfdfdfffo',
    'ofkkfdfkkfo',
    'offfffffffo',
    'offwwkwwffo',
    ' offwkwwffo',
    '  oddddddo',
    '    oooo',
]

# The volume and balance handles: a cat loafed along the groove, 14x11, head
# left, tail curled over the right.
SLIDER_LOAF = [
    ' oo  oo',
    'odfoodfo   ooo',
    'odffffffooofdo',
    'ofewffewfffdfo',
    'ofwkkwffffffdo',
    'ofwwffdffffldo',
    'odffffffffffdo',
    'odfddffddfflfo',
    ' odffffffffdo',
    '  oddddddddo',
    '   oooooooo',
]

SLIDER_LOAF_HELD = [
    '',
    'oo  oo     ooo',
    'odoodfo   ofdo',
    'odffffffooofdo',
    'ofkwffkwfffdfo',
    'ofwkkwffffffdo',
    'ofwwffdffffldo',
    'odffffffffffdo',
    'odfddffddfflfo',
    ' odffffffffdo',
    '  oddddddddo',
]

# A whole cat sitting on the playlist scrollbar, 8x18, facing out.
SCROLL_CAT = [
    ' o    o',
    'ooo  ooo',
    'odfoofdo',
    'odffffdo',
    'ofeffefo',
    'ofwkkwfo',
    ' offwfo',
    ' offffo',
    'oofwwfoo',
    'ofwwwwfo',
    'offwwffo',
    'offwwffo',
    'odfwwfdo',
    'odffffdo',
    'odfffldo',
    ' odfffo',
    '  oddo',
    '   oo',
]

SCROLL_CAT_HELD = [
    '',
    'oo    oo',
    'odo  odo',
    'odffffdo',
    'ofkffkfo',
    'ofwkkwfo',
    ' offwfo',
    ' offffo',
    'oofwwfoo',
    'ofwwwwfo',
    'offwwffo',
    'offwwffo',
    'odfwwfdo',
    'odffffdo',
    'odfffldo',
    ' odfffo',
    '  oddo',
    '   oo',
]

# A kitten walking the seek bar, 29x10, head leading, tail up behind, white
# socks on all four feet.
KITTEN_WALK = [
    '  oo                   ooooo',
    ' odfo                 oodffdo',
    ' odfo                oofwwwfo',
    ' odfo      ooooo    oofeewefo',
    ' offo   ooofffffoooofwwkkwwfo',
    ' offoooffffffdfffffffwwwkwfo',
    '  offfffdffffffdffffffffdo',
    '  odffffffffdffffffffffdo',
    '  odfoowffoowffoowffowfdo',
    '   oo  ow    ow    ow  ow',
]

KITTEN_HELD = [
    '   oo                  ooooo',
    '  odfo                oodffdo',
    ' oodfo               oofwwwfo',
    ' odfo      ooooo    oofkkwkfo',
    ' offo   ooofffffoooofwwkkwwfo',
    ' offoooffffffdfffffffwwwkwfo',
    '  offfffdffffffdffffffffdo',
    '  odffffffffdffffffffffdo',
    '  odfoowffoowffoowffowfdo',
    '    oo ow     ow    ow ow',
]

# A cat sitting so far back in the box that only its outline is lit, and its
# eyes. Drawn in the greys of the interior with one warm rim, never in fur
# colours: it has to stay behind the track title, not compete with it.
CAT_IN_THE_DARK = [
    '     ==      ==        ==',
    '    ====    ====      ====',
    '    ===========      ===',
    '   +============    ===',
    '   +=e=====e====   ===',
    '   +============  ===',
    '   ====kkk====== ===',
    '    =========== ===',
    '    ===========+===',
    '   +==============',
    '  +================',
    ' +==================',
    ' ====================',
    '=====================',
    '=====================',
]

# Another pair of eyes, further back, at the dark end of the timer window.
EYES_IN_THE_DARK = [
    ' ==      ==',
    '===     ===',
    '===========',
    '==e=====e==',
    '===========',
    '====kkk====',
    ' =========',
    '  =======',
]

# Asleep in the corner of the playlist footer: a body, with the head that goes
# on top kept separate so the muzzle can be placed after the fur.
CAT_CURLED = [
    '         ooooooooo',
    '      oooddddddddooo',
    '    ooddffffffffffddoo',
    '   odffflffffffflfffdo',
    '  odffdfflffffflffdffdo',
    ' odffffffflllllffffffffo',
    ' offdfffffffffffffffdffo',
    'odfffffffffffffffffffffdo',
    'offfdffffffffffffffffdfffo',
    'offffffffwwwwwwffffffffffdo',
    'offdfffwwwwwwwwwwfffdffffdo',
    'odffffwwwwwwwwwwwwwffffffdo',
    ' offdfwwwwwwwwwwwwwwfdffffo',
    ' oodffwwwwwwwwwwwwwwwffdffo',
    '   oodfffwwwwwwwwwwwffffffo',
    '     oodfffflwwwwwlffffffdo',
    '       ooddfffflllllfffffdo',
    '         oooddddffffffdddo',
    '            oooodddddoooo',
    '                oooo',
]

# The tucked head that sits on the curled body: 13x11.
SLEEPING_HEAD = [
    ' oo       oo',
    'odfo     odfo',
    'odffooooooffdo',
    'odfffdddffffdo',
    'offfdfffdffffo',
    'offkkfffkkfffo',
    'offwwwkwwwfffo',
    ' offwwkwwwffo',
    '  oddwwwddoo',
    '    oooooo',
]

# A printed paw mark, the kind a cat leaves on a box it has walked over.
# Three toes, not two: two toes over a wide pad read as a face.
PRINT_MARK = [
    ' pp  pp  pp',
    ' pp  pp  pp',
    '',
    '   ppppp',
    '  ppppppp',
    '  ppppppp',
    '   ppppp',
]


def check():
    """Right-pad every grid to a rectangle and prove every symbol has a colour.

    Trailing spaces carry no pixels, so requiring them typed was only a way to
    turn a shape into a width puzzle. Padding happens once, at import.
    """
    grids = {name: value for name, value in globals().items()
             if name.isupper() and isinstance(value, list)}
    for name, grid in grids.items():
        width = max(len(row) for row in grid)
        unknown = {c for row in grid for c in row if c != ' '} - set(PALETTE)
        assert not unknown, f'{name} uses undefined symbols {sorted(unknown)}'
        grid[:] = [row.ljust(width) for row in grid]
    return {name: (len(g[0]), len(g)) for name, g in sorted(grids.items())}


check()


if __name__ == '__main__':
    for name, size in check().items():
        print(f'{name:20} {size[0]}x{size[1]}')
