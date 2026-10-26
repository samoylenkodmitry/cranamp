# Background playback review evidence

The emulator test and recording were completed on October 26, 2026. The video
has not been published, and the declaration has not been saved or submitted.

Google's App content page requires a video for `FOREGROUND_SERVICE_MEDIA_PLAYBACK`.
Choose **Media playback**. Cranamp does not use picture in picture.
See [Google's foreground service requirements](https://support.google.com/googleplay/android-developer/answer/13392821).

Use a clean test device or emulator with the store build. Record the whole screen
continuously. Exclude personal notifications and private library names. The video
must show the real app. Do not substitute an animation or a Studio preview.

1. Open Cranamp with an empty playlist. Show that it does not start audio by itself.
2. Press the eject button beside the playback controls. Choose `Cranamp playback check.wav` through the system picker.
3. Selecting a file starts playback. Show the elapsed time advancing. For a track
   that is already selected and stopped, press Play to start it.
4. Go to the home screen. Open the notification shade and show Cranamp's media controls.
5. Pause and resume through those controls. Show the state change and advancing position.
6. Return to Cranamp and press Stop. Confirm that the foreground playback service ends.

Generate the sample without using private or commercial music:

```sh
python3 test/distribution/prepare_review_media.py '/tmp/Cranamp playback check.wav'
```

The sample is a quiet, generated 440 Hz tone, mono PCM, 16 kHz, two minutes.
It contains no recorded sound. Keep the volume low. It may also accompany Apple's
review notes so the reviewer has a known playable local file.

Declaration explanation supported by the recording:

> Cranamp uses the media playback foreground service when the user selects an
> audio file or presses Play.
> It keeps selected audio playing while the user opens another app or locks the
> screen. Android shows a media notification with playback controls. Deferring
> the service would delay the requested playback; interrupting it would cut off
> the audio. The user can pause from the notification and stop from the player.

Record the tested version, build number, device, artifact checksum and video
checksum with the evidence. Confirm the video link opens without an account
before using it in the declaration. Publication and store submission remain paused.

## Recorded evidence

Local directory: `/tmp/cranamp-release-0.1.91/audit/`.

- `playback-background.mp4`: continuous 77.24-second recording of Play, Home,
  notification pause/resume, return to the player and Stop. SHA-256:
  `c9c4263e5c8da9488844bd1d1d9fda1f2327e07fc4e5a1f9f23793034c0ac38e`.
- `playback-full-import.mp4`: earlier recording of the empty playlist, file picker
  and background controls. Its recording limit ended before Stop.
- `playback-receipt.json`: build identity, APK checksum and observed service states.
- The installed APK came from the uploaded 0.1.91 AAB, version code 10091. It was
  signed with the local debug key for installation on `emulator-5554`, API 36,
  arm64 with 16 KB pages. No production device was changed during this audit.
- Android reported paused position 18800 ms and resumed playback at 18980 ms.
  The foreground service was absent after Stop. The crash buffer was empty.

Android screenrecord records the screen without an audio track. The notification
state and elapsed position demonstrate playback. Review frames were inspected.
Repeat the recording on the replacement store build after the audit fixes land.
