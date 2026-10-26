# Selected-file restart check

Use a disposable test emulator with the generated review fixture
`Cranamp playback check.wav` in Downloads. Install the store-feature debug APK
and its instrumentation APK. The picker test expects the full portrait player
with all three windows visible. It releases only an existing read grant for the
named fixture before selecting it again through the real system picker.

Run the selection check, stop the app, then run the read check in a new process:

```sh
adb -s emulator-5554 shell am instrument -w -e class com.cranamp.app.FileAccessPersistenceTest#pickingAudioRetainsReadAccess com.cranamp.app.debug.test/androidx.test.runner.AndroidJUnitRunner
adb -s emulator-5554 shell am force-stop com.cranamp.app.debug
adb -s emulator-5554 shell am instrument -w -e class com.cranamp.app.FileAccessPersistenceTest#selectedFileIsReadableAfterProcessRestart com.cranamp.app.debug.test/androidx.test.runner.AndroidJUnitRunner
```

Both commands must report `OK (1 test)`. The second check opens the original
provider document through ContentResolver and reads one byte. It creates no
copy of the audio file.
