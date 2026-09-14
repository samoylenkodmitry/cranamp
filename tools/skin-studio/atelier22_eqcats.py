"""Silverplay 22's two small silver tabbies, authored with native Studio Pen.

Pure builder: background clearing belongs to the preceding EQ frame plane.
Sleeping paws end at the graph ledge, y132; the hanging tail is to its right.
The runtime graph [86,133,113,19] stays excluded by CanvasPen's native masks.
"""
from connected_canvas import CanvasPen

P = dict(ink='#102b40', dark='#687889', mid='#9da8ad', light='#d4d6ca',
         ivory='#f1ebd5', pink='#b98190', well='#21465c', teal='#508ea3',
         cyan='#8cc5d2', ice='#c7e8e7')


def sleeping(p):
    def s(points, c): p.path(points, P[c], fill=True)
    def l(points, c): p.line(points, P[c])
    # Tail grows from the right rump. Its broad outer contour descends beside
    # the graph, then hooks left into a short, fully separated rounded tip.
    s([[198,124],[204,124],[209,127],[212,131],[214,136],[214,141],
       [212,145],[208,146],[205,145],[204,142],[205,139],[208,137],
       [209,139],[207,141],[208,143],[210,143],[211,140],[210,135],
       [207,131],[202,129],[198,129]],'ink')
    s([[200,125],[204,126],[208,129],[211,133],[212,137],[212,141],
       [210,144],[207,144],[205,142],[207,139],[207,141],[208,142],
       [210,141],[210,137],[208,133],[204,130],[200,128]],'mid')
    l([[204,127],[208,130],[210,134],[211,138]],'light')
    s([[210,132],[212,134],[212,136],[210,135]],'dark')
    s([[208,142],[211,142],[210,144],[207,144]],'dark')
    p.pixel(206,141,P['light'])
    # Low relaxed back, with a folded haunch and a flat weight-bearing belly.
    s([[162,129],[166,124],[171,120],[177,118],[188,118],[196,120],
       [201,123],[204,127],[203,130],[199,132],[170,132],[163,131]],'ink')
    s([[164,129],[168,124],[173,121],[178,119],[187,119],[195,121],
       [200,124],[202,127],[201,130],[197,131],[169,131]],'mid')
    s([[167,126],[172,122],[179,120],[186,120],[194,122],[198,124],
       [190,122],[183,122],[177,123],[170,127]],'light')
    l([[174,121],[179,120],[184,120]],'ivory')
    # Saddle stripes branch into broad connected shadows, following the back.
    s([[178,119],[181,119],[180,122],[178,124],[177,127],[174,128],
       [175,125],[178,122]],'dark')
    s([[186,119],[189,120],[188,123],[186,125],[185,128],[182,128],
       [184,124],[186,122]],'dark')
    s([[194,121],[197,122],[195,124],[194,126],[191,127],[192,124]],'dark')
    # The hind knee folds over the belly, with a single open contour and a
    # pale knee cap; no isolated circular medallion in the flank.
    s([[192,124],[197,124],[201,127],[200,130],[197,131],[191,130],
       [188,128],[189,125]],'mid')
    s([[193,124],[197,125],[199,127],[198,129],[194,129],[191,127]],'light')
    l([[190,125],[188,127],[190,130],[194,131]],'dark')
    s([[198,129],[201,128],[201,130],[198,131],[195,131]],'dark')
    # Two forepaws lie along the ledge. A short dark division distinguishes
    # the far paw from the longer near paw without cutting through the face.
    s([[169,126],[174,127],[177,130],[174,132],[164,132],[160,131],
       [164,130]],'dark')
    s([[169,127],[173,128],[175,130],[173,131],[165,131],[164,130]],'light')
    p.rect(170,131,3,1,P['ivory'])
    s([[162,128],[167,129],[170,131],[168,132],[154,132],[151,131],
       [155,129]],'dark')
    s([[159,129],[165,130],[167,131],[154,131],[154,130]],'light')
    p.rect(156,131,5,1,P['ivory'])
    p.pixel(164,131,P['mid'])
    # A polygon's bottom edge can end above this row in native scan fill.
    # Explicit contact pixels keep both forepaws grounded on the graph ledge.
    p.rect(154,132,21,1,P['dark'])
    # Head rests forward on its forepaws. The ear roots are visible above
    # a broad cheek; the chin ends at y131 instead of entering the live graph.
    s([[145,117],[151,121],[156,121],[163,118],[163,124],[166,126],
       [165,129],[161,131],[151,131],[146,129],[143,126],[145,122]],'ink')
    s([[146,119],[151,123],[156,122],[162,120],[161,125],[164,127],
       [161,130],[152,130],[148,129],[145,126],[147,123]],'mid')
    s([[146,120],[150,123],[148,126],[146,124]],'dark')
    s([[147,122],[149,123],[148,125]],'pink')
    l([[145,120],[145,123]],'light')
    s([[158,123],[161,121],[161,125],[159,126]],'dark')
    p.pixel(160,123,P['pink'])
    s([[151,123],[155,123],[158,125],[157,127],[152,128],[149,126]],'light')
    p.rect(152,123,2,1,P['ivory'])
    s([[151,122],[153,123],[154,125],[152,125]],'dark')
    l([[157,123],[156,125]],'dark')
    # Closed lids are shallow downward arcs, independent of the cheek marks.
    l([[147,126],[149,127],[151,126]],'ink')
    l([[157,126],[159,127],[161,126]],'ink')
    s([[150,128],[152,127],[154,128],[154,130],[151,130],[148,128]],'ivory')
    s([[155,128],[159,128],[160,129],[157,130],[154,130]],'light')
    p.pixel(154,128,P['pink']); p.pixel(154,129,P['dark'])
    l([[146,128],[149,129]],'dark')
    l([[161,128],[163,127]],'dark')
    l([[149,129],[144,128]],'mid')
    l([[159,129],[165,128]],'mid')


def seated(p):
    def s(points, c): p.path(points, P[c], fill=True)
    def l(points, c): p.line(points, P[c])
    # The seated torso has a narrow shoulder, a broad folded right haunch,
    # and two weight-bearing forelegs. Background remains visible at the toes.
    s([[54,184],[63,184],[66,189],[67,195],[72,200],[75,205],
       [75,211],[71,215],[64,216],[49,215],[45,211],[46,205],
       [50,198],[51,191]],'ink')
    s([[55,185],[62,186],[64,190],[65,196],[70,201],[73,206],
       [73,210],[70,213],[64,214],[50,214],[47,210],[48,205],
       [52,198],[53,191]],'mid')
    s([[54,189],[58,188],[58,193],[55,199],[51,204],[48,207],
       [49,202],[52,196]],'light')
    s([[54,191],[56,190],[55,194],[53,197],[51,198]],'ivory')
    s([[62,189],[65,193],[66,198],[69,201],[66,203],[63,198]],'dark')
    s([[52,198],[56,195],[57,197],[54,200],[50,202]],'dark')
    s([[48,205],[51,202],[54,202],[53,205],[50,207],[47,209]],'dark')
    # The rounded knee rolls outward from the hip. Stepped plane boundaries
    # describe the bend; short tapered stripes remain attached to the flank.
    s([[65,199],[69,202],[72,206],[72,210],[69,213],[64,213],
       [60,210],[59,206],[61,202]],'light')
    s([[67,202],[70,205],[70,209],[67,211],[64,211],[62,209],
       [63,210],[66,209],[68,207]],'mid')
    s([[71,207],[73,207],[73,210],[70,213],[67,214],[64,213],
       [68,212],[70,210]],'dark')
    s([[65,200],[67,202],[67,205],[64,207],[64,204]],'ivory')
    # Far leg: a modest silver column and its own right-facing paw. The cool
    # seam is interrupted at the wrist so it reads as anatomy rather than trim.
    s([[61,191],[65,192],[65,199],[63,205],[64,211],[66,214],
       [70,215],[70,217],[62,217],[60,214],[60,204]],'ink')
    s([[62,193],[64,194],[64,199],[62,206],[63,211],[65,215],
       [68,215],[69,216],[63,216],[61,212],[61,204]],'mid')
    s([[62,201],[62,207],[64,213],[66,215],[64,215],[62,212],
       [61,207]],'light')
    p.pixel(66,216,P['dark'])
    p.rect(63,215,3,1,P['light'])
    # Near breast and leg descend in one curved volume; its paw is lower and
    # left of the far paw, with a one-pixel well-colored gap at the ground.
    s([[54,185],[59,186],[62,189],[61,194],[59,200],[59,206],
       [60,212],[62,215],[62,218],[52,218],[51,216],[54,213],
       [54,205],[52,199],[50,194],[51,189]],'dark')
    s([[54,187],[58,187],[60,190],[59,194],[57,199],[57,205],
       [58,211],[60,215],[60,217],[53,217],[53,216],[56,214],
       [56,206],[54,200],[52,194],[52,190]],'light')
    s([[54,188],[56,188],[57,190],[55,194],[55,197],[53,195],
       [52,192]],'ivory')
    s([[58,190],[60,190],[60,193],[58,198],[57,202],[56,199]],'mid')
    s([[56,207],[57,211],[59,214],[59,216],[56,216],[56,214]],'ivory')
    p.pixel(56,217,P['mid']); p.pixel(59,217,P['mid'])
    p.pixel(61,218,P['well']); p.pixel(62,218,P['well'])
    # Tail curls outside the silhouette, distinct from both legs and stopping
    # before the nearer paw. The terminal bend remains a silver fur cluster.
    s([[47,203],[44,203],[41,205],[39,208],[40,212],[43,215],
       [47,217],[51,217],[52,215],[49,214],[45,213],[43,210],
       [44,208],[47,208]],'ink')
    s([[45,204],[42,206],[41,209],[42,212],[46,215],[50,216],
       [50,215],[46,213],[43,210],[43,208],[46,207]],'mid')
    l([[43,206],[42,209],[43,212],[45,213]],'light')
    s([[41,210],[43,211],[44,213],[42,212]],'dark')
    s([[46,214],[47,214],[48,216],[46,215]],'dark')
    p.pixel(50,215,P['light'])
    # Broad adult head, a short muzzle, and two distinct triangular ears.
    s([[49,164],[55,168],[60,168],[68,164],[68,172],[71,176],
       [70,181],[67,184],[62,188],[56,187],[51,184],[48,181],
       [49,177],[48,174],[50,171]],'ink')
    s([[50,166],[55,170],[60,170],[67,166],[66,173],[69,176],
       [68,180],[65,184],[61,186],[56,185],[52,182],[50,180],
       [51,177],[50,174],[52,172]],'mid')
    s([[51,168],[55,171],[54,175],[51,173]],'dark')
    s([[51,169],[54,172],[53,174],[52,172]],'pink')
    l([[50,167],[50,170],[51,173]],'light')
    s([[62,171],[66,168],[65,174],[63,175]],'dark')
    l([[64,171],[65,170],[64,173]],'pink')
    s([[55,171],[60,171],[63,173],[62,176],[58,178],[53,177],
       [52,175]],'light')
    s([[54,172],[56,171],[56,173],[54,175]],'ivory')
    s([[56,171],[58,171],[58,174],[56,176],[55,175]],'dark')
    s([[61,172],[63,173],[62,175],[60,176]],'dark')
    s([[66,174],[69,177],[68,180],[66,182],[64,181],[65,178]],'dark')
    # Eyes are calm, slim cyan irises under ink lids, with a tiny dark pupil.
    l([[51,176],[54,175],[57,176]],'ink')
    p.pixel(53,176,P['cyan']); p.pixel(54,176,P['ink'])
    l([[61,176],[63,176],[65,177]],'ink')
    p.pixel(62,177,P['cyan']); p.pixel(63,177,P['ink'])
    s([[50,179],[53,179],[55,182],[52,181]],'dark')
    s([[65,179],[69,178],[68,180],[65,182]],'mid')
    # Interlocking cheeks and short chin; mouth is one quiet shadow pixel.
    s([[54,180],[57,178],[59,180],[62,180],[63,182],[60,185],
       [56,184],[53,182]],'light')
    s([[54,180],[57,179],[58,180],[57,182],[54,182]],'ivory')
    s([[60,180],[62,181],[62,183],[60,184],[59,182]],'mid')
    p.pixel(58,179,P['pink']); p.pixel(59,179,P['dark'])
    p.pixel(59,180,P['ink']); p.pixel(58,183,P['dark'])
    l([[53,181],[49,180],[47,180]],'mid')
    l([[54,182],[51,184]],'light')
    l([[63,181],[68,180],[71,180]],'mid')
    # Small fabric collar follows the neck turn, with a single metal glint.
    l([[55,185],[59,187],[64,185]],'teal')
    l([[56,185],[60,186],[63,185]],'cyan')
    p.pixel(60,187,P['ivory'])


def build():
    p = CanvasPen()
    sleeping(p)
    parts = p.parts([141,117,79,30], titles='both')
    p = CanvasPen()
    seated(p)
    return parts + p.parts([39,164,40,55], titles='none')
