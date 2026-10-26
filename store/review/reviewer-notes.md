# Reviewer instructions for the replacement build

Draft only. Attach the generated playback sample and use the final tested version
and build numbers when replacing the store submissions.

Cranamp opens in the music player with an empty playlist. No account, purchase or
subscription is needed. Cranamp does not supply a music catalogue.

To test playback, press the eject button to the right of the playback buttons in
the top window. Choose the attached `Cranamp playback check.wav` in the system
file picker. Selecting the file starts playback. Pause, Play and Stop are in the
same row. On Android, playback continues on the home screen and the media
notification offers pause and resume. Stop in the player ends the foreground
service. The sample is a generated two-minute tone, with no commercial recording.

To test a folder, open the plus button (ADD) at the bottom-left of the playlist
and choose ADD FOLDER. The app starts as tracks are found. It keeps file references
and reads selected audio from its original location. It does not copy the library
or convert whole tracks. The selected storage provider must remain available.

To open Skin Studio, press the lower-right corner of the top player window to
open Settings. Choose Open Skin Studio, then Open working copy. Canvas has the
drawing tools. Skin has playlist and visualizer colours, bitmap font generation,
window cutouts and cursor artwork. Save and share exports CSTUDIO projects or
WSZ skins. Imported skins and projects contain images and settings; they do not
execute downloaded code.

The first screenshot in each gallery shows the player. The other screenshots show
Studio after these steps. The store edition has no self-update interface or
installer capability. Updates come from the store.

## Desktop addition

MCP is available only in the desktop editor. In Studio, open Agent connection from
the top toolbar, or Project actions in a narrow window. Start server and Test
connection exercise the local service. No separate agent account is needed to
test these controls. The service binds to 127.0.0.1, stays off between sessions
and stops when Studio closes. It can edit the active skin and use files within
the app's permissions. The interface explains that a connected agent may send
the data it reads to its own provider. Cranamp does not call an AI provider itself.

The Mac App Store build uses App Sandbox with access to user-selected files.
Incoming network access supports the user-started local MCP service.

## iOS addition

The iOS build has no MCP server. HLS streams use system playback, so HLS does not
support the equalizer, balance or visualizer. Those controls work with supported
local files and direct audio streams.
