# Audio Noise Generator

This program generates white/brown/pink/etc. noise, to listen to while studying.
Like [Chroma Doze](https://github.com/pmarks-net/chromadoze), we use an inverse discrete cosine transform to generate noise based on a set of frequencies.

The original frequency intensities are uniformly sampled from -1..1, which results in white noise.
By applying weights to the frequencies we can change the resulting noise to different colors, e.g. we can do a low-pass filter to get a brown noise.

## Intended Workflow

Running the app starts the GUI, which has a soundboard-like equalizer interface to set the weights for different frequency bands.
If the daemon is not running, it is also started.

By keeping the left mouse button pressed while dragging the mouse, you can change the values of the equalizer.
Releasing the left mouse button confirms the weights and sends them to the daemon.
The GUI can be closed afterwards.
The daemon will then generate noise samples and play them continuously on repeat.
The daemon has a system-tray icon which can be used to shut down the daemon or start the GUI again.

## Install

The following will place `adh-gui` and `adh-daemon` in your `~/.cargo/bin`. Then place the icon in the local resource directory.

```bash
$ cargo install --locked --path .
$ mkdir -p ~/.local/share/adh-rs/resources
$ cp resources/tray-icon.png ~/.local/share/adh-rs/resources
```

Running the daemon and gui manually should work afterwards.
To make the GUI automatically spawn the daemon via systemd socket activation, place the two systemd unit files in your `~/.config/systemd/user/` directory.
Then reload the daemon and enable & start the socket.

```bash
$ cp systemd/adhdaemon.service systemd/adhdaemon.socket  ~/.config/systemd/user
$ systemctl --user daemon-reload
$ systemctl --user enable adhdaemon.socket
$ systemctl --user start adhdaemon.socket
```

Now just running `adh-gui` should start the daemon when releasing the mouse button to confirm the weights (notice the system tray icon appearing).

## TODO

- [x] Noise generation using inverse DCT
  - [x] Blending different samples to avoid popping sound
- [x] GUI for specifying frequency band weights
  - [x] Start/stop audio streams
  - [x] Saving frequency band weights
  - [x] Straight line algorithm to prevent skipping frequency bands when moving mouse quickly
- [x] Daemon to play the noise while in background.
  - [x] Socket activation so that when starting the GUI the daemon is started lazily.
  - [x] Better event handling in the daemon (one event queue instead of two threads)
- [x] Systray icon
- [ ] Maybe make it work on windows
- [ ] Proper logging
- [ ] More documentation (especially for all the sample iter stuff
