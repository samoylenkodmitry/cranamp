# Windows rendering checks

Run the native smoke test in the signed-in Windows desktop session. The script
starts only the specified executable, captures its largest window, rejects a
blank or undersized player, and stops that process afterward. `-Schedule`
creates the `CranampRenderingAudit` scheduled task to reach the desktop from SSH.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File test/windows-rendering.ps1 -Executable C:\path\cranamp.exe -OutputDirectory C:\path\audit -Schedule
```

Read `audit.log` and `failure.txt` in the output directory; inspect `window.png`.
The process, bounds, and stdout/stderr are saved there as well. To remove the
test task, run `Unregister-ScheduledTask CranampRenderingAudit -Confirm:$false`.

For Firefox, start Mozilla's geckodriver on port 4444 in that desktop session.
When driving it from another host over SSH, forward WebDriver and WebDriver BiDi:

```sh
ssh -N -L 4444:127.0.0.1:4444 -L 9222:127.0.0.1:9222 samoy@192.168.50.28
CRANAMP_CHECK_PLAYBACK=1 node test/windows-firefox.mjs https://example.test/cranamp/ /tmp/cranamp-firefox
```

The browser check uses a fresh Firefox profile. It waits for renderer startup
and two animation frames, saves a screenshot and console events, and fails on
startup errors or a 30-second stall. The optional playback check clicks Play at
the default Catamp layout's position and requires the audio time to advance.
The page must serve the bundled `demo-music/` directory. Set
`CRANAMP_SHADER_TRACE=1` to save the GLSL submitted to WebGL in `events.json`.

For Chrome, launch a separate desktop instance with
`--remote-debugging-port=9223 --user-data-dir=C:\path\audit-chrome-profile`
and forward port 9223 over SSH. The separate profile keeps the test isolated
from the user's browsing session. Then run:

```sh
CRANAMP_CHECK_PLAYBACK=1 node test/windows-chrome.mjs http://127.0.0.1:8765/ /tmp/cranamp-chrome
```

The Chrome check saves console events and a screenshot, waits for renderer
startup and animation frames, and optionally verifies playback. Serve the
built web package at the supplied URL; its assets and demo music must be present.

Windows 11 / RTX 2070 validation:

- The published 0.1.72 executable failed the blank-window check.
- A corrected `dist-min` build with the release's nightly/build-std options
  passed and showed the complete player.
- Windows Firefox 143 rendered the fixed player and started a demo track in
  1.5 seconds. The track resolved to the page's HTTP origin, rather than
  `file:///demo-music/…`.
- Windows Chrome 153 rendered the fixed player and started a demo track in
  1.4 seconds. Both Chrome and Firefox stalled before renderer initialization
  on the published site using Cranpose 0.1.140 during the same audit.

The native release must preserve Debug formatting: Naga uses it to generate
HLSL numeric literals. `test/release_workflow.mjs` guards the build flags.
