# WASAPI playback

PeekGame exposes one logical default render endpoint through IMMDeviceEnumerator,
IMMDevice/IMMEndpoint and a read-only IPropertyStore. IAudioClient, IAudioRenderClient
and IAudioClock are implemented in `src/api/audio/`; native handles remain inside
`src/host/pulse.rs`. All COM entry points use the Windows x64 ABI.

Playback uses libpulse and works with PulseAudio or PipeWire's PulseAudio server.
Install the libpulse development package when building (Ubuntu: `libpulse-dev`).
There is no dependency on SDL or the earlier project at build/run time.

## Supported behavior

- Shared-mode render streams, polling or event-driven.
- PCM U8/S16/S24/S32 and float32, mono/stereo. Extensible formats accept full
  valid bits and conventional mono/stereo channel masks.
- The logical mix format is 48 kHz stereo float32. Other rates/channel counts
  require AUTOCONVERTPCM or RATEADJUST; conversion runs in the host audio server.
- Owned render buffers, SILENT packets, padding, start/stop/reset, reference-counted
  COM services, and retained duplicate event handles.
- PROPVARIANT strings use CoTaskMem ownership and PropVariantClear.

RATEADJUST streams expose IAudioClockAdjustment with real PulseAudio sample-rate
updates (integral 8000–192000 Hz; invalid rates fail).

Capture, exclusive mode, session/endpoint volume APIs, surround
channel maps and endpoint-change callbacks are not implemented. Unsupported
interfaces/flags/formats return explicit HRESULTs. If no server/device is available,
no usable endpoint is advertised. The logical endpoint follows host routing; it is
not a list of physical sound cards.

## Verification

`python3 tests/test_wasapi.py` compiles a Windows COM fixture and plays float32,
PCM16 and RATEADJUST streams into a private null sink. It captures samples from that sink's monitor and
checks real output, COM identity, properties, event delivery, buffer ordering,
clock position and reset. Requires `pactl`, `parec`, MinGW and a running audio server;
it neither changes the default output nor plays the fixture on speakers.

A private-Xvfb Unity run also produced menu music in a monitor capture. Logs are
local under `logs/unity-audio-*`. This verifies the sample path, not physical speaker
routing, full-session stability, capture or device hotplug. Unity may retry at the
48 kHz mix rate after rejecting its initial non-mix-rate request.

Rebuild and stage with `cargo build --release --offline` and
`python3 tools/stage_dist.py`. Relinked games contain their runtime statically:
regenerate their ELF to pick up audio changes.
