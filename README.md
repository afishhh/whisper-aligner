## Whisper-assisted transcription alignment

With support for Japanese text via [vibrato](https://github.com/daac-tools/vibrato) which itself is a Rust implementation of the [mecab](https://taku910.github.io/mecab/) tokenizer.

### Usage

Scenario:
- You have a correct transcription for some audio.
- You want to add timestamps to this transcription.

Solution:
1. Place your correct transcription into a text file, each line of this text file will result in a single cue in the output WebVTT.<br/>
> [!WARNING]
> Attempting to place every word into a different line to get word-level timestamps *will not work well*, if this is something you need it has be implemented properly with interpolation for tokens which could not be matched.
2. `whisper-aligner transcribe -m <PATH TO WHISPER GGML MODEL> -l <WHISPER LANGUAGE CODE> -o <OUTPUT JSON FILE> <INPUT AUDIO FILE>`
> [!NOTE]
> The input audio file will be automatically transcoded with ffmpeg.

> [!NOTE]
> Q: Why not use a json file generated directly with the `whisper-cpp` tool?<br/>
> A: Whisper tends to output many partial unicode sequences as separate tokens when transcribing complex unicode characters. This means that when transcribing Japanese whisper-cpp outputs json strings with **invalid unicode** which is not a supported use case for most JSON parsers.
3. `whisper-aligner align <WHISPER JSON FILE> <TRANSCRIPTION TEXT FILE> --output-vtt <OUTPUT VTT FILE> --vibrato-dictionary <UNCOMPRESSED VIBRATO DICTIONARY FILE>`<br/>
   The `--vibrato-dictionary` argument is optional but when omitted it will cause a simple whitespace-based tokenizer to be used instead of vibrato. This does not work well on Japanese.
4. You now have a timestamped transcription in `<OUTPUT VTT FILE>`.

### Building

Dependencies:
- `ffmpeg` is required if the `whisper` feature is enabled (default).

Also make sure you initialized the `./whisper-cpp-sys/whisper.cpp/` git submodule before building.

First build `whisper-cpp`:
```command
cd whisper-cpp-sys
./build.sh # Add "GGML_CUDA=1" here for CUDA support
# If you need support for another hardware acceleration framework you're going to have to add the corrensponding argument after ./build.sh and then modify `build.rs` to link the approprioate libraries when is was specified. (I have no way to add and test other frameworks myself)
```
Then build this crate: `cargo b --release`. You should now have a `whisper-aligner` executable in `./target/release`.

Whisper models can be downloaded using `whisper-cpp-sys/whisper.cpp/models/download-ggml-model.sh`.

### TODO

- [ ] Implement silero VAD preprocessing for audio transcription
- [ ] Figure out how to use whisper with DTW
