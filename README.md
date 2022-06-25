# record-audio
To record audio from cli

To record

```
cargo run --example record_play record <clipname>
```

To play
```
cargo run --example record_play play <clippath-with-extention>
cargo run --example record_play play test.wav
```

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
record-audio = "0.1.1"
```
