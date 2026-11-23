#!/usr/bin/env bash
# Copyright 2014-2021 The winit contributors
# Copyright 2021-2023 Tauri Programme within The Commons Conservancy
# SPDX-License-Identifier: Apache-2.0

# End-to-end checks of how tao attaches windows to Android activities,
# in particular when `Event::ActivityRequested` must (and must not) be emitted.
#
# Drives the example on a connected device or emulator with adb and checks the `[e2e]` lines
# the example logs, plus the "tao window attached" label each activity shows.
#
# Usage: e2e/android.sh [--no-build] [scenario...]
#   --no-build  use the APK that is already installed
#   scenario    run only these scenarios (see `SCENARIOS` below)
#
# Set ARTIFACTS_DIR to save the full logcat and a screenshot of each failure there.

set -euo pipefail

cd "$(dirname "$0")/.."

PKG=app.tauri.tao_mobile
ACTIVITY="$PKG/.MainActivity"
TAG=tao-mobile
ADB=${ADB:-adb}
# how long to wait for events that must *not* happen
SETTLE_SECS=${SETTLE_SECS:-3}
ARTIFACTS_DIR=${ARTIFACTS_DIR:-}

SCENARIOS=(
  cold_launch
  rotation
  recreation
  new_window
  back_navigation
  finish_and_relaunch
  swipe_away_with_background_task
  swipe_away_without_background_task
  process_death
)

failures=0
current_scenario=

# --- output -------------------------------------------------------------------------------------

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
pass() { printf '  \033[32mok\033[0m   %s\n' "$*"; }
fail() {
  printf '  \033[31mFAIL\033[0m %s\n' "$*"
  failures=$((failures + 1))
  failed_scenarios+=("$current_scenario")
  if [[ -n $ARTIFACTS_DIR ]]; then
    $ADB exec-out screencap -p >"$ARTIFACTS_DIR/$failures-$current_scenario.png" || true
  fi
}
failed_scenarios=()

# --- device helpers -----------------------------------------------------------------------------

sh_() { $ADB shell "$@" | tr -d '\r'; }

app_pid() { sh_ pidof "$PKG" || true; }

# `logcat -c` does not reliably drop lines logged right before it (e.g. Android 11),
# so log a marker instead and only look at the lines after it
run_id="$$-$RANDOM"
mark=0
clear_logs() {
  mark=$((mark + 1))
  sh_ log -t tao-e2e "mark $run_id-$mark"
}

e2e_logs() {
  $ADB logcat -d -s "$TAG:I" tao-e2e:I |
    awk -v m="mark $run_id-$mark" 'index($0, "tao-e2e") && substr($0, length($0) - length(m) + 1) == m { f = 1; next } f; END { if (!f) print "[e2e] log-marker-missing" }' |
    grep -F '[e2e]' || true
}

# number of `[e2e] <event>` lines since the logs were cleared
# -1 when the logs since `clear_logs` are not available, so no expected count matches
count() {
  local logs
  logs=$(e2e_logs)
  if grep -qF '[e2e] log-marker-missing' <<<"$logs"; then
    echo -1
  else
    grep -c -- "\[e2e\] $1\b" <<<"$logs" || true
  fi
}

# activity IDs of the `[e2e] <event>` lines since the logs were cleared
ids() { e2e_logs | grep -- "\[e2e\] $1\b" | grep -oE 'activity=-?[0-9]+' | cut -d= -f2 || true; }

wait_for() {
  local event=$1 expected=${2:-1} timeout=${3:-10}
  local deadline=$((SECONDS + timeout))
  while ((SECONDS < deadline)); do
    if (($(count "$event") >= expected)); then return 0; fi
    sleep 0.5
  done
  fail "timed out waiting for $expected '$event' event(s)"
  return 1
}

wait_for_pid() {
  local want=$1 timeout=${2:-10}
  local deadline=$((SECONDS + timeout))
  while ((SECONDS < deadline)); do
    local pid
    pid=$(app_pid)
    if [[ $want == none && -z $pid ]] || [[ $want == some && -n $pid ]]; then return 0; fi
    sleep 0.5
  done
  fail "timed out waiting for the process to be $([[ $want == none ]] && echo dead || echo alive)"
  return 1
}

ui_dump() {
  sh_ uiautomator dump /sdcard/tao-e2e.xml >/dev/null 2>&1 || true
  sh_ cat /sdcard/tao-e2e.xml
}

tap_button() {
  local label=$1 bounds
  bounds=$(ui_dump | grep -oiE "text=\"$label\"[^>]*bounds=\"\[[0-9]+,[0-9]+\]\[[0-9]+,[0-9]+\]\"" |
    grep -oE '\[[0-9]+,[0-9]+\]\[[0-9]+,[0-9]+\]' | head -1)
  if [[ -z $bounds ]]; then
    fail "button '$label' not found"
    return 1
  fi
  local x1 y1 x2 y2
  read -r x1 y1 x2 y2 <<<"$(tr '[],' '   ' <<<"$bounds")"
  sh_ input tap $(((x1 + x2) / 2)) $(((y1 + y2) / 2))
}

launch_from_launcher() {
  sh_ monkey -p "$PKG" -c android.intent.category.LAUNCHER 1 >/dev/null 2>&1
}

swipe_away_from_recents() {
  sh_ input keyevent KEYCODE_APP_SWITCH
  sleep 2
  local size w h
  size=$(sh_ wm size | grep -oE '[0-9]+x[0-9]+' | tail -1)
  w=${size%x*}
  h=${size#*x}
  sh_ input swipe $((w / 2)) $((h / 2)) $((w / 2)) $((h / 20)) 200
  sleep 1
  sh_ input keyevent KEYCODE_HOME
}

# like the low memory killer: `am kill` only kills cached processes, which takes a moment after
# the app goes to the background, and `run-as` is not allowed to kill it on some versions
kill_in_background() {
  local deadline=$((SECONDS + 10)) pid
  while ((SECONDS < deadline)); do
    sh_ am kill "$PKG"
    sleep 1
    [[ -z $(app_pid) ]] && return 0
  done
  pid=$(app_pid)
  [[ -n $pid ]] && sh_ run-as "$PKG" kill -9 "$pid"
  wait_for_pid none
}

reset_app() {
  sh_ am force-stop "$PKG"
  clear_logs
}

# --- assertions ---------------------------------------------------------------------------------

expect_count() {
  local event=$1 expected=$2 actual
  actual=$(count "$event")
  if [[ $actual == "$expected" ]]; then
    pass "$event: $actual"
  else
    fail "$event: expected $expected, got $actual"
  fi
}

# every requested activity got a window
expect_requested_activities_attached() {
  local id
  for id in $(ids activity-requested); do
    if ids window-created | grep -qx -- "$id"; then
      pass "window created for requested activity $id"
    else
      fail "no window created for requested activity $id"
    fi
  done
}

# the foreground activity shows that a tao window is attached to it
expect_attached() {
  local deadline=$((SECONDS + 5)) xml
  while ((SECONDS < deadline)); do
    xml=$(ui_dump)
    if grep -q 'text="tao window attached"' <<<"$xml"; then
      pass "foreground activity is attached to a window"
      return
    fi
    sleep 0.5
  done
  if grep -q 'text="no tao window attached"' <<<"$xml"; then
    fail "foreground activity is not attached to a window"
  else
    fail "the example's activity is not in the foreground"
  fi
}

expect_same_pid() {
  local before=$1 after
  after=$(app_pid)
  if [[ -n $after && $after == "$before" ]]; then
    pass "process kept alive ($after)"
  else
    fail "expected process $before to be alive, got '${after:-none}'"
  fi
}

# --- scenarios ----------------------------------------------------------------------------------

# the activity that launched the app is attached to the window created on `StartCause::Init`
cold_launch() {
  reset_app
  launch_from_launcher
  wait_for window-created || return
  sleep "$SETTLE_SECS"
  expect_count activity-requested 0
  expect_count window-created 1
  expect_attached
}

# handled by the activity (`android:configChanges`), so it is not recreated
rotation() {
  reset_app
  launch_from_launcher
  wait_for window-created || return
  clear_logs
  sh_ settings put system accelerometer_rotation 0
  sh_ settings put system user_rotation 1
  sleep "$SETTLE_SECS"
  sh_ settings put system user_rotation 0
  sleep "$SETTLE_SECS"
  expect_count activity-requested 0
  expect_count activity-created 0
  expect_count window-closed 0
  expect_attached
}

# the activity is recreated with the same ID and must stay attached to its window
recreation() {
  reset_app
  launch_from_launcher
  wait_for window-created || return
  local attached
  attached=$(ids window-created | head -1)
  clear_logs
  sh_ cmd uimode night yes >/dev/null
  wait_for activity-created
  sleep "$SETTLE_SECS"
  sh_ cmd uimode night no >/dev/null
  wait_for activity-created 2
  sleep "$SETTLE_SECS"
  expect_count activity-requested 0
  expect_count window-created 0
  expect_count window-closed 0
  if [[ $(ids activity-created | sort -u) == "$attached" ]]; then
    pass "recreated activity kept its ID ($attached)"
  else
    fail "recreated activity changed its ID: $attached -> $(ids activity-created | xargs)"
  fi
  expect_attached
}

# `Window::new` with an activity name starts a new activity, which is not a requested one
new_window() {
  reset_app
  launch_from_launcher
  wait_for window-created || return
  clear_logs
  tap_button "New window" || return
  wait_for window-created || return
  sleep "$SETTLE_SECS"
  expect_count activity-requested 0
  expect_count activity-created 1
  if [[ $(ids activity-created) == "$(ids window-created)" ]]; then
    pass "window attached to the activity it created"
  else
    fail "window attached to $(ids window-created), created $(ids activity-created)"
  fi
  expect_attached
}

# back on a secondary activity destroys it. Back on the root activity either moves the task to the
# background (launcher activities on Android 12+) or destroys the activity too, depending on the
# device, so launching the app again either resumes it or makes the system create a new activity
back_navigation() {
  reset_app
  launch_from_launcher
  wait_for window-created || return
  tap_button "New window" || return
  wait_for window-created 2 || return
  clear_logs
  sh_ input keyevent KEYCODE_BACK
  wait_for window-closed || return
  sh_ input keyevent KEYCODE_BACK
  sleep "$SETTLE_SECS"
  local closed
  closed=$(count window-closed)
  launch_from_launcher
  sleep "$SETTLE_SECS"
  if ((closed == 1)); then
    pass "root activity moved to the background"
    expect_count activity-requested 0
  else
    pass "root activity destroyed"
    expect_count activity-requested 1
    expect_requested_activities_attached
  fi
  expect_attached
}

# all activities are destroyed while the process is alive (cached),
# so the system creates an activity that no window is attached to
finish_and_relaunch() {
  reset_app
  launch_from_launcher
  wait_for window-created || return
  local pid
  pid=$(app_pid)
  clear_logs
  tap_button "Finish activity" || return
  wait_for window-closed || return
  launch_from_launcher
  wait_for activity-requested || return
  sleep "$SETTLE_SECS"
  expect_same_pid "$pid"
  expect_count activity-requested 1
  expect_requested_activities_attached
  expect_attached
}

# a foreground service keeps the process alive after the app is removed from the recents screen
swipe_away_with_background_task() {
  reset_app
  sh_ am start -n "$ACTIVITY" --ez startBackgroundTask true >/dev/null
  wait_for window-created || return
  sleep 1
  if sh_ dumpsys activity services "$PKG" | grep -q BackgroundTaskService; then
    pass "background task running"
  else
    fail "background task not running"
    return
  fi
  local pid
  pid=$(app_pid)
  clear_logs
  swipe_away_from_recents
  wait_for window-closed || return
  expect_same_pid "$pid"
  launch_from_launcher
  wait_for activity-requested || return
  sleep "$SETTLE_SECS"
  expect_same_pid "$pid"
  expect_count activity-requested 1
  expect_requested_activities_attached
  expect_attached
}

# without a background task the process dies with its task, so this is a cold launch
swipe_away_without_background_task() {
  reset_app
  launch_from_launcher
  wait_for window-created || return
  swipe_away_from_recents
  wait_for_pid none || return
  clear_logs
  launch_from_launcher
  wait_for window-created || return
  sleep "$SETTLE_SECS"
  expect_count activity-requested 0
  expect_count window-created 1
  expect_attached
}

# the system kills the process in the background and restores the activity from its saved state
process_death() {
  reset_app
  launch_from_launcher
  wait_for window-created || return
  local before
  before=$(ids window-created | head -1)
  sh_ input keyevent KEYCODE_HOME
  wait_for state-saved || return
  kill_in_background || return
  clear_logs
  launch_from_launcher
  wait_for window-created || return
  sleep "$SETTLE_SECS"
  if [[ $(ids activity-created) == "$before" ]]; then
    pass "activity restored from its saved state ($before)"
  else
    fail "expected activity $before to be restored, got $(ids activity-created | xargs)"
  fi
  expect_count activity-requested 0
  expect_count window-created 1
  expect_attached
}

# --- main ---------------------------------------------------------------------------------------

build=1
selected=()
for arg in "$@"; do
  case $arg in
  --no-build) build=0 ;;
  -h | --help)
    sed -n '/^# End-to-end/,/^$/p' "$0" | sed 's/^# \{0,1\}//'
    exit 0
    ;;
  *) selected+=("$arg") ;;
  esac
done
((${#selected[@]})) || selected=("${SCENARIOS[@]}")

if [[ -z $($ADB get-state 2>/dev/null || true) ]]; then
  echo "no device connected" >&2
  exit 1
fi

# room for the logs of a whole scenario, see `clear_logs`
$ADB logcat -G 16M >/dev/null 2>&1 || true

if ((build)); then
  case $(sh_ getprop ro.product.cpu.abi) in
  arm64-v8a) target=aarch64 ;;
  armeabi-v7a) target=armv7 ;;
  x86) target=i686 ;;
  *) target=x86_64 ;;
  esac
  step "building for $target"
  cargo android apk build "$target"
  $ADB install -r gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
fi

logcat_pid=
if [[ -n $ARTIFACTS_DIR ]]; then
  mkdir -p "$ARTIFACTS_DIR"
  # keeps streaming when the scenarios clear the log buffer
  $ADB logcat -v threadtime >"$ARTIFACTS_DIR/logcat.txt" 2>&1 &
  logcat_pid=$!
fi

cleanup() {
  sh_ settings put system user_rotation 0 || true
  sh_ cmd uimode night no >/dev/null || true
  sh_ am force-stop "$PKG" || true
  if [[ -n $logcat_pid ]]; then kill "$logcat_pid" 2>/dev/null || true; fi
}
trap cleanup EXIT

for scenario in "${selected[@]}"; do
  current_scenario=$scenario
  step "$scenario"
  # marks the scenario in the saved logcat
  sh_ log -t tao-e2e "== $scenario" || true
  # a scenario returns early when a step fails, the failure is already counted
  "$scenario" || true
done

echo
if ((failures)); then
  printf '\033[31m%d check(s) failed in: %s\033[0m\n' "$failures" "$(printf '%s\n' "${failed_scenarios[@]}" | sort -u | xargs)"
  exit 1
fi
printf '\033[32mall %d scenarios passed\033[0m\n' "${#selected[@]}"
