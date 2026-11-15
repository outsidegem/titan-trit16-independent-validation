#!/usr/bin/env bash
set -euo pipefail

echo "Waiting for Android emulator system services..."
adb wait-for-device

for attempt in $(seq 1 90); do
  boot_completed="$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r' || true)"

  if [ "$boot_completed" = "1" ] &&
     adb shell pm list packages >/dev/null 2>&1 &&
     adb shell am get-current-user >/dev/null 2>&1; then
    echo "Android emulator is fully ready (attempt $attempt)."
    adb shell settings put global window_animation_scale 0 || true
    adb shell settings put global transition_animation_scale 0 || true
    adb shell settings put global animator_duration_scale 0 || true
    exit 0
  fi

  echo "Android services not ready ($attempt/90). Waiting 5 seconds..."
  sleep 5
done

echo "ERROR: Android emulator services failed to initialize." >&2
adb shell getprop sys.boot_completed >&2 || true
adb shell pm list packages >&2 || true
adb shell am get-current-user >&2 || true
adb logcat -d -t 300 >&2 || true
exit 1
