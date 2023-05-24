# Audio Noise Generator

This program generates white/brown/pink/etc. noise, to listen to while studying.
Like [Chroma Doze](https://github.com/pmarks-net/chromadoze), we use an inverse discrete cosine transform to generate noise based on a set of frequencies.

The original frequency intensities are uniformly sampled from -1..1, which results in white noise.
By applying weights to the frequencies we can change the resulting noise to different colors, e.g. we can do a low-pass filter to get a brown noise.

## Intended Workflow

Running the app starts the GUI, which has a soundboard-like equalizer interface to set the weights for different frequency bands.
If the daemon is not running, it is also started.

Setting some weights sends them to the daemon.
The GUI can be closed afterwards.
The daemon will then generate noise samples and play them continuously on repeat.
The daemon has a system-tray icon which can be used to shut down the daemon or start the GUI again.

## TODO

- [x] Noise generation using inverse DCT
  - [x] Blending different samples to avoid popping sound
- [x] GUI for specifying frequency band weights
  - [x] Start/stop audio streams
  - [x] Saving frequency band weights
  - [ ] Straight line algorithm to prevent skipping frequency bands when moving mouse quickly
- [x] Daemon to play the noise while in background.
- [ ] Systray icon
