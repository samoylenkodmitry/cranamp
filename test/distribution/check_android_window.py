"""Check the merged Activity contract used by player and Studio in DeX."""
from pathlib import Path
import sys
import xml.etree.ElementTree as ET

ANDROID = '{http://schemas.android.com/apk/res/android}'

def verify(manifest, styles):
    root = ET.parse(manifest).getroot()
    activity = next(node for node in root.iter('activity')
                    if node.get(ANDROID + 'name') == 'dev.cranpose.android.CranposeActivity')
    assert activity.get(ANDROID + 'resizeableActivity') == 'true'
    assert activity.get(ANDROID + 'theme') == '@style/CranampActivity'
    assert activity.get(ANDROID + 'screenOrientation') is None
    config = activity.get(ANDROID + 'configChanges', '').split('|')
    assert {'screenSize', 'smallestScreenSize', 'screenLayout'} <= set(config)
    style = next(node for node in ET.parse(styles).getroot().iter('style')
                 if node.get('name') == 'CranampActivity')
    values = {item.get('name'): item.text for item in style.iter('item')}
    assert values['android:windowFullscreen'] == 'false'
    assert values['android:windowIsFloating'] == 'false'
    assert values['android:windowSoftInputMode'] == 'adjustResize'
    print('Resizable Activity with non-fullscreen theme and keyboard resize: passed')

if __name__ == '__main__':
    verify(Path(sys.argv[1]), Path(sys.argv[2]))
