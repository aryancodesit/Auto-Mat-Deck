# EP-001 Bug Log

> Classify every failure. Reproduce before fixing.
> Codes: ENV (environment), NET (networking), APP (Android app), DESK (desktop agent), TEST (procedure).

| ID | Description | Phase | Observed | Evidence | Status |
|----|-------------|-------|----------|----------|--------|
| ENV-001 | Android build fails: terminal uses Java 8 (Oracle JDK 1.8.0_491) while AGP 8.2 requires Java 11+. Android Studio is installed with JDK 21 bundled (`jbr/bin/java.exe`). Build must run via Android Studio (embedded JDK) or with `JAVA_HOME` pointing to Studio's JBR. Gradle wrapper not generated. | P0 | Session 1 | `gradle wrapper` failed: "consumer needed Java 8, AGP 8.2 requires Java 11+" | **Closed** — toolchain config, not a bug. Workaround: use Android Studio Run or set `JAVA_HOME` to Studio's JBR. |

| DESK-001 | `AdvertisementProvider::stop()` unused — compiler warning. Expected for spike. | P0 | Session 1 | `warning: method 'stop' is never used` | Won't fix |

| APP-001 | mDNS discovery fails immediately with error=0 because Android `NsdManager.discoverServices()` service type format requires `"_service._proto."` without `.local.` suffix. The constant was `_amd._tcp.local.` (wrong) instead of `_amd._tcp.` (correct). | P0 | Session 2 | `E/MdnsDiscoveryProvider: mDNS start discovery failed: error=0` in 4ms with old type. After fix: `Service found: name=AutoMatDeckDesktop` in ~67ms. Logcat: `realme-RMX3392-Android-14_2026-07-06_221443.logcat` lines 95603–95858. | **Fixed** — `SERVICE_TYPE` changed from `"_amd._tcp.local."` to `"_amd._tcp."` |
