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


## Example

```rust
use record_audio::audio_clip::AudioClip as ac;

fn main() {
    match ac::record(None) {
        Ok(clip) => {
            println!("Successfully recorded!");
            match clip.export(format!("{}.wav", clip.name).as_str()) {
                Ok(_) => {
                    println!("Successfully saved as {}", clip.name);
                    
                    //to play immediately, after ctrl-c
                    clip.play().unwrap();
                }
                Err(err) => println!("Error {}", err),
            }
        }
        Err(err) => println!("Error {}", err),
    }


    // match ac::import(String::from("clip.wav")) {
    //     Ok(clip) => {
    //         println!("Successfully imported!");
    //         match clip.play() {
    //             Ok(_) => {
    //                 println!("Successfully played");                    
    //             }
    //             Err(err) => println!("Error {}", err),
    //         }
    //     }
    //     Err(err) => println!("Error {}", err),
    // }
}
```
