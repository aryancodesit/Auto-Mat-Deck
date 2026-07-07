# EP-003 Hardware Validation

**Date**: 2026-07-07
**Desktop**: AryanGupta (192.168.29.59)
**Agent**: `amd-desktop.exe` (release, v0.1.0)
**Build**: `cargo build --release` (3.3 MB)
**Test client**: Python `websockets` library via localhost

## Test 1: Launch Chrome

```
→ {"type":"action","request_id":"r1","action":"launch","payload":{"app":"chrome"}}
← {"type":"action_result","request_id":"r1","success":true,"data":{"pid":23196}}
```

Chrome browser window opened on desktop. ✅

## Test 2: Open URL

```
→ {"type":"action","request_id":"r2","action":"open_url","payload":{"url":"https://github.com"}}
← {"type":"action_result","request_id":"r2","success":true,"data":{"opened":"https://github.com"}}
```

Default browser opened to GitHub. ✅

## Test 3: Open File

```
→ {"type":"action","request_id":"r3","action":"open_file","payload":{"path":"C:\\Windows\\System32\\calc.exe"}}
← {"type":"action_result","request_id":"r3","success":true,"data":{"opened":"C:\\Windows\\System32\\calc.exe"}}
```

Calculator application launched. ✅

## Test 4: Lock WorkStation

```
→ {"type":"action","request_id":"lock1","action":"lock","payload":{}}
← {"type":"action_result","request_id":"lock1","success":true,"data":{"locked":true}}
```

WorkStation locked immediately after response. ✅

## Test 5: Desktop Notification

```
→ {"type":"action","request_id":"r4","action":"notify","payload":{"title":"AutoMatDeck","body":"Test notification from EP-003"}}
← {"type":"action_result","request_id":"r4","success":true,"data":{"notified":true}}
```

Windows 10 toast notification appeared in action center. ✅

## Summary

| Action | Result |
|--------|--------|
| Launch Chrome | ✅ PASS |
| Open Calculator | ✅ PASS |
| Open URL | ✅ PASS |
| Lock WorkStation | ✅ PASS |
| Desktop Notification | ✅ PASS |

**All 5 criteria met. EP-003 certified.**
