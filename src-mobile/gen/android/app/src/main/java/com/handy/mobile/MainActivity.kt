package com.handy.mobile

import android.Manifest
import android.content.pm.PackageManager
import android.os.Bundle
import androidx.activity.enableEdgeToEdge
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat

class MainActivity : TauriActivity() {
  companion object {
    private const val REQ_RECORD_AUDIO = 1001
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    // Request RECORD_AUDIO at startup. Without runtime grant, Oboe's
    // openStream returns AAUDIO_ERROR_INTERNAL on Android 6+ even though
    // the permission is declared in AndroidManifest.xml. We don't block on
    // the result here — start_recording will fail gracefully if the user
    // denies, and the LogPanel will show the Oboe error.
    if (ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO)
        != PackageManager.PERMISSION_GRANTED) {
      ActivityCompat.requestPermissions(
        this,
        arrayOf(Manifest.permission.RECORD_AUDIO),
        REQ_RECORD_AUDIO,
      )
    }
  }
}
