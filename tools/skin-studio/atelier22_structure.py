"""Hand-authored continuous crystal case. Native source mapping; no raster filters."""
from connected_canvas import CanvasPen
from canvas_bridge import bridge
from atelier22_common import LetterPen as Art
C=dict(ink='#102b40',frame='#193e55',well='#21465c',mid='#32627b',teal='#508ea3',cyan='#8cc5d2',ice='#c7e8e7',shine='#f1f5e6')
LEFT=['ink','mid','cyan','ice','teal','frame','mid','teal','cyan','mid','well','well']
RIGHT=['frame','mid','teal','cyan','mid','ink']

def walls(p,y,h):
    for x,c in enumerate(LEFT):p.rect(x,y,1,h,C[c])
    for x,c in enumerate(RIGHT,269):p.rect(x,y,1,h,C[c])

def footer_buttons(p):
    from atelier22_controls import crystal
    for x,mark in [(12,'+'),(41,'-'),(70,'V'),(99,'M'),(230,'L')]:
        q=crystal(24,18)
        # Transparent/key backgrounds belong to controls, never cut holes in case.
        q.ops=[o for o in q.ops if o.get('color')!='#ff00ff']
        p.place(q,x,346)
        if mark in '+-':
            a=Art();a.glyph(mark,x+10,353,C['ink']);p.ops.extend(a.ops)
        elif mark=='V':p.line([[x+7,354],[x+10,357],[x+16,351]],C['ink'],2)
        elif mark=='M':
            for xx in [x+6,x+11,x+16]:p.rect(xx,354,2,2,C['ink'])
        else:
            for yy in [351,354,357]:p.rect(x+7,yy,10,1,C['ink'])

def build():
    p=CanvasPen()
    # Remove inherited fish fragments outside the new illustration footprint.
    p.rect(154,9,37,5,C['frame']);p.rect(265,14,4,101,C['frame'])
    p.rect(12,116,257,116,C['frame'])
    # One clear, grounded response-glass seat. Cats rest on the upper lit edge.
    p.round(83,130,120,25,4,C['ink'])
    p.round(84,131,118,23,3,C['mid'])
    p.line([[86,132],[199,132],[201,134]],C['ice'])
    p.line([[85,134],[85,149],[88,153],[197,153],[201,150]],C['teal'])
    p.line([[89,154],[197,154]],C['ink'])
    # A single physical slider chamber with straight seats for all 11 grooves.
    p.round(14,154,246,66,7,C['ink'])
    p.round(15,155,244,64,6,C['mid'])
    p.round(17,157,240,60,5,C['well'])
    p.path([[21,158],[252,158],[255,161],[255,166],[19,164],[18,162]],'#285268')
    p.line([[19,160],[22,157],[251,157],[255,161]],C['teal'])
    p.line([[23,156],[95,156]],C['cyan'])
    p.line([[18,163],[18,209],[21,213]],C['mid'])
    p.line([[22,216],[251,216],[256,211]],C['mid'])
    p.line([[24,217],[248,217]],C['teal'])
    # Frequency marks sit in one recessed engraving band, with a defined floor.
    p.rect(18,221,239,9,C['frame'])
    a=Art()
    for word,x in [('PRE',23),('60',81),('170',97),('310',115),('600',133),('1K',153),('3K',171),('6K',189),('12K',205),('14K',223),('16K',241)]:
        a.glyph(word,x,223,C['cyan'])
    p.ops.extend(a.ops)
    # Playlist brow is connected to the walls; its shared fill is supplied below.
    p.rect(0,232,275,20,C['frame']);p.rect(12,248,243,4,C['well'])
    p.line([[11,244],[258,244]],C['mid'])
    p.line([[90,244],[177,244]],C['teal'])
    p.line([[90,240],[95,244],[90,248]],C['teal'])
    for x,h in [(105,3),(117,4),(129,4),(141,4),(153,3),(165,3)]:
        p.line([[x-2,244-h],[x,244],[x-2,244+h]],C['teal'])
    p.path([[173,242],[177,240],[181,240],[185,243],[185,245],[181,247],[177,247],[173,245]],C['teal'])
    p.pixel(181,242,C['ice']);p.pixel(184,245,C['frame'])
    p.line([[256,238],[260,238]],C['cyan'])
    p.stamp(265,236,['x x',' x ','x x'],{'x':C['teal']})
    # Sidewall reflection is one cross-section across main/EQ/header/rail/footer.
    walls(p,12,352)
    # Right playlist gutter includes a deeply inset scroll channel; exact tile
    # edges repeat and the footer cap is drawn from that identical cross-section.
    for x,c in [(255,'well'),(256,'well'),(257,'well'),(258,'mid'),(259,'teal'),(260,'mid'),(261,'ink'),(262,'ink'),(263,'ink'),(264,'ink'),(265,'ink'),(266,'ink'),(267,'teal'),(268,'frame')]:
        p.rect(x,248,1,96,C[c])
    p.line([[260,248],[262,246],[265,246],[267,248]],C['teal'])
    # Footer interior shares the chamber's dark glass, with one status console.
    p.rect(12,339,243,38,C['well'])
    p.round(128,343,99,31,4,C['ink'])
    p.round(129,344,97,29,3,C['mid'])
    p.round(130,345,95,27,2,C['well'])
    p.round(132,348,73,10,2,C['cyan'])
    p.rect(133,349,71,8,'#afced2')
    p.line([[133,347],[199,347]],C['ice'])
    p.line([[133,359],[204,359]],C['ink'])
    p.rect(192,363,30,8,'#afced2')
    p.line([[192,362],[221,362]],C['teal'])
    p.path([[145,366],[141,368],[145,370]],C['ice']);p.rect(139,366,1,5,C['ice'])
    p.path([[150,366],[154,368],[150,370]],C['ice'])
    p.rect(158,366,2,5,C['ice']);p.rect(162,366,2,5,C['ice'])
    p.rect(168,366,5,5,C['ice'])
    p.path([[176,366],[180,368],[176,370]],C['ice']);p.rect(182,366,1,5,C['ice'])
    p.path([[185,369],[188,366],[190,369]],C['ice']);p.rect(185,371,6,1,C['ice'])
    footer_buttons(p)
    # Close the scroll channel with a shaped U, rather than cutting it at fy.
    p.path([[258,339],[268,339],[268,345],[265,349],[261,349],[258,345]],C['mid'])
    p.path([[260,339],[267,339],[267,344],[264,347],[262,347],[260,344]],C['ink'])
    p.line([[259,339],[259,344],[262,348],[265,348],[267,345],[267,339]],C['teal'])
    p.rect(260,339,1,5,C['mid']);p.rect(268,339,1,5,C['frame'])
    # Match all native columns right up to the return into the lower perimeter.
    walls(p,339,25)
    p.path([[5,363],[11,363],[13,369],[18,372],[258,372],[265,369],[269,363],[269,376],[5,376]],C['mid'])
    p.line([[7,363],[8,368],[12,372],[17,374],[259,374],[265,371],[268,367],[268,363]],C['teal'])
    p.line([[8,363],[9,367],[13,370],[18,372],[257,372],[264,369],[267,365],[267,363]],C['cyan'])
    p.line([[9,363],[10,366],[14,369],[19,371],[92,371]],C['mid'])
    p.line([[16,371],[57,371]],C['ice'])
    p.line([[229,373],[259,373],[264,370]],C['ice'])
    # Original rounded bottom silhouette, with continuous outer glass rim.
    p.path([[0,368],[1,372],[5,376],[269,376],[273,372],[274,368],[274,376],[0,376]],'#ff00ff')
    for pts,c in [([[0,364],[0,368],[2,372],[6,376],[268,376],[272,372],[274,368],[274,364]],'ink'),
                  ([[1,364],[1,368],[3,372],[7,375],[267,375],[271,372],[273,368],[273,364]],'mid'),
                  ([[2,364],[2,368],[4,372],[8,374],[266,374],[270,371],[272,368],[272,364]],'cyan'),
                  ([[3,364],[3,368],[5,371],[9,373],[265,373],[269,370],[271,368],[271,364]],'ice'),
                  ([[4,364],[4,368],[6,370],[10,372],[264,372],[268,369],[270,368],[270,364]],'teal')]:p.line(pts,C[c])
    for x,c in enumerate(RIGHT,269):p.rect(x,363,1,5,C[c])
    out=p.parts([0,0,275,377],repeats='skip',preserve_runtime=False)
    # Explicit native shared sources, no omissions hidden by a big ownership box.
    q=CanvasPen();walls(q,252,29)
    q.ops += [o for o in p.ops if o.get('op')=='rect' and o.get('y')==248]
    out+=bridge(q.ops,[{'sheet':'pledit.bmp','rect':[0,42,12,29],'destination':[0,252]},
                       {'sheet':'pledit.bmp','rect':[31,42,20,29],'destination':[255,252]}])
    q=CanvasPen();q.rect(0,0,25,20,C['frame']);q.rect(0,16,25,4,C['well']);q.line([[0,12],[24,12]],C['mid'])
    out+=bridge(q.ops,[{'sheet':'pledit.bmp','rect':[127,y,25,20],'destination':[0,0]} for y in [0,21]])
    # A darker translucent-looking response face in exact palette bands.
    q=CanvasPen();q.bands(0,0,113,19,['#91bdc7','#82b1c0','#75a6b7'],[0,5,13,19])
    q.line([[0,0],[112,0]],C['ice']);q.line([[0,1],[0,17]],C['cyan'])
    q.line([[1,18],[112,18]],C['mid']);q.line([[112,1],[112,17]],C['teal'])
    for x in [9,27,45,63,81,99]:q.pixel(x,16,'#6395aa')
    out+=q.control_parts('eq_graph',state='normal')
    q=CanvasPen();q.rect(0,0,113,1,'#ff00ff');out+=q.control_parts('eq_preamp',state='normal')
    return out
