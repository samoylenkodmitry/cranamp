"""Silverplay 22 main illustration: quiet silver fur and carved aquamarine.

Pure native Studio Pen recipe.  Owns x14..268 / y14..109 and the existing
cat-ear title strip. Only static main/title
backgrounds are painted; live number, text and control atlases are untouched.
"""
from connected_canvas import CanvasPen
from atelier22_common import LetterPen as Art

INK = '#102b40'
FRAME = '#193e55'
WELL = '#21465c'
MID = '#32627b'
TEAL = '#508ea3'
CYAN = '#8cc5d2'
ICE = '#c7e8e7'
SHINE = '#f1f5e6'
FUR_DARK = '#687889'
FUR = '#9da8ad'
LIGHT = '#d4d6ca'
IVORY = '#f1ebd5'
ROSE = '#b98190'


EDGE = "#304355"
DARK = FUR_DARK

def background(p):
    # One continuous stepped crystal edge, matching EQ/PL sidewall columns.
    p.rect(0,0,275,115,'#ff00ff')
    p.path([[8,0],[267,0],[272,3],[274,8],[274,114],[0,114],[0,8],[3,3]],INK)
    p.path([[8,1],[266,1],[271,4],[273,8],[273,114],[1,114],[1,8],[4,4]],MID)
    p.path([[8,2],[265,2],[270,5],[272,9],[272,114],[2,114],[2,9],[5,5]],CYAN)
    p.path([[9,3],[265,3],[269,6],[271,10],[271,114],[3,114],[3,10],[6,6]],ICE)
    p.path([[9,4],[264,4],[269,8],[270,11],[270,114],[4,114],[4,11],[7,7]],TEAL)
    for x,c in [(0,INK),(1,MID),(2,CYAN),(3,ICE),(4,TEAL),(5,FRAME),(269,FRAME),(270,MID),(271,TEAL),(272,CYAN),(273,MID),(274,INK)]:
        p.rect(x,12,1,103,c)
    p.path([[10,5],[264,5],[268,8],[269,12],[269,114],
        [6,114],[6,12],[7,8]],FRAME)
    p.path([[8,15],[119,15],[107,23],[19,24],[12,34],[9,62],[6,70],[6,21]],MID)
    p.path([[8,17],[99,17],[87,20],[19,21],[12,27],[9,49]],TEAL)
    p.line([[10,18],[28,18]],CYAN)
    # Shelves are lit physically at their front edge. No diagonal stickers.
    p.path([[9,72],[262,72],[268,77],[268,99],[264,106],[13,106],[8,100],[8,79]],INK)
    p.path([[12,75],[261,75],[264,79],[264,99],[261,103],[14,103],[11,99],[11,80]],MID)
    p.path([[13,75],[260,75],[263,78],[12,78]],TEAL)
    p.line([[15,75],[94,75]],CYAN)
    p.line([[102,75],[169,75]],TEAL)
    p.path([[12,98],[31,100],[241,100],[264,96],[261,103],[14,103]],FRAME)
    p.line([[19,104],[92,104]],TEAL)
    p.line([[184,104],[258,104]],MID)
    # Quiet continuous dark ground through the main/EQ docking seam.
    p.rect(6,108,264,7,FRAME)
    # Title: small punched silver letters, no independent title panel.
    a=Art();a.glyph('CATAMP',140,5,CYAN);p.ops+=a.ops
    p.line([[189,8],[226,8]],MID)
    p.stamp(232,5,['  s  ',' sss ','sssss',' sss ','  s  '],{'s':TEAL})

def tabby(p):
    # Full skull silhouette. The high ear tips are asymmetrical and turn inward.
    p.path([[31,3],[36,4],[51,16],[61,13],[78,13],[94,17],[107,3],[112,2],
        [111,21],[111,28],[109,33],[110,37],[107,39],[107,43],[100,47],
        [91,48],[73,49],[48,48],[34,43],[28,43],[23,40],[28,36],[25,32],
        [29,26],[30,17]],EDGE)
    # Head: large ivory left forehead, cool shaded right turn, connected cheeks.
    p.path([[33,6],[38,10],[51,20],[62,16],[78,16],[94,20],[109,6],
        [108,22],[109,28],[107,33],[108,36],[105,39],[106,41],[97,44],
        [87,44],[78,48],[64,48],[53,46],[43,45],[34,41],[29,38],[32,35],
        [28,32],[33,25]],FUR)
    p.path([[49,21],[58,17],[69,15],[75,17],[73,25],[66,30],[60,38],
        [54,42],[44,41],[36,37],[35,31],[40,27]],LIGHT)
    p.path([[52,20],[62,17],[69,17],[70,20],[66,24],[58,26],[53,24]],IVORY)
    p.path([[77,17],[88,19],[96,23],[103,27],[106,33],[104,38],[96,42],
        [87,42],[78,45],[74,40],[79,33]],DARK)
    p.path([[76,18],[82,19],[85,23],[82,28],[77,32],[74,32],[73,24]],LIGHT)
    # Left ear cavity follows cartilage, not a triangle pasted on the head.
    p.path([[34,10],[46,21],[47,26],[40,27],[34,23]],DARK)
    p.path([[36,13],[43,20],[44,23],[40,23],[36,20]],ROSE)
    p.path([[38,17],[42,21],[40,21],[38,20]],'#d4aaa4')
    p.line([[32,7],[33,18],[35,23]],IVORY)
    p.path([[34,23],[36,21],[40,25],[41,23],[44,27],[39,28]],LIGHT)
    # Right ear remains appreciably darker: one light-bearing outer rim.
    p.path([[107,10],[105,24],[98,28],[96,24]],EDGE)
    p.path([[105,15],[104,22],[101,24],[100,22]],'#976d7c')
    p.line([[109,7],[108,18],[107,22]],LIGHT)
    p.path([[99,25],[103,22],[103,25],[100,27],[96,27]],FUR)
    # Tabby M: chunky curved tapered locks that follow brow volume.
    p.path([[51,19],[55,18],[60,23],[62,28],[59,28],[56,23]],DARK)
    p.path([[63,16],[66,16],[68,21],[67,26],[65,28],[64,22]],DARK)
    p.path([[72,17],[76,17],[75,23],[72,27],[71,25],[73,21]],EDGE)
    p.path([[83,19],[88,20],[87,24],[82,28],[79,28],[84,23]],EDGE)
    p.path([[91,22],[95,24],[96,27],[90,29],[86,28]],DARK)
    p.line([[58,19],[61,21],[62,24]],IVORY)
    p.line([[68,17],[70,19],[70,22]],IVORY)
    p.line([[78,20],[78,23],[76,26]],FUR)
    # Cheek masks turn from eyes into muzzle with broad, connected fur locks.
    p.path([[32,28],[40,29],[44,32],[43,35],[37,34],[32,32]],DARK)
    p.path([[32,34],[36,36],[43,36],[46,39],[41,40],[35,38]],IVORY)
    p.path([[32,38],[38,40],[43,40],[47,43],[42,43],[36,42]],DARK)
    p.path([[100,29],[107,29],[110,32],[105,34],[99,34]],EDGE)
    p.path([[101,35],[108,34],[107,37],[102,39],[96,39]],FUR)
    p.path([[98,39],[104,39],[101,42],[95,43],[90,43]],LIGHT)
    # Narrow crystal spectacles, held by brow straps. Four digit cells retain
    # original coordinates; the framing curves beyond them rather than boxing.
    p.line([[33,28],[42,29],[47,29]],EDGE,2)
    p.line([[33,27],[42,28]],TEAL)
    p.line([[99,29],[106,27]],EDGE,2)
    for x in (46,76):
        p.path([[x-3,29],[x,24],[x+6,23],[x+21,24],[x+25,28],
            [x+25,36],[x+21,40],[x+6,41],[x,38]],EDGE)
        p.path([[x-2,29],[x+1,25],[x+6,24],[x+20,25],[x+24,28],
            [x+24,35],[x+20,39],[x+6,40],[x+1,37]],TEAL)
        p.path([[x,29],[x+3,25],[x+19,25],[x+23,28],[x+23,36],
            [x+19,39],[x+4,39],[x,36]],ICE)
        p.line([[x+2,26],[x+6,24],[x+17,24]],SHINE)
        p.line([[x+4,40],[x+18,40],[x+22,37]],MID)
    # Nose, separate muzzle pads and chin. The spectrum rim starts below them.
    p.path([[67,36],[72,35],[77,36],[80,39],[80,42],[75,44],
        [69,44],[64,42],[64,39]],LIGHT)
    p.path([[65,39],[69,38],[72,40],[71,43],[66,42]],IVORY)
    p.path([[74,39],[77,38],[80,40],[77,43],[73,42]],LIGHT)
    p.path([[69,37],[75,37],[74,39],[72,40],[70,39]],ROSE)
    p.line([[70,37],[74,37]],'#d4aaa4')
    p.line([[72,40],[72,42],[69,43]],EDGE)
    p.line([[73,42],[76,43]],DARK)
    p.line([[64,40],[55,39],[47,37]],IVORY)
    p.line([[64,42],[55,42],[49,43]],LIGHT)
    p.line([[80,40],[88,39],[96,36]],LIGHT)
    p.line([[80,42],[89,42],[94,41]],FUR)
    # One continuous laughing-cat lower face. The mouth reserves the complete
    # fixed spectrum box; its corner fangs, tongue and fur chin sit outside it.
    p.path([[28,37],[36,40],[47,41],[61,40],[73,41],[86,41],
        [100,37],[107,39],[112,44],[113,51],[110,58],[104,64],
        [96,68],[85,70],[62,72],[42,70],[28,66],[20,60],
        [16,52],[17,45],[22,40]],EDGE)
    p.path([[28,39],[37,42],[49,43],[63,42],[74,43],[87,43],
        [100,40],[106,42],[110,46],[110,51],[107,57],[102,62],
        [94,66],[83,70],[62,72],[43,70],[30,64],[23,58],
        [20,52],[20,46],[24,42]],FUR)
    # Upper-left cheek turns into an ivory jaw; right-side mass stays cooler.
    p.path([[26,41],[31,42],[26,47],[25,53],[28,60],[35,64],
        [44,67],[62,69],[80,68],[86,67],[83,70],[63,72],
        [44,70],[30,64],[23,58],[20,52],[21,46]],LIGHT)
    p.path([[24,44],[27,43],[24,49],[24,53],[27,58],
        [33,62],[43,66],[39,66],[30,62],[24,57],[22,52],[22,47]],IVORY)
    p.path([[102,42],[107,45],[108,50],[106,56],[102,61],
        [94,65],[82,69],[70,70],[84,67],[96,62],[101,55],
        [103,49]],DARK)
    # A warm gum/lip silhouette with rounded corner mass explains the dark
    # acoustic interior as a mouth, rather than another crystal display panel.
    p.path([[24,41],[39,42],[55,42],[74,43],[95,41],[104,42],
        [107,46],[107,53],[103,59],[96,63],[83,66],[60,67],
        [42,65],[28,60],[21,55],[19,49],[20,45]],'#6c4e61')
    p.path([[23,43],[39,43],[55,44],[76,44],[97,43],[103,44],
        [105,47],[105,53],[101,58],[94,61],[82,64],[60,65],
        [43,63],[29,58],[23,54],[21,49]],'#443b50')
    p.path([[23,43],[100,43],[103,46],[103,54],[99,58],
        [92,61],[80,63],[60,64],[44,62],[30,58],[23,54],[21,49]],FRAME)
    p.rect(24,43,76,16,FRAME)
    # The tongue is a single connected warm cluster under the live waveform.
    p.path([[53,60],[59,59],[68,59],[75,60],[79,62],[74,64],
        [63,65],[56,64],[51,62]],'#895d70')
    p.path([[57,60],[63,60],[65,61],[68,60],[73,61],[75,63],
        [68,64],[61,64],[56,62]],ROSE)
    p.line([[65,61],[65,63]],'#6c4e61')
    p.line([[58,60],[61,60]],'#d4aaa4')
    # Corner canines remain outside x24..99: they survive every spectrum state.
    p.path([[20,44],[23,44],[23,50],[22,53],[20,48]],IVORY)
    p.line([[23,46],[23,49]],LIGHT)
    p.path([[100,43],[104,43],[104,47],[101,53],[100,49]],LIGHT)
    p.path([[100,43],[102,44],[102,47],[101,50],[100,47]],IVORY)
    # Fur is grouped along the curved chin; no mitten/arm silhouettes remain.
    p.path([[31,62],[42,65],[56,67],[68,68],[83,66],[96,61],
        [94,64],[84,68],[69,71],[55,71],[43,69],[34,66]],LIGHT)
    p.path([[35,64],[44,66],[54,68],[66,69],[76,68],[84,67],
        [80,69],[67,71],[55,70],[45,69],[38,67]],IVORY)
    p.line([[32,62],[37,64],[41,64]],DARK)
    p.line([[88,66],[93,64],[96,61]],FUR)
    p.path([[100,39],[105,41],[106,44],[102,43],[99,41]],LIGHT)
    # Muzzle pads and nose reconnect clearly to the upper lip above the cavity.
    p.path([[59,39],[64,37],[69,38],[73,40],[71,42],
        [64,43],[59,42],[56,40]],IVORY)
    p.path([[76,38],[82,37],[88,39],[89,41],[83,43],[76,42],
        [73,40]],LIGHT)
    p.path([[69,37],[75,37],[76,38],[73,41],[71,40]],ROSE)
    p.line([[70,37],[74,37]],'#d4aaa4')
    p.line([[73,41],[72,43]],EDGE)
    p.line([[58,40],[48,38],[42,38]],IVORY)
    p.line([[58,42],[47,42],[40,43]],LIGHT)
    p.line([[88,40],[99,38],[104,36]],LIGHT)

def silver_cat(p):
    # Keep the established timer-eye coordinates, ear silhouette, and brow M.
    # The previous open jaw is completely lifted before the new small jaw.
    tabby(p)
    p.rect(14,39,100,35,FRAME)

    # Ear cartilage uses the same four silver clusters as the cheek.  The
    # left-facing plane receives warm light; its right counterpart stays cool.
    p.path([[34,10],[45,21],[46,25],[40,27],[34,23]], FUR_DARK)
    p.path([[36,14],[43,20],[43,23],[39,22],[36,20]], ROSE)
    p.line([[33,9],[33,18],[35,23]], IVORY)
    p.path([[35,23],[39,24],[40,23],[44,26],[41,28],[36,26]], LIGHT)
    p.path([[107,11],[105,23],[99,27],[97,24]], INK)
    p.path([[105,16],[103,23],[101,24],[100,22]], ROSE)
    p.line([[109,7],[108,18],[107,22]], LIGHT)
    p.path([[100,25],[103,24],[103,26],[99,29],[96,27]], FUR)

    # A complete jaw wraps OUTSIDE every corner of the 76x16 spectrum.
    # Broad side-cheek masses physically join the brow to the light chin.
    p.path([[28,36],[37,39],[51,40],[63,39],[73,40],[89,39],
            [102,36],[108,39],[112,44],[113,51],[111,58],[105,63],
            [95,68],[81,70],[61,70],[44,70],[30,67],[21,61],
            [16,56],[15,49],[17,43],[22,39]], INK)
    p.path([[28,38],[37,41],[52,42],[64,41],[73,42],[89,41],
            [101,39],[106,41],[110,45],[111,51],[109,57],[103,61],
            [94,66],[80,68],[61,70],[45,68],[31,65],[23,60],
            [18,55],[17,49],[19,44],[24,41]], FUR)
    p.path([[26,40],[30,41],[24,45],[22,50],[23,56],[29,60],
            [39,65],[52,68],[67,68],[79,68],[82,68],[62,70],
            [45,68],[31,65],[23,60],[18,55],[17,49],[19,44]], LIGHT)
    p.path([[21,45],[23,44],[20,49],[20,53],[23,57],[30,62],
            [39,66],[48,68],[44,68],[31,64],[24,60],[19,54],
            [18,49]], IVORY)
    p.path([[105,42],[109,46],[110,51],[108,56],[103,60],
            [94,65],[82,68],[74,69],[84,66],[96,60],[102,55],
            [104,49]], FUR_DARK)

    # Rounded mouth cavity with no rectangular paint operation.  It encloses
    # x24..99 all the way through y58; corner gums soften its upper contour.
    p.path([[24,42],[38,43],[55,43],[71,44],[88,43],[102,42],
            [107,46],[108,51],[106,57],[101,60],[92,64],[79,67],
            [60,68],[43,65],[30,61],[22,58],[19,53],[19,48],[21,44]], FUR_DARK)
    p.path([[24,43],[39,44],[55,44],[72,44],[88,44],[101,43],
            [105,46],[106,51],[104,57],[100,60],[91,63],[78,66],
            [60,66],[44,63],[31,60],[22,58],[20,53],[20,48],[22,45]], FRAME)
    # A warm tongue is anchored at the bottom of the cavity, with one cleft.
    p.path([[52,62],[58,60],[65,60],[69,61],[75,60],[81,62],
            [78,64],[70,66],[61,66],[55,64]], FUR_DARK)
    p.path([[55,62],[60,61],[65,61],[69,62],[75,61],[78,62],
            [75,64],[68,65],[61,65],[57,64]], ROSE)
    p.line([[68,62],[68,64]], FUR_DARK)
    p.line([[59,61],[63,61]], LIGHT)
    # Short corner teeth never cross into the live display rectangle.
    p.path([[20,45],[23,44],[23,50],[22,54],[20,49]], IVORY)
    p.line([[23,46],[23,49]], LIGHT)
    p.path([[100,44],[104,44],[104,48],[101,54],[100,50]], LIGHT)
    p.path([[100,44],[102,45],[102,48],[101,51],[100,48]], IVORY)

    # Muzzle pads overlap the continuous top lip. Whiskers emerge from these
    # connected pale shapes, with a darker outer cheek visible on both sides.
    p.path([[43,38],[51,38],[57,39],[62,37],[67,37],[72,40],
            [69,42],[61,43],[54,42],[48,41]], IVORY)
    p.path([[74,39],[79,37],[84,37],[90,39],[99,37],[99,39],
            [92,41],[86,43],[79,43],[74,41]], LIGHT)
    p.path([[68,37],[75,37],[76,38],[72,41],[69,39]], ROSE)
    p.line([[69,37],[74,37]], IVORY)
    p.line([[72,41],[72,43]], INK)
    p.line([[52,40],[42,38],[35,37]], IVORY)
    p.line([[53,42],[42,42],[33,44]], LIGHT)
    p.line([[89,40],[97,38],[103,35]], LIGHT)
    p.path([[28,38],[32,38],[35,40],[30,40],[26,43],[24,42]], LIGHT)
    p.path([[101,38],[105,38],[108,42],[103,41]], FUR)
    # Spectacle rims turn into their temples instead of cutting the face flat.
    p.line([[46,37],[49,39],[54,40],[65,40],[68,38]], MID)
    p.line([[50,39],[54,39],[63,39]], CYAN)
    p.line([[77,38],[80,40],[94,40],[99,37]], MID)
    p.line([[81,39],[94,39],[97,37]], CYAN)
    # Connected lower fur follows the jaw volume and the upper-left light.
    p.path([[32,63],[44,67],[57,68],[70,69],[82,68],[93,65],
            [88,68],[75,70],[62,70],[48,69],[38,67]], LIGHT)
    p.line([[43,68],[55,69],[68,70],[78,69]], IVORY)
    p.line([[27,60],[31,61],[36,62]], FUR_DARK)
    p.line([[97,63],[101,60],[104,57]], FUR)


def crystal_fish(p):
    # A long carved herring sits partially behind the cat.  Its curved back,
    # tail root, belly and full cheek are one continuous silhouette.
    p.path([[109,22],[117,24],[133,22],[154,18],[174,17],
            [181,14],[188,17],[211,18],[235,21],[253,25],
            [262,29],[267,35],[268,42],[268,53],[264,55],
            [258,56],[237,56],[214,58],[190,58],[167,56],
            [146,52],[132,48],[126,50],[116,58],[111,58],
            [116,50],[110,45],[108,35]], INK)
    p.path([[111,24],[118,26],[134,24],[154,20],[174,19],
            [182,16],[187,19],[211,20],[235,23],[252,27],
            [260,31],[265,36],[266,42],[267,49],[267,53],
            [257,54],[236,54],[214,56],[190,56],[168,54],
            [147,50],[132,46],[125,48],[115,55],[119,49],
            [112,44],[110,35]], TEAL)
    p.path([[111,27],[121,28],[137,25],[156,22],[175,21],
            [189,21],[211,22],[234,25],[251,29],[260,33],
            [264,37],[265,43],[267,49],[267,53],[257,53],[236,52],
            [214,54],[190,54],[170,52],[149,48],[131,44],
            [123,47],[112,48],[110,42],[110,33]], ICE)
    # A wide light-bearing crescent explains the crystal's curved face.  It
    # follows the back rather than ending in a rectangular text-field patch.
    p.path([[119,26],[137,22],[156,20],[175,19],[184,19],
            [174,21],[157,22],[140,25],[128,27]], CYAN)
    p.line([[127,24],[144,21],[158,20]], SHINE)
    p.path([[143,25],[156,23],[174,22],[183,22],[173,23],
            [157,25]], SHINE)
    p.path([[190,20],[210,21],[232,24],[250,28],[258,32],
            [247,30],[230,27],[211,24],[195,23]], CYAN)
    p.line([[196,21],[211,22],[224,24]], ICE)
    # Broad underside facet is kept BELOW the two mono/stereo indicator rows.
    # The pale head extends through y53 so neither label hangs off the fish.
    p.path([[145,50],[168,54],[191,56],[214,56],[237,54],
            [257,54],[267,54],[264,55],[258,56],
            [237,56],[214,58],[191,58],[168,56],[152,53]], MID)
    p.path([[156,50],[174,53],[191,54],[214,54],[235,52],
            [248,52],[236,54],[214,56],[190,56],[172,54]], CYAN)
    p.line([[182,56],[198,57],[215,56],[231,55]], ICE)
    p.line([[263,33],[265,36],[266,40]], CYAN)
    p.line([[264,35],[265,38]], SHINE)
    # The forked tail's lower lobe connects to its root behind the left well.
    p.path([[119,47],[127,45],[131,46],[125,49],[116,56],
            [119,51]], CYAN)
    p.line([[115,54],[122,49],[127,47]], SHINE)
    p.path([[175,19],[181,15],[186,18],[182,18],[180,17]], CYAN)
    p.line([[178,17],[181,15]], SHINE)
    # Small engraved head details use the free band between the live rows.
    p.path([[246,35],[244,39],[240,41],[238,42],[243,42],
            [247,39],[248,36]], CYAN)
    p.line([[247,35],[247,38],[244,41]], MID)
    p.line([[244,35],[243,38],[240,40]], SHINE)
    p.stamp(256,35, [' iii ', 'iccli', 'icdsi', ' tti '],
            {'i':MID,'c':CYAN,'l':SHINE,'d':INK,'s':INK,'t':TEAL})
    p.line([[264,40],[267,39]], INK)
    a=Art();a.glyph('KBPS',132,41,MID);a.glyph('KHZ',175,41,MID)
    p.ops += a.ops


def build():
    p=CanvasPen()
    # Rebuild the inherited static ground inside our ownership, so all old
    # chin/fin fragments disappear even where new art leaves negative space.
    background(p)
    crystal_fish(p)
    silver_cat(p)
    out=p.parts([14,14,255,96], titles='none', preserve_runtime=False)
    # Ear tips need both native focus states.  This strip intentionally stops
    # before the title text and never touches window controls.
    out += p.parts([30,2,83,12], titles='both', preserve_runtime=False)
    return out


if __name__ == '__main__':
    parts=build()
    print(len(parts), 'parts;', sum(len(p['operations']) for p in parts), 'native marks')
